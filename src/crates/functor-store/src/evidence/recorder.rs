//! The durable evidence recorder (ADR-0013, ADR-0015), generic over where entities and bytes live.
//!
//! Evidence is stored as immutable artifacts and relationships in an ordinary store, so it stays
//! inspectable through the normal artifact and query APIs. Every method is a sequence of short,
//! individually atomic writes in the order of ADR-0013; the write lock is never held while a child
//! process runs. A run is complete if and only if its `run_result` artifact exists: that single
//! write is the commit point, and everything before it can be left behind harmlessly.

use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use functor_core::{
    ArtifactRepository, CapturedBytes, Clock, EvidenceRecorder, FunctorError, IdGenerator,
    IntentRecord, RelationshipRepository, ReportRecorder, ReportSubject, RepositoryReader, Result,
    RunFinish, RunLease, RunStreams,
};
use functor_model::{
    Artifact, ArtifactId, ArtifactRef, ArtifactType, Capture, CaptureRole, Content, ContentFormat,
    ContentPath, EntityKind, EvidenceKind, Metadata, NewArtifact, NewRelationship,
    RelationshipType, RepositorySnapshot, Revision, RunIntent, RunOutcome, RunRecord, RunResult,
    RunResultSpec, RunStatus, Slug, Timestamp, Title, Truncation, ValidationReport, bounds,
};

use crate::evidence::blobs::{EvidenceBlobs, Guard};
use crate::evidence::codec::{
    BlobRef, MetaBuilder, OutputRef, StoredResult, capture_metadata, decode_capture, decode_intent,
    decode_report, decode_result, encode_intent, encode_report, encode_result, meta_id, meta_ids,
    meta_int, meta_str, report_metadata, result_metadata, run_metadata, sha256,
};

/// What the recorder needs from the store that holds the entities.
pub trait EvidenceBacking: ArtifactRepository + RelationshipRepository + RepositoryReader {}

impl<T> EvidenceBacking for T where T: ArtifactRepository + RelationshipRepository + RepositoryReader
{}

/// Evidence for runs and validation reports, stored in `S` with its bytes and locks in `B`.
pub struct EvidenceStore<S, B> {
    store: S,
    blobs: B,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl<S, B> fmt::Debug for EvidenceStore<S, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvidenceStore").finish_non_exhaustive()
    }
}

struct StoreLease {
    run: ArtifactId,
    _guard: Guard,
}

impl fmt::Debug for StoreLease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunLease")
            .field("run", &self.run)
            .finish_non_exhaustive()
    }
}

impl RunLease for StoreLease {
    fn run(&self) -> &ArtifactId {
        &self.run
    }
}

type Edge = (ArtifactId, RelationshipType, ArtifactId);

fn broken(artifact: &Artifact, detail: impl fmt::Display) -> FunctorError {
    FunctorError::Inconsistent {
        detail: format!("evidence `{}`: {detail}", artifact.slug()),
    }
}

fn not_found(id: &ArtifactId) -> FunctorError {
    FunctorError::not_found(EntityKind::Artifact, id.to_string())
}

/// A slug-safe version of a binding name: hyphens only, no leading, trailing or double hyphen.
fn slug_part(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let c = if c == '_' { '-' } else { c };
        if c == '-' && (out.is_empty() || out.ends_with('-')) {
            continue;
        }
        out.push(c);
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() { "x".to_owned() } else { out }
}

fn inline_text(artifact: &Artifact) -> Result<&str> {
    match artifact.content() {
        Content::Inline { text, .. } => Ok(text),
        _ => Err(broken(artifact, "its content is not inline text")),
    }
}

fn content_path(artifact: &Artifact) -> Result<&ContentPath> {
    match artifact.content() {
        Content::Path { path, .. } => Ok(path),
        _ => Err(broken(artifact, "it has no content file")),
    }
}

fn as_text(artifact: &Artifact, bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes).map_err(|_| broken(artifact, "its content is not UTF-8"))
}

fn kind_of(snapshot: &RepositorySnapshot, id: &ArtifactId, kind: EvidenceKind) -> Option<Artifact> {
    snapshot
        .artifact(id)
        .filter(|artifact| EvidenceKind::of(artifact) == Some(kind))
        .cloned()
}

fn of_kind(snapshot: &RepositorySnapshot, kind: EvidenceKind) -> Vec<&Artifact> {
    let mut found: Vec<&Artifact> = snapshot
        .artifacts()
        .iter()
        .filter(|artifact| EvidenceKind::of(artifact) == Some(kind))
        .collect();
    found.sort_by_key(|artifact| *artifact.id());
    found
}

fn result_artifact<'a>(snapshot: &'a RepositorySnapshot, run: &ArtifactId) -> Option<&'a Artifact> {
    of_kind(snapshot, EvidenceKind::RunResult)
        .into_iter()
        .find(|artifact| meta_id(artifact, "run") == Some(*run))
}

impl<S: EvidenceBacking, B: EvidenceBlobs> EvidenceStore<S, B> {
    /// Evidence over `store`, with bytes and locks in `blobs`, taking time and ids from the
    /// injected sources.
    #[must_use]
    pub fn new(store: S, blobs: B, clock: Arc<dyn Clock>, ids: Arc<dyn IdGenerator>) -> Self {
        Self {
            store,
            blobs,
            clock,
            ids,
        }
    }

    /// The entity store.
    #[must_use]
    pub fn store(&self) -> &S {
        &self.store
    }

    /// The byte and lock storage.
    #[must_use]
    pub fn blobs(&self) -> &B {
        &self.blobs
    }

    async fn snapshot(&self) -> Result<RepositorySnapshot> {
        RepositoryReader::snapshot(&self.store).await
    }

    async fn resolve(&self, reference: &ArtifactRef) -> Result<Option<Artifact>> {
        ArtifactRepository::resolve(&self.store, reference).await
    }

    fn suffix(&self) -> String {
        let id = self.ids.artifact_id().to_string();
        id[id.len().saturating_sub(10)..].to_ascii_lowercase()
    }

    /// Creates an artifact with a generated slug `<stem><suffix>`, retrying on a slug collision.
    async fn create_artifact(
        &self,
        stem: &str,
        artifact_type: ArtifactType,
        title: &str,
        metadata: Metadata,
        content: Content,
    ) -> Result<Artifact> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let new = NewArtifact::new(
                Slug::parse(&format!("{stem}{}", self.suffix()))?,
                artifact_type.clone(),
                Title::new(title)?,
            )
            .with_metadata(metadata.clone())
            .with_content(content.clone());
            match ArtifactRepository::create(&self.store, new).await {
                Err(FunctorError::Conflict { .. }) if attempt < 5 => {}
                other => return other,
            }
        }
    }

    /// Creates the edges that do not exist yet; a duplicate is not an error.
    async fn ensure_edges(&self, edges: &[Edge]) -> Result<()> {
        for (source, relationship_type, target) in edges {
            let new = NewRelationship::new(relationship_type.clone(), *source, *target);
            match RelationshipRepository::create(&self.store, new).await {
                Ok(_)
                | Err(FunctorError::Conflict {
                    entity: EntityKind::Relationship,
                    ..
                }) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    async fn source_bytes(&self, artifact: &Artifact) -> Result<Vec<u8>> {
        match artifact.content() {
            Content::Inline { text, .. } => Ok(text.clone().into_bytes()),
            Content::Path { path, .. } => {
                self.blobs
                    .get(path)
                    .await?
                    .ok_or_else(|| FunctorError::Inconsistent {
                        detail: format!(
                            "content file `{path}` of `{}` is missing",
                            artifact.slug()
                        ),
                    })
            }
            _ => Err(FunctorError::invalid_input(
                "content",
                format!("`{}` has no content to capture", artifact.slug()),
            )),
        }
    }

    fn run_artifact(snapshot: &RepositorySnapshot, run: &ArtifactId) -> Result<Artifact> {
        kind_of(snapshot, run, EvidenceKind::Run).ok_or_else(|| not_found(run))
    }

    fn intent_of(artifact: &Artifact) -> Result<RunIntent> {
        decode_intent(inline_text(artifact)?).map_err(|detail| broken(artifact, detail))
    }

    /// The result document of a completed run, with the id of its `run_result` artifact.
    async fn stored_result(
        &self,
        snapshot: &RepositorySnapshot,
        run: &Artifact,
        intent: &RunIntent,
    ) -> Result<Option<(ArtifactId, StoredResult)>> {
        let Some(artifact) = result_artifact(snapshot, run.id()) else {
            return Ok(None);
        };
        let path = content_path(artifact)?;
        let bytes = self
            .blobs
            .get(path)
            .await?
            .ok_or_else(|| broken(artifact, format!("its result file `{path}` is missing")))?;
        let text = as_text(artifact, bytes)?;
        let stored =
            decode_result(&text, intent.started).map_err(|detail| broken(artifact, detail))?;
        if stored.result.run() != run.id() {
            return Err(broken(artifact, "it describes another run"));
        }
        Ok(Some((*artifact.id(), stored)))
    }

    async fn record_of(&self, snapshot: &RepositorySnapshot, run: &Artifact) -> Result<RunRecord> {
        let intent = Self::intent_of(run)?;
        let record = RunRecord::new(*run.id(), intent.clone());
        match self.stored_result(snapshot, run, &intent).await? {
            Some((result_id, stored)) => record
                .complete(result_id, stored.result)
                .map_err(|error| broken(run, error)),
            None => Ok(record),
        }
    }

    /// Verifies stored bytes against their digest.
    async fn verified(
        &self,
        owner: &Artifact,
        path: &ContentPath,
        digest: &functor_model::ContentDigest,
        size: Option<u64>,
    ) -> Result<Vec<u8>> {
        let bytes = self
            .blobs
            .get(path)
            .await?
            .ok_or_else(|| broken(owner, format!("the file `{path}` is missing")))?;
        if &sha256(&bytes) != digest || size.is_some_and(|size| size != bytes.len() as u64) {
            return Err(broken(
                owner,
                format!("the file `{path}` does not match its digest"),
            ));
        }
        Ok(bytes)
    }

    /// Publishes outputs and then the `run_result`, the commit point (ADR-0013 steps 6 to 8).
    async fn complete(
        &self,
        run: &ArtifactId,
        intent: &RunIntent,
        finish: RunFinish,
    ) -> Result<RunRecord> {
        let finished = self.clock.now().max(intent.started);
        let spec = |outputs: Vec<ArtifactId>| RunResultSpec {
            outcome: finish.outcome,
            finished,
            exit_code: finish.exit_code,
            truncated: finish.truncated,
            outputs,
            diagnostics: finish.diagnostics.clone(),
        };

        // Nothing is written until the finish is known to be consistent.
        RunResult::new(*run, intent.started, spec(vec![*run; finish.outputs.len()]))?;
        let mut names = HashSet::new();
        if !finish
            .outputs
            .iter()
            .all(|o| names.insert(o.spec.name.as_str()))
        {
            return Err(FunctorError::invalid_input(
                "outputs",
                "an output name is published twice",
            ));
        }

        let stream = |bytes: &[u8]| BlobRef {
            digest: sha256(bytes),
            size: bytes.len() as u64,
        };
        let (stdout, stderr) = (stream(&finish.stdout), stream(&finish.stderr));
        self.blobs
            .put("streams", &stdout.digest, finish.stdout.clone())
            .await?;
        self.blobs
            .put("streams", &stderr.digest, finish.stderr.clone())
            .await?;

        let mut published = Vec::new();
        for output in &finish.outputs {
            let digest = sha256(&output.bytes);
            let path = self
                .blobs
                .put("outputs", &digest, output.bytes.clone())
                .await?;
            let mut metadata = MetaBuilder::plain();
            metadata
                .put("run", run.to_string())?
                .put("output", output.spec.name.as_str())?
                .put("sha256", digest.as_str())?;
            let artifact = self
                .create_artifact(
                    &format!("out-{}-", slug_part(&output.spec.name)),
                    output.spec.artifact_type.clone(),
                    &format!("Output {} of {run}", output.spec.name),
                    metadata.finish(),
                    Content::Path {
                        format: ContentFormat::new(&output.spec.format)?,
                        path,
                    },
                )
                .await?;
            published.push(OutputRef {
                artifact: *artifact.id(),
                name: output.spec.name.clone(),
                digest,
                size: output.bytes.len() as u64,
            });
        }

        let result = RunResult::new(
            *run,
            intent.started,
            spec(published.iter().map(|o| o.artifact).collect()),
        )?;
        let document = encode_result(&result, &stdout, &stderr, &published);
        let digest = sha256(document.as_bytes());
        let path = self
            .blobs
            .put("results", &digest, document.into_bytes())
            .await?;
        let committed = self
            .create_artifact(
                EvidenceKind::RunResult.slug_prefix(),
                EvidenceKind::RunResult.to_artifact_type(),
                &format!("Result of {run}"),
                result_metadata(&result)?,
                Content::Path {
                    format: ContentFormat::new("json")?,
                    path,
                },
            )
            .await?;

        // Past the commit point the run is complete; a failure here leaves edges that `recover`
        // derives from the records, and is still reported (never a silent success).
        let mut edges: Vec<Edge> = vec![(*committed.id(), RelationshipType::DerivedFrom, *run)];
        edges.extend(
            published
                .iter()
                .map(|o| (o.artifact, RelationshipType::DerivedFrom, *committed.id())),
        );
        self.ensure_edges(&edges).await?;

        RunRecord::new(*run, intent.clone())
            .complete(*committed.id(), result)
            .map_err(|error| FunctorError::Internal(error.to_string()))
    }

    /// The edges the records imply, for repair.
    fn implied_edges(snapshot: &RepositorySnapshot) -> Vec<Edge> {
        let exists = |id: &ArtifactId| snapshot.artifact(id).is_some();
        let mut edges = Vec::new();
        for capture in of_kind(snapshot, EvidenceKind::Capture) {
            if let Some(source) = meta_id(capture, "source").filter(exists) {
                edges.push((*capture.id(), RelationshipType::DerivedFrom, source));
            }
        }
        for run in of_kind(snapshot, EvidenceKind::Run) {
            let Ok(intent) = Self::intent_of(run) else {
                continue;
            };
            for capture in std::iter::once(&intent.workflow).chain(intent.inputs.values()) {
                if exists(capture) {
                    edges.push((*run.id(), RelationshipType::DependsOn, *capture));
                }
            }
        }
        for result in of_kind(snapshot, EvidenceKind::RunResult) {
            if let Some(run) = meta_id(result, "run").filter(exists) {
                edges.push((*result.id(), RelationshipType::DerivedFrom, run));
            }
            for output in meta_ids(result, "outputs").into_iter().filter(exists) {
                edges.push((output, RelationshipType::DerivedFrom, *result.id()));
            }
        }
        for report in of_kind(snapshot, EvidenceKind::ValidationReport) {
            if let Some(subject) = meta_id(report, "subject").filter(exists) {
                edges.push((*report.id(), RelationshipType::Validates, subject));
            }
        }
        edges
    }

    async fn repair_relationships(&self) -> Result<()> {
        let snapshot = self.snapshot().await?;
        let present: HashSet<Edge> = snapshot
            .relationships()
            .iter()
            .map(|r| (*r.source(), r.relationship_type().clone(), *r.target()))
            .collect();
        let missing: Vec<Edge> = Self::implied_edges(&snapshot)
            .into_iter()
            .filter(|edge| !present.contains(edge))
            .collect();
        self.ensure_edges(&missing).await
    }
}

#[async_trait]
impl<S: EvidenceBacking, B: EvidenceBlobs> EvidenceRecorder for EvidenceStore<S, B> {
    async fn capture(&self, source: &ArtifactRef, role: CaptureRole) -> Result<CapturedBytes> {
        let limit = match role {
            CaptureRole::Workflow => bounds::MAX_WORKFLOW_BYTES,
            _ => bounds::MAX_ITEM_BYTES,
        };
        // Read the record and the bytes twice; if either changed in between, try once more, then
        // give up rather than record a state that never existed.
        let mut stable = None;
        for _ in 0..2 {
            let first = self
                .resolve(source)
                .await?
                .ok_or_else(|| FunctorError::not_found(EntityKind::Artifact, source.to_string()))?;
            let bytes = self.source_bytes(&first).await?;
            let second = self
                .resolve(&ArtifactRef::Id(*first.id()))
                .await?
                .ok_or_else(|| not_found(first.id()))?;
            if first.revision() == second.revision() && bytes == self.source_bytes(&second).await? {
                stable = Some((first, bytes));
                break;
            }
        }
        let Some((live, bytes)) = stable else {
            return Err(FunctorError::conflict(
                EntityKind::Artifact,
                format!("`{source}` changed while it was being captured"),
            ));
        };
        if bytes.len() as u64 > limit {
            return Err(FunctorError::invalid_input(
                "content",
                format!("`{}` is larger than {limit} bytes", live.slug()),
            ));
        }

        let digest = sha256(&bytes);
        let mut capture = Capture {
            artifact: *live.id(), // replaced by the capture's own id once it exists
            source: *live.id(),
            source_revision: live.revision(),
            role,
            digest: digest.clone(),
            size: bytes.len() as u64,
        };
        let path = self.blobs.put("captures", &digest, bytes.clone()).await?;
        let stem = format!(
            "{}{}-",
            EvidenceKind::Capture.slug_prefix(),
            &digest.hex()[..12]
        );
        let created = self
            .create_artifact(
                &stem,
                EvidenceKind::Capture.to_artifact_type(),
                &format!("Capture of {}", live.slug()),
                capture_metadata(&capture)?,
                Content::Path {
                    format: ContentFormat::new("binary")?,
                    path,
                },
            )
            .await?;
        capture.artifact = *created.id();
        self.ensure_edges(&[(*created.id(), RelationshipType::DerivedFrom, *live.id())])
            .await?;
        Ok(CapturedBytes { capture, bytes })
    }

    async fn record_intent(&self, intent: RunIntent) -> Result<IntentRecord> {
        let _guard = self.blobs.lock_request(&intent.request_id).await?;
        let snapshot = self.snapshot().await?;

        let existing = of_kind(&snapshot, EvidenceKind::Run)
            .into_iter()
            .find(|run| meta_str(run, "request_id") == Some(intent.request_id.as_str()));
        if let Some(run) = existing {
            // The caller must not start another child; make sure the run's edges exist.
            let record = self.record_of(&snapshot, run).await?;
            self.repair_relationships().await?;
            return Ok(IntentRecord {
                record,
                created: false,
            });
        }

        let checks = std::iter::once((&intent.workflow, CaptureRole::Workflow))
            .chain(intent.inputs.values().map(|id| (id, CaptureRole::Input)));
        for (id, expected) in checks {
            let capture =
                kind_of(&snapshot, id, EvidenceKind::Capture).ok_or_else(|| not_found(id))?;
            let recorded = decode_capture(&capture).map_err(|detail| broken(&capture, detail))?;
            if recorded.role != expected {
                return Err(FunctorError::invalid_input(
                    "capture",
                    format!(
                        "`{}` is a {} capture, expected {}",
                        capture.slug(),
                        recorded.role.as_str(),
                        expected.as_str()
                    ),
                ));
            }
        }

        let run = self
            .create_artifact(
                EvidenceKind::Run.slug_prefix(),
                EvidenceKind::Run.to_artifact_type(),
                &format!("Run of {}", intent.request_id),
                run_metadata(&intent)?,
                Content::Inline {
                    format: ContentFormat::new("json")?,
                    text: encode_intent(&intent),
                },
            )
            .await?;
        let edges: Vec<Edge> = std::iter::once(&intent.workflow)
            .chain(intent.inputs.values())
            .map(|capture| (*run.id(), RelationshipType::DependsOn, *capture))
            .collect();
        self.ensure_edges(&edges).await?;
        Ok(IntentRecord {
            record: RunRecord::new(*run.id(), intent),
            created: true,
        })
    }

    async fn acquire_lease(&self, run: &ArtifactId) -> Result<Box<dyn RunLease>> {
        let incomplete = |snapshot: &RepositorySnapshot| -> Result<()> {
            Self::run_artifact(snapshot, run)?;
            if result_artifact(snapshot, run).is_some() {
                return Err(FunctorError::conflict(
                    EntityKind::Artifact,
                    "the run is already complete",
                ));
            }
            Ok(())
        };
        incomplete(&self.snapshot().await?)?;
        let guard = self.blobs.try_lease(run).await?;
        // The owner we replaced may have finished just before we took over.
        incomplete(&self.snapshot().await?)?;
        Ok(Box::new(StoreLease {
            run: *run,
            _guard: guard,
        }))
    }

    async fn record_result(&self, run: &ArtifactId, finish: RunFinish) -> Result<RunRecord> {
        let snapshot = self.snapshot().await?;
        let artifact = Self::run_artifact(&snapshot, run)?;
        if result_artifact(&snapshot, run).is_some() {
            return Err(FunctorError::conflict(
                EntityKind::Artifact,
                "the run already has a result",
            ));
        }
        let intent = Self::intent_of(&artifact)?;
        self.complete(run, &intent, finish).await
    }

    async fn run(&self, run: &ArtifactRef) -> Result<Option<RunRecord>> {
        let snapshot = self.snapshot().await?;
        match snapshot
            .resolve(run)
            .filter(|artifact| EvidenceKind::of(artifact) == Some(EvidenceKind::Run))
        {
            Some(artifact) => Ok(Some(self.record_of(&snapshot, artifact).await?)),
            None => Ok(None),
        }
    }

    async fn runs(&self) -> Result<Vec<RunRecord>> {
        let snapshot = self.snapshot().await?;
        let mut records = Vec::new();
        for run in of_kind(&snapshot, EvidenceKind::Run) {
            records.push(self.record_of(&snapshot, run).await?);
        }
        Ok(records)
    }

    async fn status(&self, run: &ArtifactId) -> Result<RunStatus> {
        let snapshot = self.snapshot().await?;
        let artifact = Self::run_artifact(&snapshot, run)?;
        let record = self.record_of(&snapshot, &artifact).await?;
        Ok(record.status(self.blobs.is_leased(run).await?))
    }

    async fn streams(&self, run: &ArtifactId) -> Result<Option<RunStreams>> {
        let snapshot = self.snapshot().await?;
        let artifact = Self::run_artifact(&snapshot, run)?;
        let intent = Self::intent_of(&artifact)?;
        let Some((result_id, stored)) = self.stored_result(&snapshot, &artifact, &intent).await?
        else {
            return Ok(None);
        };
        let owner = snapshot
            .artifact(&result_id)
            .ok_or_else(|| not_found(&result_id))?;
        let mut read = Vec::new();
        for blob in [&stored.stdout, &stored.stderr] {
            let path =
                ContentPath::new(&format!("content/evidence/streams/{}", blob.digest.hex()))?;
            read.push(
                self.verified(owner, &path, &blob.digest, Some(blob.size))
                    .await?,
            );
        }
        let stderr = read.pop().unwrap_or_default();
        let stdout = read.pop().unwrap_or_default();
        Ok(Some(RunStreams { stdout, stderr }))
    }

    async fn read_capture(&self, capture: &ArtifactId) -> Result<Vec<u8>> {
        let snapshot = self.snapshot().await?;
        let artifact =
            kind_of(&snapshot, capture, EvidenceKind::Capture).ok_or_else(|| not_found(capture))?;
        let recorded = decode_capture(&artifact).map_err(|detail| broken(&artifact, detail))?;
        let path = content_path(&artifact)?;
        self.verified(&artifact, path, &recorded.digest, Some(recorded.size))
            .await
    }

    async fn recover(&self) -> Result<Vec<RunRecord>> {
        let snapshot = self.snapshot().await?;
        let mut orphans = Vec::new();
        for run in of_kind(&snapshot, EvidenceKind::Run) {
            if result_artifact(&snapshot, run.id()).is_none() {
                orphans.push((*run.id(), Self::intent_of(run)?));
            }
        }

        let mut recovered = Vec::new();
        for (run, intent) in orphans {
            let guard = match self.blobs.try_lease(&run).await {
                Ok(guard) => guard,
                // Its owner is alive: leave it alone.
                Err(FunctorError::Locked { .. }) => continue,
                Err(error) => return Err(error),
            };
            if result_artifact(&self.snapshot().await?, &run).is_some() {
                continue;
            }
            let finish = RunFinish {
                outcome: RunOutcome::Interrupted,
                exit_code: None,
                truncated: Truncation::default(),
                diagnostics: vec!["the run was interrupted before it finished".to_owned()],
                stdout: Vec::new(),
                stderr: Vec::new(),
                outputs: Vec::new(),
            };
            recovered.push(self.complete(&run, &intent, finish).await?);
            drop(guard);
        }
        self.repair_relationships().await?;
        Ok(recovered)
    }
}

#[async_trait]
impl<S: EvidenceBacking, B: EvidenceBlobs> ReportRecorder for EvidenceStore<S, B> {
    async fn save_report(
        &self,
        report: &ValidationReport,
        subject: ReportSubject,
        strict: bool,
        rules: &str,
    ) -> Result<ArtifactId> {
        if rules.is_empty() || rules.chars().count() > 100 || rules.chars().any(char::is_control) {
            return Err(FunctorError::invalid_input(
                "rules",
                "must be 1 to 100 characters without control characters",
            ));
        }
        let (label, target, revision, digest) = match &subject {
            ReportSubject::Repository => ("repository".to_owned(), None, None, None),
            ReportSubject::Artifact {
                id,
                revision,
                digest,
            } => (id.to_string(), Some(*id), Some(*revision), digest.clone()),
            _ => {
                return Err(FunctorError::unsupported("this kind of report subject"));
            }
        };
        let snapshot = self.snapshot().await?;
        if let Some(id) = &target
            && snapshot.artifact(id).is_none()
        {
            return Err(not_found(id));
        }

        let document = encode_report(report, strict).map_err(|detail| {
            FunctorError::Internal(format!("a report could not be encoded: {detail}"))
        })?;
        let content_digest = sha256(document.as_bytes());
        let path = self
            .blobs
            .put("reports", &content_digest, document.into_bytes())
            .await?;
        let metadata = report_metadata(report, strict, &label, revision, digest.as_ref(), rules)?;
        let saved = self
            .create_artifact(
                EvidenceKind::ValidationReport.slug_prefix(),
                EvidenceKind::ValidationReport.to_artifact_type(),
                &format!("Validation report: {}", report.outcome()),
                metadata,
                Content::Path {
                    format: ContentFormat::new("json")?,
                    path,
                },
            )
            .await?;
        if let Some(subject) = target {
            self.ensure_edges(&[(*saved.id(), RelationshipType::Validates, subject)])
                .await?;
        }
        Ok(*saved.id())
    }

    async fn load_report(
        &self,
        id: &ArtifactId,
    ) -> Result<Option<(ValidationReport, ReportSubject)>> {
        let snapshot = self.snapshot().await?;
        let Some(artifact) = kind_of(&snapshot, id, EvidenceKind::ValidationReport) else {
            return Ok(None);
        };
        let bytes = self
            .blobs
            .get(content_path(&artifact)?)
            .await?
            .ok_or_else(|| broken(&artifact, "its report file is missing"))?;
        let completed: Timestamp = meta_str(&artifact, "completed")
            .and_then(|text| text.parse().ok())
            .ok_or_else(|| broken(&artifact, "missing `completed`"))?;
        let (report, _) = decode_report(&as_text(&artifact, bytes)?, completed)
            .map_err(|detail| broken(&artifact, detail))?;

        let subject = match meta_str(&artifact, "subject") {
            Some("repository") => ReportSubject::Repository,
            Some(text) => {
                let subject_id: ArtifactId = text
                    .parse()
                    .map_err(|_| broken(&artifact, "malformed `subject`"))?;
                let revision = meta_int(&artifact, "subject_revision")
                    .and_then(|n| u64::try_from(n).ok())
                    .and_then(|n| Revision::new(n).ok())
                    .ok_or_else(|| broken(&artifact, "missing `subject_revision`"))?;
                let digest = match meta_str(&artifact, "subject_sha256") {
                    Some(text) => Some(
                        text.parse()
                            .map_err(|_| broken(&artifact, "malformed `subject_sha256`"))?,
                    ),
                    None => None,
                };
                ReportSubject::Artifact {
                    id: subject_id,
                    revision,
                    digest,
                }
            }
            None => return Err(broken(&artifact, "missing `subject`")),
        };
        Ok(Some((report, subject)))
    }
}
