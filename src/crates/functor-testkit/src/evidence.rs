//! A fake evidence recorder and the evidence conformance suite (ADR-0013, ADR-0015).
//!
//! [`run_evidence_conformance`] is the behavioral contract every [`EvidenceRecorder`] and
//! [`ReportRecorder`] must pass: exact and stable captures, idempotent intent, exclusive leases,
//! the result as the single commit point, rejection of inconsistent records without writes,
//! recovery that completes only orphans, and damaged-capture detection. The file-backed recorder
//! of P1.2 runs the same scenarios. [`MemoryEvidence`] is the in-memory fake used to test the
//! suite and the orchestration; it keeps its own maps rather than repository artifacts.

// The scenarios are assertions: they panic on failure by design.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::missing_panics_doc)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use functor_core::{
    CapturedBytes, Clock, ErrorKind, EvidenceRecorder, FunctorError, IdGenerator, IntentRecord,
    ReportRecorder, ReportSubject, Result, RunFinish, RunLease, RunStreams,
};
use functor_model::{
    ArtifactId, ArtifactRef, Capture, CaptureRole, ContentDigest, EntityKind, ExecutionLimits,
    Finding, LOCAL_PROCESS_PROVIDER, ProgramProvenance, RequestId, Revision, RuleCode, RunIntent,
    RunOutcome, RunRecord, RunResult, RunResultSpec, RunStatus, Severity, Subject, Timestamp,
    Truncation, ValidationReport, bounds,
};

use crate::conformance::{Env, Fresh};

/// A deterministic stand-in for SHA-256: four FNV-1a lanes, formatted as a digest. Not
/// cryptographic; it exists so the testkit needs no hashing dependency.
///
/// # Panics
/// Never.
#[must_use]
pub fn fake_digest(bytes: &[u8]) -> ContentDigest {
    let mut out = [0_u8; 32];
    for (lane, chunk) in out.chunks_mut(8).enumerate() {
        let lane = u64::try_from(lane).unwrap_or(0);
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325 ^ lane.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        chunk.copy_from_slice(&hash.to_be_bytes());
    }
    ContentDigest::from_bytes(out)
}

/// Deliberate defects the suite must detect.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Defect {
    CaptureReadsLiveState,
    IntentNotIdempotent,
    IntentSkipsCaptureCheck,
    LeaseNotExclusive,
    SecondResultOverwrites,
    AcceptsInconsistentFinish,
    RecoverTakesRunningRuns,
    DigestNotVerified,
    ReportMutatedOnSave,
}

#[derive(Debug, Clone)]
struct Source {
    slug: String,
    revision: Revision,
    content: String,
}

#[derive(Debug, Clone)]
struct RunEntry {
    intent: RunIntent,
    result: Option<(ArtifactId, RunResult)>,
    streams: Option<RunStreams>,
}

#[derive(Debug, Default)]
struct State {
    sources: BTreeMap<ArtifactId, Source>,
    captures: BTreeMap<ArtifactId, (Capture, Vec<u8>)>,
    runs: BTreeMap<ArtifactId, RunEntry>,
    requests: BTreeMap<RequestId, ArtifactId>,
    reports: BTreeMap<ArtifactId, (ValidationReport, ReportSubject)>,
}

type Leases = Arc<Mutex<BTreeSet<ArtifactId>>>;

#[derive(Debug)]
struct Lease {
    run: ArtifactId,
    leases: Leases,
}

impl RunLease for Lease {
    fn run(&self) -> &ArtifactId {
        &self.run
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.leases
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.run);
    }
}

/// An in-memory evidence recorder driven by the injected clock and ids.
pub struct MemoryEvidence {
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    state: Mutex<State>,
    leases: Leases,
    defect: Option<Defect>,
}

impl std::fmt::Debug for MemoryEvidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryEvidence").finish_non_exhaustive()
    }
}

impl MemoryEvidence {
    /// A correct recorder using the environment's clock and ids.
    #[must_use]
    pub fn new(env: &Env) -> Self {
        Self::build(env, None)
    }

    #[cfg(test)]
    pub(crate) fn with_defect(env: &Env, defect: Defect) -> Self {
        Self::build(env, Some(defect))
    }

    fn build(env: &Env, defect: Option<Defect>) -> Self {
        Self {
            clock: env.clock(),
            ids: env.ids(),
            state: Mutex::default(),
            leases: Leases::default(),
            defect,
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn held(&self) -> MutexGuard<'_, BTreeSet<ArtifactId>> {
        self.leases.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn has(&self, defect: Defect) -> bool {
        self.defect == Some(defect)
    }

    fn not_found(id: &ArtifactId) -> FunctorError {
        FunctorError::not_found(EntityKind::Artifact, id.to_string())
    }

    fn record(id: &ArtifactId, entry: &RunEntry) -> RunRecord {
        let record = RunRecord::new(*id, entry.intent.clone());
        match &entry.result {
            Some((result_id, result)) => record
                .complete(*result_id, result.clone())
                .expect("stored results are consistent"),
            None => record,
        }
    }

    fn complete_run(
        &self,
        state: &mut State,
        run: &ArtifactId,
        spec: RunResultSpec,
        streams: RunStreams,
    ) -> Result<RunRecord> {
        let started = state
            .runs
            .get(run)
            .ok_or_else(|| Self::not_found(run))?
            .intent
            .started;
        let result = RunResult::new(*run, started, spec)?;
        let result_id = self.ids.artifact_id();
        let entry = state
            .runs
            .get_mut(run)
            .ok_or_else(|| Self::not_found(run))?;
        entry.result = Some((result_id, result));
        entry.streams = Some(streams);
        Ok(Self::record(run, entry))
    }

    /// Test harness: creates a source artifact with inline text content.
    pub fn put_source(&self, slug: &str, content: &str) -> ArtifactId {
        let id = self.ids.artifact_id();
        self.state().sources.insert(
            id,
            Source {
                slug: slug.to_owned(),
                revision: Revision::FIRST,
                content: content.to_owned(),
            },
        );
        id
    }

    /// Test harness: replaces a source's content and bumps its revision.
    ///
    /// # Errors
    /// `NotFound` for an unknown source.
    pub fn edit_source(&self, id: &ArtifactId, content: &str) -> Result<()> {
        let mut state = self.state();
        let source = state
            .sources
            .get_mut(id)
            .ok_or_else(|| Self::not_found(id))?;
        content.clone_into(&mut source.content);
        source.revision = source.revision.next().expect("revision does not overflow");
        Ok(())
    }

    /// Test harness: damages the stored bytes of a capture.
    ///
    /// # Errors
    /// `NotFound` for an unknown capture.
    pub fn corrupt_capture(&self, capture: &ArtifactId) -> Result<()> {
        let mut state = self.state();
        let (_, bytes) = state
            .captures
            .get_mut(capture)
            .ok_or_else(|| Self::not_found(capture))?;
        bytes.push(b'!');
        Ok(())
    }
}

#[async_trait]
impl EvidenceRecorder for MemoryEvidence {
    async fn capture(&self, source: &ArtifactRef, role: CaptureRole) -> Result<CapturedBytes> {
        let mut state = self.state();
        let (id, found) = match source {
            ArtifactRef::Id(id) => state.sources.get(id).map(|s| (*id, s.clone())),
            ArtifactRef::Slug(slug) => state
                .sources
                .iter()
                .find(|(_, s)| s.slug == slug.as_str())
                .map(|(id, s)| (*id, s.clone())),
        }
        .ok_or_else(|| FunctorError::not_found(EntityKind::Artifact, source.to_string()))?;
        let bytes = found.content.into_bytes();
        if bytes.len() as u64 > bounds::MAX_ITEM_BYTES {
            return Err(FunctorError::invalid_input(
                "content",
                "larger than the capture limit",
            ));
        }
        let capture = Capture {
            artifact: self.ids.artifact_id(),
            source: id,
            source_revision: found.revision,
            role,
            digest: fake_digest(&bytes),
            size: bytes.len() as u64,
        };
        state
            .captures
            .insert(capture.artifact, (capture.clone(), bytes.clone()));
        Ok(CapturedBytes { capture, bytes })
    }

    async fn record_intent(&self, intent: RunIntent) -> Result<IntentRecord> {
        let mut state = self.state();
        if !self.has(Defect::IntentNotIdempotent)
            && let Some(existing) = state.requests.get(&intent.request_id)
        {
            let entry = &state.runs[existing];
            return Ok(IntentRecord {
                record: Self::record(existing, entry),
                created: false,
            });
        }
        if !self.has(Defect::IntentSkipsCaptureCheck) {
            for capture in std::iter::once(&intent.workflow).chain(intent.inputs.values()) {
                if !state.captures.contains_key(capture) {
                    return Err(Self::not_found(capture));
                }
            }
        }
        let run = self.ids.artifact_id();
        state.requests.insert(intent.request_id.clone(), run);
        let entry = RunEntry {
            intent,
            result: None,
            streams: None,
        };
        let record = Self::record(&run, &entry);
        state.runs.insert(run, entry);
        Ok(IntentRecord {
            record,
            created: true,
        })
    }

    async fn acquire_lease(&self, run: &ArtifactId) -> Result<Box<dyn RunLease>> {
        {
            let state = self.state();
            let entry = state.runs.get(run).ok_or_else(|| Self::not_found(run))?;
            if entry.result.is_some() {
                return Err(FunctorError::conflict(
                    EntityKind::Artifact,
                    "the run is already complete",
                ));
            }
        }
        let mut held = self.held();
        if !self.has(Defect::LeaseNotExclusive) && held.contains(run) {
            return Err(FunctorError::Locked {
                path: PathBuf::from(format!("run:{run}")),
            });
        }
        held.insert(*run);
        Ok(Box::new(Lease {
            run: *run,
            leases: Arc::clone(&self.leases),
        }))
    }

    async fn record_result(&self, run: &ArtifactId, mut finish: RunFinish) -> Result<RunRecord> {
        let mut state = self.state();
        let entry = state.runs.get(run).ok_or_else(|| Self::not_found(run))?;
        if entry.result.is_some() && !self.has(Defect::SecondResultOverwrites) {
            return Err(FunctorError::conflict(
                EntityKind::Artifact,
                "the run already has a result",
            ));
        }
        if self.has(Defect::AcceptsInconsistentFinish) && finish.outcome.is_success() {
            finish.exit_code = Some(0);
            finish.diagnostics.clear();
        }
        let streams = RunStreams {
            stdout: finish.stdout,
            stderr: finish.stderr,
        };
        let outputs: Vec<ArtifactId> = finish
            .outputs
            .iter()
            .map(|_| self.ids.artifact_id())
            .collect();
        let spec = RunResultSpec {
            outcome: finish.outcome,
            finished: self.clock.now(),
            exit_code: finish.exit_code,
            truncated: finish.truncated,
            outputs,
            diagnostics: finish.diagnostics,
        };
        self.complete_run(&mut state, run, spec, streams)
    }

    async fn run(&self, run: &ArtifactRef) -> Result<Option<RunRecord>> {
        let state = self.state();
        let id = match run {
            ArtifactRef::Id(id) => Some(*id),
            ArtifactRef::Slug(_) => None,
        };
        Ok(id.and_then(|id| state.runs.get(&id).map(|entry| Self::record(&id, entry))))
    }

    async fn runs(&self) -> Result<Vec<RunRecord>> {
        Ok(self
            .state()
            .runs
            .iter()
            .map(|(id, entry)| Self::record(id, entry))
            .collect())
    }

    async fn status(&self, run: &ArtifactId) -> Result<RunStatus> {
        let state = self.state();
        let entry = state.runs.get(run).ok_or_else(|| Self::not_found(run))?;
        let record = Self::record(run, entry);
        Ok(record.status(self.held().contains(run)))
    }

    async fn streams(&self, run: &ArtifactId) -> Result<Option<RunStreams>> {
        let state = self.state();
        let entry = state.runs.get(run).ok_or_else(|| Self::not_found(run))?;
        Ok(entry.result.as_ref().and_then(|_| entry.streams.clone()))
    }

    async fn read_capture(&self, capture: &ArtifactId) -> Result<Vec<u8>> {
        let state = self.state();
        let (recorded, bytes) = state
            .captures
            .get(capture)
            .ok_or_else(|| Self::not_found(capture))?;
        if self.has(Defect::CaptureReadsLiveState)
            && let Some(source) = state.sources.get(&recorded.source)
        {
            return Ok(source.content.clone().into_bytes());
        }
        if !self.has(Defect::DigestNotVerified) && fake_digest(bytes) != recorded.digest {
            return Err(FunctorError::Inconsistent {
                detail: format!("capture {capture} does not match its digest"),
            });
        }
        Ok(bytes.clone())
    }

    async fn recover(&self) -> Result<Vec<RunRecord>> {
        let mut state = self.state();
        let orphans: Vec<ArtifactId> = state
            .runs
            .iter()
            .filter(|(id, entry)| {
                entry.result.is_none()
                    && (self.has(Defect::RecoverTakesRunningRuns) || !self.held().contains(id))
            })
            .map(|(id, _)| *id)
            .collect();
        let mut recovered = Vec::new();
        for run in orphans {
            let started = state.runs[&run].intent.started;
            let spec = RunResultSpec {
                outcome: RunOutcome::Interrupted,
                finished: self.clock.now().max(started),
                exit_code: None,
                truncated: Truncation::default(),
                outputs: Vec::new(),
                diagnostics: vec!["the run was interrupted before it finished".to_owned()],
            };
            let streams = RunStreams {
                stdout: Vec::new(),
                stderr: Vec::new(),
            };
            recovered.push(self.complete_run(&mut state, &run, spec, streams)?);
        }
        Ok(recovered)
    }
}

#[async_trait]
impl ReportRecorder for MemoryEvidence {
    async fn save_report(
        &self,
        report: &ValidationReport,
        subject: ReportSubject,
        _strict: bool,
        rules: &str,
    ) -> Result<ArtifactId> {
        if rules.is_empty() || rules.len() > 100 {
            return Err(FunctorError::invalid_input(
                "rules",
                "must be 1 to 100 characters",
            ));
        }
        let mut state = self.state();
        if let ReportSubject::Artifact { id, .. } = &subject
            && !state.sources.contains_key(id)
        {
            return Err(Self::not_found(id));
        }
        let id = self.ids.artifact_id();
        let stored = if self.has(Defect::ReportMutatedOnSave) {
            ValidationReport::new(Vec::new(), 0, 0, false, report.completed())
        } else {
            report.clone()
        };
        state.reports.insert(id, (stored, subject));
        Ok(id)
    }

    async fn load_report(
        &self,
        id: &ArtifactId,
    ) -> Result<Option<(ValidationReport, ReportSubject)>> {
        Ok(self.state().reports.get(id).cloned())
    }
}

/// What an evidence implementation must offer to be tested, beyond the recorder traits: a way to
/// create and change the live artifacts it captures, the digest it uses, and a way to damage a
/// stored capture.
#[async_trait]
pub trait EvidenceUnderTest: EvidenceRecorder + ReportRecorder {
    /// Creates a live artifact with inline text content and returns its id.
    async fn put_source(&self, slug: &str, content: &str) -> Result<ArtifactId>;

    /// Replaces the content of a live artifact (its revision increases).
    async fn edit_source(&self, id: &ArtifactId, content: &str) -> Result<()>;

    /// The digest this implementation computes for bytes.
    fn digest(&self, bytes: &[u8]) -> ContentDigest;

    /// Damages the stored bytes of a capture.
    async fn corrupt_capture(&self, capture: &ArtifactId) -> Result<()>;
}

#[async_trait]
impl EvidenceUnderTest for MemoryEvidence {
    async fn put_source(&self, slug: &str, content: &str) -> Result<ArtifactId> {
        Ok(MemoryEvidence::put_source(self, slug, content))
    }

    async fn edit_source(&self, id: &ArtifactId, content: &str) -> Result<()> {
        MemoryEvidence::edit_source(self, id, content)
    }

    fn digest(&self, bytes: &[u8]) -> ContentDigest {
        fake_digest(bytes)
    }

    async fn corrupt_capture(&self, capture: &ArtifactId) -> Result<()> {
        MemoryEvidence::corrupt_capture(self, capture)
    }
}

fn expect_err<T: Debug>(result: Result<T>, kind: ErrorKind, context: &str) {
    match result {
        Ok(value) => panic!("{context}: expected a {kind:?} error, got Ok({value:?})"),
        Err(error) => assert_eq!(error.kind(), kind, "{context}: got `{error}`"),
    }
}

/// A run intent with the standard shape, for the suite and for orchestration tests.
#[must_use]
pub fn sample_intent(
    request: &str,
    workflow: ArtifactId,
    inputs: &[(&str, ArtifactId)],
    started: Timestamp,
) -> RunIntent {
    RunIntent {
        request_id: RequestId::new(request).unwrap(),
        provider: LOCAL_PROCESS_PROVIDER.to_owned(),
        provider_version: "test".to_owned(),
        workflow,
        inputs: inputs
            .iter()
            .map(|(name, id)| ((*name).to_owned(), *id))
            .collect(),
        args: Vec::new(),
        limits: ExecutionLimits::DEFAULT,
        env_names: Vec::new(),
        program: ProgramProvenance {
            path: "tool".to_owned(),
            digest: None,
        },
        platform: "test".to_owned(),
        started,
    }
}

/// A successful finish with one `result` output, for the suite and for orchestration tests.
#[must_use]
pub fn sample_success() -> RunFinish {
    RunFinish {
        outcome: RunOutcome::Succeeded,
        exit_code: Some(0),
        truncated: Truncation::default(),
        diagnostics: Vec::new(),
        stdout: b"ok\n".to_vec(),
        stderr: Vec::new(),
        outputs: vec![functor_core::PublishedOutput {
            spec: functor_model::OutputSpec {
                name: "result".to_owned(),
                artifact_type: functor_model::ArtifactType::new("dataset").unwrap(),
                format: "text".to_owned(),
                required: true,
            },
            bytes: b"out:hello".to_vec(),
        }],
    }
}

fn failure(outcome: RunOutcome, exit_code: Option<i32>, diagnostics: &[&str]) -> RunFinish {
    RunFinish {
        outcome,
        exit_code,
        truncated: Truncation::default(),
        diagnostics: diagnostics.iter().map(|d| (*d).to_owned()).collect(),
        stdout: Vec::new(),
        stderr: b"boom\n".to_vec(),
        outputs: Vec::new(),
    }
}

struct Prepared {
    workflow: ArtifactId,
    input: ArtifactId,
}

async fn prepare<R: EvidenceUnderTest>(recorder: &R, tag: &str) -> Prepared {
    let workflow_source = recorder
        .put_source(&format!("wf-{tag}"), "workflow text")
        .await
        .unwrap();
    let input_source = recorder
        .put_source(&format!("in-{tag}"), "hello")
        .await
        .unwrap();
    let workflow = recorder
        .capture(&ArtifactRef::Id(workflow_source), CaptureRole::Workflow)
        .await
        .unwrap()
        .capture
        .artifact;
    let input = recorder
        .capture(&ArtifactRef::Id(input_source), CaptureRole::Input)
        .await
        .unwrap()
        .capture
        .artifact;
    Prepared { workflow, input }
}

async fn start_run<R: EvidenceUnderTest>(recorder: &R, env: &Env, request: &str) -> ArtifactId {
    let prepared = prepare(recorder, request).await;
    *recorder
        .record_intent(sample_intent(
            request,
            prepared.workflow,
            &[("data", prepared.input)],
            env.now(),
        ))
        .await
        .unwrap()
        .record
        .run()
}

/// Runs every scenario, each against a fresh empty recorder built by `factory`.
pub async fn run_evidence_conformance<R, F>(factory: F)
where
    R: EvidenceUnderTest,
    F: Fn(&Env) -> Fresh<R>,
{
    macro_rules! scenario {
        ($name:ident) => {{
            let env = Env::new();
            let fresh = factory(&env);
            $name(&fresh.store, &env).await;
            drop(fresh);
        }};
    }

    scenario!(a_capture_records_the_exact_state);
    scenario!(a_capture_survives_later_edits);
    scenario!(capturing_an_unknown_artifact_is_not_found);
    scenario!(intent_is_idempotent_by_request_id);
    scenario!(intent_needs_existing_captures);
    scenario!(leases_are_exclusive_and_released_on_drop);
    scenario!(status_follows_the_records);
    scenario!(the_result_is_the_commit_point);
    scenario!(an_inconsistent_finish_writes_nothing);
    scenario!(failures_are_recorded_as_outcomes);
    scenario!(recovery_completes_only_orphans);
    scenario!(damaged_captures_are_detected);
    scenario!(runs_are_listed_in_id_order);
    scenario!(reports_round_trip_and_stay_immutable);
    scenario!(reports_need_an_existing_subject);
}

/// A capture holds the exact bytes, digest, size, source and revision.
pub async fn a_capture_records_the_exact_state<R: EvidenceUnderTest>(recorder: &R, _env: &Env) {
    let source = recorder.put_source("input", "hello").await.unwrap();
    let captured = recorder
        .capture(&ArtifactRef::Id(source), CaptureRole::Input)
        .await
        .unwrap();
    assert_eq!(captured.bytes, b"hello");
    let capture = &captured.capture;
    assert_eq!(capture.source, source);
    assert_eq!(capture.source_revision, Revision::FIRST);
    assert_eq!(capture.role, CaptureRole::Input);
    assert_eq!(capture.size, 5);
    assert_eq!(capture.digest, recorder.digest(b"hello"));
    assert_ne!(capture.artifact, source, "a capture is its own artifact");
    assert_eq!(
        recorder.read_capture(&capture.artifact).await.unwrap(),
        b"hello"
    );
}

/// Editing the live artifact afterwards does not change an existing capture, and a new capture
/// sees the new state.
pub async fn a_capture_survives_later_edits<R: EvidenceUnderTest>(recorder: &R, _env: &Env) {
    let source = recorder.put_source("input", "hello").await.unwrap();
    let first = recorder
        .capture(&ArtifactRef::Id(source), CaptureRole::Input)
        .await
        .unwrap()
        .capture;
    recorder.edit_source(&source, "changed").await.unwrap();

    assert_eq!(
        recorder.read_capture(&first.artifact).await.unwrap(),
        b"hello",
        "the original capture is still inspectable"
    );
    let second = recorder
        .capture(&ArtifactRef::Id(source), CaptureRole::Input)
        .await
        .unwrap();
    assert_eq!(second.bytes, b"changed");
    assert_eq!(second.capture.source_revision.get(), 2);
    assert_ne!(second.capture.digest, first.digest);
    assert_ne!(second.capture.artifact, first.artifact);
}

/// Capturing something that does not exist is `NotFound` and records nothing.
pub async fn capturing_an_unknown_artifact_is_not_found<R: EvidenceUnderTest>(
    recorder: &R,
    _env: &Env,
) {
    expect_err(
        recorder
            .capture(
                &ArtifactRef::Id(ArtifactId::from_bits(9999)),
                CaptureRole::Input,
            )
            .await,
        ErrorKind::NotFound,
        "unknown artifact",
    );
}

/// Recording the same `request_id` twice returns the first run and writes nothing.
pub async fn intent_is_idempotent_by_request_id<R: EvidenceUnderTest>(recorder: &R, env: &Env) {
    let prepared = prepare(recorder, "a").await;
    let intent = sample_intent(
        "req-1",
        prepared.workflow,
        &[("data", prepared.input)],
        env.now(),
    );
    let first = recorder.record_intent(intent.clone()).await.unwrap();
    assert!(first.created);
    env.advance(10);
    let mut again = intent;
    again.started = env.now();
    let second = recorder.record_intent(again).await.unwrap();
    assert!(!second.created, "the request was already recorded");
    assert_eq!(second.record.run(), first.record.run());
    assert_eq!(
        second.record.intent().started,
        first.record.intent().started,
        "the original intent is kept"
    );
    assert_eq!(
        recorder.runs().await.unwrap().len(),
        1,
        "no second run exists"
    );
}

/// An intent that references a capture which does not exist is refused and records nothing.
pub async fn intent_needs_existing_captures<R: EvidenceUnderTest>(recorder: &R, env: &Env) {
    let prepared = prepare(recorder, "a").await;
    expect_err(
        recorder
            .record_intent(sample_intent(
                "req-1",
                prepared.workflow,
                &[("data", ArtifactId::from_bits(9999))],
                env.now(),
            ))
            .await,
        ErrorKind::NotFound,
        "unknown input capture",
    );
    expect_err(
        recorder
            .record_intent(sample_intent(
                "req-2",
                ArtifactId::from_bits(9998),
                &[],
                env.now(),
            ))
            .await,
        ErrorKind::NotFound,
        "unknown workflow capture",
    );
    assert_eq!(recorder.runs().await.unwrap(), Vec::<RunRecord>::new());
}

/// Only one owner can hold a run; dropping the lease releases it; unknown and completed runs
/// cannot be leased.
pub async fn leases_are_exclusive_and_released_on_drop<R: EvidenceUnderTest>(
    recorder: &R,
    env: &Env,
) {
    let run = start_run(recorder, env, "req-1").await;
    let lease = recorder.acquire_lease(&run).await.unwrap();
    assert_eq!(lease.run(), &run);
    expect_err(
        recorder.acquire_lease(&run).await,
        ErrorKind::Locked,
        "a second owner",
    );
    drop(lease);
    let again = recorder.acquire_lease(&run).await.unwrap();
    drop(again);

    expect_err(
        recorder.acquire_lease(&ArtifactId::from_bits(9999)).await,
        ErrorKind::NotFound,
        "unknown run",
    );

    recorder
        .record_result(&run, sample_success())
        .await
        .unwrap();
    expect_err(
        recorder.acquire_lease(&run).await,
        ErrorKind::Conflict,
        "a completed run",
    );
}

/// Status is derived: running while the lease is held, interrupted-unknown without it, completed
/// once a result exists.
pub async fn status_follows_the_records<R: EvidenceUnderTest>(recorder: &R, env: &Env) {
    let run = start_run(recorder, env, "req-1").await;
    assert_eq!(
        recorder.status(&run).await.unwrap(),
        RunStatus::InterruptedUnknown,
        "no owner and no result"
    );
    let lease = recorder.acquire_lease(&run).await.unwrap();
    assert_eq!(recorder.status(&run).await.unwrap(), RunStatus::Running);
    recorder
        .record_result(&run, sample_success())
        .await
        .unwrap();
    drop(lease);
    assert_eq!(
        recorder.status(&run).await.unwrap(),
        RunStatus::Completed(RunOutcome::Succeeded)
    );
    expect_err(
        recorder.status(&ArtifactId::from_bits(9999)).await,
        ErrorKind::NotFound,
        "unknown run",
    );
}

/// The result is written once, with the injected time, the output ids and the streams; a second
/// result is a conflict and the first is unchanged.
pub async fn the_result_is_the_commit_point<R: EvidenceUnderTest>(recorder: &R, env: &Env) {
    let run = start_run(recorder, env, "req-1").await;
    assert!(
        recorder.streams(&run).await.unwrap().is_none(),
        "incomplete runs have no streams"
    );
    let before = recorder.run(&ArtifactRef::Id(run)).await.unwrap().unwrap();
    assert!(before.result().is_none());

    env.advance(30);
    let done = recorder
        .record_result(&run, sample_success())
        .await
        .unwrap();
    let (_, result) = done.result().expect("the run is complete");
    assert_eq!(result.outcome(), RunOutcome::Succeeded);
    assert_eq!(
        result.finished(),
        env.now(),
        "finished comes from the injected clock"
    );
    assert_eq!(result.outputs().len(), 1);
    assert_eq!(
        done.intent(),
        before.intent(),
        "the intent is never rewritten"
    );

    let streams = recorder.streams(&run).await.unwrap().unwrap();
    assert_eq!(streams.stdout, b"ok\n");

    expect_err(
        recorder
            .record_result(&run, failure(RunOutcome::Cancelled, None, &[]))
            .await,
        ErrorKind::Conflict,
        "second result",
    );
    let stored = recorder.run(&ArtifactRef::Id(run)).await.unwrap().unwrap();
    assert_eq!(stored, done, "the first result stands");
    expect_err(
        recorder
            .record_result(&ArtifactId::from_bits(9999), sample_success())
            .await,
        ErrorKind::NotFound,
        "unknown run",
    );
}

/// A finish that no honest run could produce is refused before anything is written.
pub async fn an_inconsistent_finish_writes_nothing<R: EvidenceUnderTest>(recorder: &R, env: &Env) {
    let run = start_run(recorder, env, "req-1").await;
    for (name, finish) in [
        (
            "a success with a non-zero exit status",
            RunFinish {
                exit_code: Some(1),
                ..sample_success()
            },
        ),
        (
            "outputs on a failure",
            RunFinish {
                outcome: RunOutcome::Failed,
                exit_code: Some(1),
                ..sample_success()
            },
        ),
        (
            "a success with failure diagnostics",
            RunFinish {
                diagnostics: vec!["missing".to_owned()],
                ..sample_success()
            },
        ),
        (
            "a start failure without a reason",
            failure(RunOutcome::StartFailed, None, &[]),
        ),
    ] {
        expect_err(
            recorder.record_result(&run, finish).await,
            ErrorKind::InvalidInput,
            name,
        );
        assert!(
            recorder.streams(&run).await.unwrap().is_none(),
            "{name}: nothing was written"
        );
        assert_eq!(
            recorder.status(&run).await.unwrap(),
            RunStatus::InterruptedUnknown,
            "{name}: the run is still incomplete"
        );
    }
}

/// Failures, timeouts, cancellations and start failures are recorded as outcomes without
/// outputs, keeping the streams.
pub async fn failures_are_recorded_as_outcomes<R: EvidenceUnderTest>(recorder: &R, env: &Env) {
    for (n, (outcome, exit_code, diagnostics)) in [
        (RunOutcome::Failed, Some(2), &[][..]),
        (RunOutcome::TimedOut, None, &[][..]),
        (RunOutcome::Cancelled, None, &[][..]),
        (RunOutcome::StartFailed, None, &["program not found"][..]),
    ]
    .into_iter()
    .enumerate()
    {
        let run = start_run(recorder, env, &format!("req-{n}")).await;
        let done = recorder
            .record_result(&run, failure(outcome, exit_code, diagnostics))
            .await
            .unwrap();
        let (_, result) = done.result().unwrap();
        assert_eq!(result.outcome(), outcome);
        assert_eq!(result.exit_code(), exit_code);
        assert!(
            result.outputs().is_empty(),
            "{outcome}: nothing is published"
        );
        assert_eq!(
            recorder.status(&run).await.unwrap(),
            RunStatus::Completed(outcome)
        );
        assert_eq!(
            recorder.streams(&run).await.unwrap().unwrap().stderr,
            b"boom\n"
        );
    }
}

/// Recovery completes runs whose owner is gone as `interrupted`, leaves running and complete
/// runs alone, and is idempotent.
pub async fn recovery_completes_only_orphans<R: EvidenceUnderTest>(recorder: &R, env: &Env) {
    let running = start_run(recorder, env, "running").await;
    let orphan = start_run(recorder, env, "orphan").await;
    let complete = start_run(recorder, env, "complete").await;
    recorder
        .record_result(&complete, sample_success())
        .await
        .unwrap();
    let lease = recorder.acquire_lease(&running).await.unwrap();

    env.advance(5);
    let recovered = recorder.recover().await.unwrap();
    let ids: Vec<&ArtifactId> = recovered.iter().map(RunRecord::run).collect();
    assert_eq!(ids, [&orphan], "only the orphan is recovered");
    let (_, result) = recovered[0].result().unwrap();
    assert_eq!(result.outcome(), RunOutcome::Interrupted);
    assert!(
        result.outputs().is_empty(),
        "recovery never publishes outputs"
    );

    assert_eq!(recorder.status(&running).await.unwrap(), RunStatus::Running);
    assert_eq!(
        recorder.status(&complete).await.unwrap(),
        RunStatus::Completed(RunOutcome::Succeeded)
    );
    assert_eq!(
        recorder.status(&orphan).await.unwrap(),
        RunStatus::Completed(RunOutcome::Interrupted)
    );
    assert!(
        recorder.recover().await.unwrap().is_empty(),
        "recovery is idempotent"
    );
    drop(lease);
}

/// Damaged or altered captured bytes are reported, not returned.
pub async fn damaged_captures_are_detected<R: EvidenceUnderTest>(recorder: &R, _env: &Env) {
    let source = recorder.put_source("input", "hello").await.unwrap();
    let capture = recorder
        .capture(&ArtifactRef::Id(source), CaptureRole::Input)
        .await
        .unwrap()
        .capture
        .artifact;
    recorder.corrupt_capture(&capture).await.unwrap();
    expect_err(
        recorder.read_capture(&capture).await,
        ErrorKind::Inconsistent,
        "damaged capture",
    );
    expect_err(
        recorder.read_capture(&ArtifactId::from_bits(9999)).await,
        ErrorKind::NotFound,
        "unknown capture",
    );
}

/// Runs are listed in id order, which is creation order.
pub async fn runs_are_listed_in_id_order<R: EvidenceUnderTest>(recorder: &R, env: &Env) {
    let first = start_run(recorder, env, "req-1").await;
    let second = start_run(recorder, env, "req-2").await;
    let listed: Vec<ArtifactId> = recorder
        .runs()
        .await
        .unwrap()
        .iter()
        .map(|r| *r.run())
        .collect();
    assert_eq!(listed, [first, second]);
    assert_eq!(
        recorder
            .run(&ArtifactRef::Id(ArtifactId::from_bits(9999)))
            .await
            .unwrap(),
        None
    );
}

fn sample_report(env: &Env) -> ValidationReport {
    ValidationReport::new(
        vec![Finding::new(
            RuleCode::new("V0012"),
            Severity::Warning,
            Subject::Repository,
            "custom relationship type `related_to`",
        )],
        2,
        1,
        false,
        env.now(),
    )
}

/// A saved report loads back unchanged with its subject; every save is a distinct immutable
/// copy.
pub async fn reports_round_trip_and_stay_immutable<R: EvidenceUnderTest>(recorder: &R, env: &Env) {
    let report = sample_report(env);
    let whole = recorder
        .save_report(&report, ReportSubject::Repository, false, RULES)
        .await
        .unwrap();
    let (loaded, subject) = recorder.load_report(&whole).await.unwrap().unwrap();
    assert_eq!(loaded, report, "the saved report is the computed report");
    assert_eq!(subject, ReportSubject::Repository);

    let source = recorder.put_source("claim", "text").await.unwrap();
    let scoped_subject = ReportSubject::Artifact {
        id: source,
        revision: Revision::FIRST,
        digest: Some(recorder.digest(b"text")),
    };
    let scoped = recorder
        .save_report(&report, scoped_subject.clone(), true, RULES)
        .await
        .unwrap();
    let again = recorder
        .save_report(&report, scoped_subject.clone(), true, RULES)
        .await
        .unwrap();
    assert_ne!(scoped, again, "every save is a new artifact");
    assert_ne!(scoped, whole);
    let (loaded, subject) = recorder.load_report(&scoped).await.unwrap().unwrap();
    assert_eq!(loaded, report);
    assert_eq!(subject, scoped_subject);

    assert_eq!(
        recorder.load_report(&source).await.unwrap(),
        None,
        "an artifact that is not a report"
    );
}

/// A report about an artifact that does not exist is refused, and nothing is stored; so is a
/// report without a rule-set identity.
pub async fn reports_need_an_existing_subject<R: EvidenceUnderTest>(recorder: &R, env: &Env) {
    expect_err(
        recorder
            .save_report(
                &sample_report(env),
                ReportSubject::Artifact {
                    id: ArtifactId::from_bits(9999),
                    revision: Revision::FIRST,
                    digest: None,
                },
                false,
                RULES,
            )
            .await,
        ErrorKind::NotFound,
        "unknown subject",
    );
    expect_err(
        recorder
            .save_report(&sample_report(env), ReportSubject::Repository, false, "")
            .await,
        ErrorKind::InvalidInput,
        "no rule-set identity",
    );
    expect_err(
        recorder
            .save_report(
                &sample_report(env),
                ReportSubject::Repository,
                false,
                &"x".repeat(101),
            )
            .await,
        ErrorKind::InvalidInput,
        "an over-long rule-set identity",
    );
}

/// The rule-set identity the suite saves reports with.
const RULES: &str = "V0001-V0023";

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;
    use crate::block_on::block_on;

    fn suite(defect: Option<Defect>) {
        block_on(run_evidence_conformance(|env: &Env| {
            Fresh::new(match defect {
                Some(defect) => MemoryEvidence::with_defect(env, defect),
                None => MemoryEvidence::new(env),
            })
        }));
    }

    #[test]
    fn a_correct_recorder_passes_the_suite() {
        suite(None);
    }

    #[test]
    fn the_suite_rejects_every_deliberately_broken_recorder() {
        for defect in [
            Defect::CaptureReadsLiveState,
            Defect::IntentNotIdempotent,
            Defect::IntentSkipsCaptureCheck,
            Defect::LeaseNotExclusive,
            Defect::SecondResultOverwrites,
            Defect::AcceptsInconsistentFinish,
            Defect::RecoverTakesRunningRuns,
            Defect::DigestNotVerified,
            Defect::ReportMutatedOnSave,
        ] {
            let outcome = catch_unwind(AssertUnwindSafe(|| suite(Some(defect))));
            assert!(outcome.is_err(), "the suite did not detect {defect:?}");
        }
    }

    #[test]
    fn fake_digests_are_stable_distinct_and_well_formed() {
        assert_eq!(fake_digest(b"a"), fake_digest(b"a"));
        assert_ne!(fake_digest(b"a"), fake_digest(b"b"));
        assert_ne!(fake_digest(b""), fake_digest(b"a"));
        assert!(ContentDigest::new(fake_digest(b"x").as_str()).is_ok());
    }

    #[test]
    fn recorders_are_usable_as_trait_objects() {
        let env = Env::new();
        let recorder = MemoryEvidence::new(&env);
        let evidence: &dyn EvidenceRecorder = &recorder;
        let reports: &dyn ReportRecorder = &recorder;
        let _ = (evidence, reports);
    }
}
