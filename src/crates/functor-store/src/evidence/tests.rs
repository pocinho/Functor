//! Tests of the durable evidence recorder: the shared conformance suite on both backends, then
//! what only a real repository can show (restart, moved directories, concurrency, damaged files
//! and a failure injected at every publication boundary).

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use functor_core::{
    ArtifactRepository, ErrorKind, EvidenceRecorder, FunctorError, InitOptions, IntentRecord,
    PublishedOutput, RelationshipRepository, ReportRecorder, ReportSubject, RepositoryReader,
    Result, RunFinish, RunLease,
};
use functor_model::RelationshipType::{DependsOn, DerivedFrom};
use functor_model::{
    Artifact, ArtifactId, ArtifactPatch, ArtifactRef, ArtifactType, CaptureRole, Content,
    ContentDigest, ContentFormat, ContentPath, EvidenceKind, NewArtifact, NewRelationship,
    OutputSpec, Relationship, RelationshipId, RelationshipType, RepositoryInfo, RunOutcome,
    RunStatus, Slug, Title,
};
use functor_repository::FileRepositoryManager;
use functor_testkit::{
    Env, EvidenceUnderTest, Fresh, run_evidence_conformance, sample_intent, sample_success,
};
use tempfile::TempDir;

use super::blobs::{EvidenceBlobs, FileBlobs, Guard};
use super::codec::sha256;
use super::{EvidenceBacking, EvidenceStore, FileEvidence, InMemoryEvidence};
use crate::{FileStore, StoreOptions};

// ---------------------------------------------------------------------------------------------
// harness shared by both backends
// ---------------------------------------------------------------------------------------------

async fn put_source<S: EvidenceBacking>(store: &S, slug: &str, text: &str) -> Result<ArtifactId> {
    let new = NewArtifact::new(
        Slug::parse(slug)?,
        ArtifactType::document(),
        Title::new(slug)?,
    )
    .with_content(Content::Inline {
        format: ContentFormat::new("text")?,
        text: text.to_owned(),
    });
    Ok(*ArtifactRepository::create(store, new).await?.id())
}

async fn edit_source<S: EvidenceBacking>(store: &S, id: &ArtifactId, text: &str) -> Result<()> {
    let patch = ArtifactPatch::new().with_content(Content::Inline {
        format: ContentFormat::new("text")?,
        text: text.to_owned(),
    });
    ArtifactRepository::update(store, id, patch, None).await?;
    Ok(())
}

async fn capture_path<S: EvidenceBacking>(store: &S, capture: &ArtifactId) -> Result<ContentPath> {
    let artifact = ArtifactRepository::get(store, capture)
        .await?
        .ok_or_else(|| FunctorError::not_found(functor_model::EntityKind::Artifact, "capture"))?;
    match artifact.content() {
        Content::Path { path, .. } => Ok(path.clone()),
        _ => Err(FunctorError::Internal("capture without a file".to_owned())),
    }
}

#[async_trait]
impl EvidenceUnderTest for InMemoryEvidence {
    async fn put_source(&self, slug: &str, content: &str) -> Result<ArtifactId> {
        put_source(self.store(), slug, content).await
    }

    async fn edit_source(&self, id: &ArtifactId, content: &str) -> Result<()> {
        edit_source(self.store(), id, content).await
    }

    fn digest(&self, bytes: &[u8]) -> ContentDigest {
        sha256(bytes)
    }

    async fn corrupt_capture(&self, capture: &ArtifactId) -> Result<()> {
        let path = capture_path(self.store(), capture).await?;
        let _ = self.blobs().damage(&path);
        Ok(())
    }
}

#[async_trait]
impl EvidenceUnderTest for FileEvidence {
    async fn put_source(&self, slug: &str, content: &str) -> Result<ArtifactId> {
        put_source(self.store(), slug, content).await
    }

    async fn edit_source(&self, id: &ArtifactId, content: &str) -> Result<()> {
        edit_source(self.store(), id, content).await
    }

    fn digest(&self, bytes: &[u8]) -> ContentDigest {
        sha256(bytes)
    }

    async fn corrupt_capture(&self, capture: &ArtifactId) -> Result<()> {
        let path = capture_path(self.store(), capture).await?;
        let file = self.blobs().layout().content_file(&path);
        let mut bytes = fs::read(&file).map_err(|e| FunctorError::storage("read", &file, e))?;
        bytes.push(b'!');
        fs::write(&file, bytes).map_err(|e| FunctorError::storage("write", &file, e))
    }
}

fn init_repository(env: &Env) -> (TempDir, FileEvidence) {
    let dir = tempfile::tempdir().unwrap();
    let manager = FileRepositoryManager::new(env.clock(), env.ids());
    let handle = manager
        .init_blocking(dir.path(), &InitOptions::new().with_name("Evidence"))
        .unwrap();
    let store = FileStore::with_options(
        &handle,
        env.clock(),
        env.ids(),
        StoreOptions::default().with_lock_timeout(std::time::Duration::from_secs(10)),
    );
    (dir, FileEvidence::open(store, env.clock(), env.ids()))
}

fn reopen(env: &Env, root: &std::path::Path) -> FileEvidence {
    let manager = FileRepositoryManager::new(env.clock(), env.ids());
    let handle = manager.open_blocking(root).unwrap();
    FileEvidence::open(
        FileStore::new(&handle, env.clock(), env.ids()),
        env.clock(),
        env.ids(),
    )
}

#[tokio::test]
async fn the_memory_recorder_passes_the_conformance_suite() {
    run_evidence_conformance(|env: &Env| {
        Fresh::new(InMemoryEvidence::in_memory(env.clock(), env.ids()))
    })
    .await;
}

#[tokio::test]
async fn the_file_recorder_passes_the_conformance_suite() {
    run_evidence_conformance(|env: &Env| {
        let (dir, evidence) = init_repository(env);
        Fresh::with_guard(evidence, dir)
    })
    .await;
}

// ---------------------------------------------------------------------------------------------
// a complete run, used by most tests below
// ---------------------------------------------------------------------------------------------

struct Finished {
    workflow_source: ArtifactId,
    input_source: ArtifactId,
    workflow_capture: ArtifactId,
    input_capture: ArtifactId,
    run: ArtifactId,
    result: ArtifactId,
    output: ArtifactId,
}

fn success(output: &[u8]) -> RunFinish {
    let mut finish = sample_success();
    finish.outputs = vec![PublishedOutput {
        spec: OutputSpec {
            name: "result".to_owned(),
            artifact_type: ArtifactType::new("dataset").unwrap(),
            format: "text".to_owned(),
            required: true,
        },
        bytes: output.to_vec(),
    }];
    finish
}

async fn complete_run(evidence: &FileEvidence, env: &Env, request: &str) -> Finished {
    let workflow_source = evidence
        .put_source(&format!("wf-{request}"), "program")
        .await
        .unwrap();
    let input_source = evidence
        .put_source(&format!("in-{request}"), "hello")
        .await
        .unwrap();
    let workflow = evidence
        .capture(&ArtifactRef::Id(workflow_source), CaptureRole::Workflow)
        .await
        .unwrap()
        .capture
        .artifact;
    let input = evidence
        .capture(&ArtifactRef::Id(input_source), CaptureRole::Input)
        .await
        .unwrap()
        .capture
        .artifact;
    let IntentRecord { record, created } = evidence
        .record_intent(sample_intent(
            request,
            workflow,
            &[("data", input)],
            env.now(),
        ))
        .await
        .unwrap();
    assert!(created);
    let run = *record.run();
    let lease = evidence.acquire_lease(&run).await.unwrap();
    let done = evidence
        .record_result(&run, success(b"out:hello"))
        .await
        .unwrap();
    drop(lease);
    let (result, stored) = done.result().unwrap().clone();
    Finished {
        workflow_source,
        input_source,
        workflow_capture: workflow,
        input_capture: input,
        run,
        result,
        output: stored.outputs()[0],
    }
}

fn edge_set(relationships: &[Relationship]) -> HashSet<(ArtifactId, RelationshipType, ArtifactId)> {
    relationships
        .iter()
        .map(|r| (*r.source(), r.relationship_type().clone(), *r.target()))
        .collect()
}

#[tokio::test]
async fn evidence_is_ordinary_artifacts_and_relationships_that_trace_both_ways() {
    let env = Env::new();
    let (_dir, evidence) = init_repository(&env);
    let r = complete_run(&evidence, &env, "req-1").await;

    let snapshot = RepositoryReader::snapshot(evidence.store()).await.unwrap();
    let kind = |id: &ArtifactId| EvidenceKind::of(snapshot.artifact(id).unwrap());
    assert_eq!(kind(&r.workflow_capture), Some(EvidenceKind::Capture));
    assert_eq!(kind(&r.input_capture), Some(EvidenceKind::Capture));
    assert_eq!(kind(&r.run), Some(EvidenceKind::Run));
    assert_eq!(kind(&r.result), Some(EvidenceKind::RunResult));
    assert_eq!(kind(&r.output), None, "an output is an ordinary artifact");
    assert_eq!(
        snapshot
            .artifact(&r.output)
            .unwrap()
            .artifact_type()
            .as_str(),
        "dataset"
    );

    let edges = edge_set(snapshot.relationships());
    for expected in [
        (r.workflow_capture, DerivedFrom, r.workflow_source),
        (r.input_capture, DerivedFrom, r.input_source),
        (r.run, DependsOn, r.workflow_capture),
        (r.run, DependsOn, r.input_capture),
        (r.result, DerivedFrom, r.run),
        (r.output, DerivedFrom, r.result),
    ] {
        assert!(edges.contains(&expected), "missing {expected:?}");
    }
    assert_eq!(edges.len(), 6, "no other edges");

    // Forward from the output to the original input, and back from the input to the output.
    let mut forward = vec![r.output];
    let mut seen = HashSet::new();
    while let Some(id) = forward.pop() {
        if seen.insert(id) {
            forward.extend(snapshot.outgoing(&id).map(|e| *e.target()));
        }
    }
    for reached in [
        r.result,
        r.run,
        r.workflow_capture,
        r.input_capture,
        r.input_source,
    ] {
        assert!(seen.contains(&reached));
    }
    let mut backward = vec![r.input_source];
    let mut dependents = HashSet::new();
    while let Some(id) = backward.pop() {
        if dependents.insert(id) {
            backward.extend(snapshot.incoming(&id).map(|e| *e.source()));
        }
    }
    for reached in [r.input_capture, r.run, r.result, r.output] {
        assert!(dependents.contains(&reached));
    }

    let load = RepositoryReader::load(evidence.store()).await.unwrap();
    assert!(
        load.issues.is_empty(),
        "the repository stays valid: {:?}",
        load.issues
    );
}

#[tokio::test]
async fn evidence_files_are_content_addressed_under_content_evidence() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let r = complete_run(&evidence, &env, "req-1").await;

    let path = capture_path(evidence.store(), &r.input_capture)
        .await
        .unwrap();
    let digest = sha256(b"hello");
    assert_eq!(
        path.as_str(),
        format!("content/evidence/captures/{}", digest.hex())
    );
    assert_eq!(fs::read(dir.path().join(path.as_str())).unwrap(), b"hello");
    let areas: HashSet<String> = fs::read_dir(dir.path().join("content/evidence"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        areas,
        ["captures", "outputs", "results", "streams"]
            .map(String::from)
            .into()
    );
}

#[tokio::test]
async fn evidence_artifacts_cannot_be_updated_through_the_store() {
    let env = Env::new();
    let (_dir, evidence) = init_repository(&env);
    let r = complete_run(&evidence, &env, "req-1").await;
    for id in [r.workflow_capture, r.run, r.result] {
        let error = ArtifactRepository::update(
            evidence.store(),
            &id,
            ArtifactPatch::new().with_title(Title::new("edited").unwrap()),
            None,
        )
        .await
        .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Conflict, "{id}");
    }
    // Outputs are ordinary artifacts and stay editable.
    ArtifactRepository::update(
        evidence.store(),
        &r.output,
        ArtifactPatch::new().with_title(Title::new("annotated").unwrap()),
        None,
    )
    .await
    .unwrap();
}

// ---------------------------------------------------------------------------------------------
// durability: restart, moves, interrupted work
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn evidence_survives_a_restart_and_a_moved_repository() {
    let env = Env::new();
    let parent = tempfile::tempdir().unwrap();
    let first = parent.path().join("first");
    let manager = FileRepositoryManager::new(env.clock(), env.ids());
    let handle = manager.init_blocking(&first, &InitOptions::new()).unwrap();
    let evidence = FileEvidence::open(
        FileStore::new(&handle, env.clock(), env.ids()),
        env.clock(),
        env.ids(),
    );
    let r = complete_run(&evidence, &env, "req-1").await;
    let before = evidence.runs().await.unwrap();
    let streams_before = evidence.streams(&r.run).await.unwrap().unwrap();
    drop(evidence);

    let second = parent.path().join("second");
    fs::rename(&first, &second).unwrap();
    let reopened = reopen(&env, &second);
    assert_eq!(reopened.runs().await.unwrap(), before);
    assert_eq!(
        reopened.streams(&r.run).await.unwrap().unwrap(),
        streams_before
    );
    assert_eq!(
        reopened.read_capture(&r.input_capture).await.unwrap(),
        b"hello"
    );
    assert_eq!(
        reopened.status(&r.run).await.unwrap(),
        RunStatus::Completed(RunOutcome::Succeeded)
    );
}

#[tokio::test]
async fn reopening_exposes_interrupted_work_without_erasing_or_rerunning_it() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let source = evidence.put_source("wf", "program").await.unwrap();
    let capture = evidence
        .capture(&ArtifactRef::Id(source), CaptureRole::Workflow)
        .await
        .unwrap()
        .capture
        .artifact;
    let run = *evidence
        .record_intent(sample_intent("req-1", capture, &[], env.now()))
        .await
        .unwrap()
        .record
        .run();
    // The owner dies: its lease dies with it.
    drop(evidence.acquire_lease(&run).await.unwrap());
    drop(evidence);

    let reopened = reopen(&env, dir.path());
    let listed = reopened.runs().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert!(listed[0].result().is_none(), "nothing was invented");
    assert_eq!(
        reopened.status(&run).await.unwrap(),
        RunStatus::InterruptedUnknown
    );

    // The same request is never started twice.
    let again = reopened
        .record_intent(sample_intent("req-1", capture, &[], env.now()))
        .await
        .unwrap();
    assert!(!again.created);

    let recovered = reopened.recover().await.unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(
        reopened.status(&run).await.unwrap(),
        RunStatus::Completed(RunOutcome::Interrupted)
    );
    assert_eq!(reopened.recover().await.unwrap().len(), 0);
    assert_eq!(reopened.read_capture(&capture).await.unwrap(), b"program");
}

#[tokio::test]
async fn the_live_input_can_change_while_the_capture_stays_inspectable() {
    let env = Env::new();
    let (_dir, evidence) = init_repository(&env);
    let r = complete_run(&evidence, &env, "req-1").await;
    evidence
        .edit_source(&r.input_source, "changed after the run")
        .await
        .unwrap();
    assert_eq!(
        evidence.read_capture(&r.input_capture).await.unwrap(),
        b"hello"
    );
    let record = evidence
        .run(&ArtifactRef::Id(r.run))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(record.intent().inputs["data"], r.input_capture);
}

// ---------------------------------------------------------------------------------------------
// concurrency
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn concurrent_recordings_of_one_request_create_exactly_one_run() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let source = evidence.put_source("wf", "program").await.unwrap();
    let capture = evidence
        .capture(&ArtifactRef::Id(source), CaptureRole::Workflow)
        .await
        .unwrap()
        .capture
        .artifact;

    let mut tasks = Vec::new();
    for _ in 0..6 {
        let evidence = reopen(&env, dir.path());
        let started = env.now();
        tasks.push(tokio::spawn(async move {
            evidence
                .record_intent(sample_intent("same", capture, &[], started))
                .await
        }));
    }
    let mut created = 0;
    let mut runs = HashSet::new();
    for task in tasks {
        let outcome = task.await.unwrap().unwrap();
        created += usize::from(outcome.created);
        runs.insert(*outcome.record.run());
    }
    assert_eq!(created, 1, "exactly one recording created the run");
    assert_eq!(runs.len(), 1, "every caller saw the same run");
    assert_eq!(evidence.runs().await.unwrap().len(), 1);
}

#[tokio::test]
async fn concurrent_runs_and_captures_do_not_interfere() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let mut sources = Vec::new();
    for n in 0..4 {
        sources.push(
            evidence
                .put_source(&format!("wf-{n}"), &format!("program {n}"))
                .await
                .unwrap(),
        );
    }
    let mut tasks = Vec::new();
    for (n, source) in sources.into_iter().enumerate() {
        let evidence = reopen(&env, dir.path());
        let started = env.now();
        tasks.push(tokio::spawn(async move {
            let capture = evidence
                .capture(&ArtifactRef::Id(source), CaptureRole::Workflow)
                .await?
                .capture
                .artifact;
            let run = *evidence
                .record_intent(sample_intent(&format!("req-{n}"), capture, &[], started))
                .await?
                .record
                .run();
            let lease = evidence.acquire_lease(&run).await?;
            evidence
                .record_result(
                    &run,
                    RunFinish {
                        outputs: vec![],
                        ..sample_success()
                    },
                )
                .await?;
            drop(lease);
            Ok::<_, FunctorError>(run)
        }));
    }
    for task in tasks {
        task.await.unwrap().unwrap();
    }
    let runs = evidence.runs().await.unwrap();
    assert_eq!(runs.len(), 4);
    assert!(runs.iter().all(|r| r.result().is_some()));
    let load = RepositoryReader::load(evidence.store()).await.unwrap();
    assert!(load.issues.is_empty(), "{:?}", load.issues);
}

/// A blob store that rewrites a user content file right after it is read, to prove that captures
/// refuse a state that never existed.
struct ChangingBlobs {
    inner: FileBlobs,
    path: ContentPath,
    rewrites: AtomicUsize,
    max_rewrites: usize,
}

#[async_trait]
impl EvidenceBlobs for ChangingBlobs {
    async fn put(&self, area: &str, digest: &ContentDigest, bytes: Vec<u8>) -> Result<ContentPath> {
        self.inner.put(area, digest, bytes).await
    }

    async fn get(&self, path: &ContentPath) -> Result<Option<Vec<u8>>> {
        let bytes = self.inner.get(path).await?;
        if path == &self.path {
            let n = self.rewrites.fetch_add(1, Ordering::SeqCst);
            if n < self.max_rewrites {
                let file = self.inner.layout().content_file(path);
                fs::write(file, format!("version {}", n + 2)).unwrap();
            }
        }
        Ok(bytes)
    }

    async fn try_lease(&self, run: &ArtifactId) -> Result<Guard> {
        self.inner.try_lease(run).await
    }

    async fn is_leased(&self, run: &ArtifactId) -> Result<bool> {
        self.inner.is_leased(run).await
    }

    async fn lock_request(&self, request: &functor_model::RequestId) -> Result<Guard> {
        self.inner.lock_request(request).await
    }
}

async fn path_backed_source(
    evidence: &FileEvidence,
    dir: &TempDir,
    text: &str,
) -> (ArtifactId, ContentPath) {
    let path = ContentPath::new("content/live.txt").unwrap();
    fs::create_dir_all(dir.path().join("content")).unwrap();
    fs::write(dir.path().join(path.as_str()), text).unwrap();
    let new = NewArtifact::new(
        Slug::parse("live").unwrap(),
        ArtifactType::document(),
        Title::new("live").unwrap(),
    )
    .with_content(Content::Path {
        format: ContentFormat::new("text").unwrap(),
        path: path.clone(),
    });
    let id = *ArtifactRepository::create(evidence.store(), new)
        .await
        .unwrap()
        .id();
    (id, path)
}

fn changing(
    evidence: &FileEvidence,
    env: &Env,
    path: ContentPath,
    max_rewrites: usize,
) -> EvidenceStore<FileStore, ChangingBlobs> {
    let layout = evidence.store().layout().clone();
    EvidenceStore::new(
        evidence.store().clone(),
        ChangingBlobs {
            inner: FileBlobs::new(layout, std::time::Duration::from_secs(5)),
            path,
            rewrites: AtomicUsize::new(0),
            max_rewrites,
        },
        env.clock(),
        env.ids(),
    )
}

#[tokio::test]
async fn an_input_that_keeps_changing_during_capture_is_refused() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let (source, path) = path_backed_source(&evidence, &dir, "version 1").await;
    let racing = changing(&evidence, &env, path, usize::MAX);
    let error = racing
        .capture(&ArtifactRef::Id(source), CaptureRole::Input)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Conflict, "{error}");
    let artifacts = ArtifactRepository::list(racing.store()).await.unwrap();
    assert!(
        artifacts.iter().all(|a| EvidenceKind::of(a).is_none()),
        "no capture of a state that never existed"
    );
}

#[tokio::test]
async fn one_change_during_capture_is_retried_and_the_stable_state_is_captured() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let (source, path) = path_backed_source(&evidence, &dir, "version 1").await;
    // Only the first read triggers a rewrite: the second attempt sees a stable "version 2".
    let racing = changing(&evidence, &env, path, 1);
    let captured = racing
        .capture(&ArtifactRef::Id(source), CaptureRole::Input)
        .await
        .unwrap();
    assert_eq!(captured.bytes, b"version 2");
}

// ---------------------------------------------------------------------------------------------
// damaged, missing and refused
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn damaged_and_missing_evidence_is_reported_with_explicit_diagnostics() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let r = complete_run(&evidence, &env, "req-1").await;

    // A missing capture file is caught when the repository is read: the loader names the file.
    let capture_file = dir.path().join(
        capture_path(evidence.store(), &r.input_capture)
            .await
            .unwrap()
            .as_str(),
    );
    let original = fs::read(&capture_file).unwrap();
    fs::remove_file(&capture_file).unwrap();
    let error = evidence.read_capture(&r.input_capture).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Inconsistent);
    assert!(
        error.to_string().contains("content/evidence/captures/")
            && error.to_string().contains("does not exist"),
        "{error}"
    );
    fs::write(&capture_file, &original).unwrap();

    // Altered bytes are found by their digest.
    let mut altered = original.clone();
    altered.push(b'!');
    fs::write(&capture_file, altered).unwrap();
    let error = evidence.read_capture(&r.input_capture).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Inconsistent);
    assert!(error.to_string().contains("digest"), "{error}");
    fs::write(&capture_file, original).unwrap();

    // A damaged stream.
    let streams = dir.path().join("content/evidence/streams");
    for entry in fs::read_dir(&streams).unwrap() {
        let path = entry.unwrap().path();
        let mut bytes = fs::read(&path).unwrap();
        bytes.push(b'x');
        fs::write(path, bytes).unwrap();
    }
    let error = evidence.streams(&r.run).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Inconsistent);
    assert!(error.to_string().contains("digest"), "{error}");

    // A missing result file makes the run unreadable instead of silently incomplete.
    for entry in fs::read_dir(dir.path().join("content/evidence/results")).unwrap() {
        fs::remove_file(entry.unwrap().path()).unwrap();
    }
    let error = evidence.status(&r.run).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Inconsistent);
    assert!(
        error.to_string().contains("content/evidence/results/"),
        "{error}"
    );
}

#[tokio::test]
async fn a_hand_damaged_run_record_is_described_not_trusted() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let r = complete_run(&evidence, &env, "req-1").await;
    let file = dir.path().join(format!("artifacts/{}.toml", r.run));
    let text = fs::read_to_string(&file).unwrap();
    fs::write(&file, text.replace("\"workflow\"", "\"workflowx\"")).unwrap();
    let error = evidence.status(&r.run).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Inconsistent, "{error}");
    assert!(error.to_string().contains("evidence"), "{error}");
}

#[tokio::test]
async fn a_storage_fault_before_any_record_exists_leaves_nothing_behind() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let source = evidence.put_source("input", "hello").await.unwrap();
    // `content/evidence` cannot be created when a file is in the way.
    fs::create_dir_all(dir.path().join("content")).unwrap();
    fs::write(dir.path().join("content/evidence"), b"in the way").unwrap();
    let error = evidence
        .capture(&ArtifactRef::Id(source), CaptureRole::Input)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Storage, "{error}");
    let artifacts = ArtifactRepository::list(evidence.store()).await.unwrap();
    assert_eq!(artifacts.len(), 1, "only the source exists");
}

#[tokio::test]
async fn evidence_in_a_damaged_repository_is_refused_not_guessed() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let r = complete_run(&evidence, &env, "req-1").await;
    fs::write(
        dir.path().join(format!("artifacts/{}.toml", r.output)),
        "not = [toml",
    )
    .unwrap();
    let error = evidence.runs().await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Inconsistent);
    let error = evidence
        .record_result(&r.run, success(b"x"))
        .await
        .unwrap_err();
    assert_eq!(
        error.kind(),
        ErrorKind::Inconsistent,
        "writes are refused too"
    );
}

// ---------------------------------------------------------------------------------------------
// failure injection at every publication boundary
// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct FaultPlan {
    writes: Arc<AtomicUsize>,
    fail_at: Arc<AtomicUsize>,
}

impl FaultPlan {
    fn new(fail_at: usize) -> Self {
        Self {
            writes: Arc::new(AtomicUsize::new(0)),
            fail_at: Arc::new(AtomicUsize::new(fail_at)),
        }
    }

    fn check(&self) -> Result<()> {
        let n = self.writes.fetch_add(1, Ordering::SeqCst) + 1;
        if n == self.fail_at.load(Ordering::SeqCst) {
            return Err(FunctorError::storage(
                "write",
                PathBuf::from("injected"),
                io::Error::other(format!("injected failure at write {n}")),
            ));
        }
        Ok(())
    }

    fn heal(&self) {
        self.fail_at.store(0, Ordering::SeqCst);
    }
}

struct FaultyStore {
    inner: FileStore,
    plan: FaultPlan,
}

#[async_trait]
impl ArtifactRepository for FaultyStore {
    async fn create(&self, new: NewArtifact) -> Result<Artifact> {
        self.plan.check()?;
        ArtifactRepository::create(&self.inner, new).await
    }
    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>> {
        ArtifactRepository::get(&self.inner, id).await
    }
    async fn resolve(&self, reference: &ArtifactRef) -> Result<Option<Artifact>> {
        ArtifactRepository::resolve(&self.inner, reference).await
    }
    async fn list(&self) -> Result<Vec<Artifact>> {
        ArtifactRepository::list(&self.inner).await
    }
    async fn update(
        &self,
        id: &ArtifactId,
        patch: ArtifactPatch,
        expected: Option<functor_model::Revision>,
    ) -> Result<functor_model::Patched> {
        ArtifactRepository::update(&self.inner, id, patch, expected).await
    }
}

#[async_trait]
impl RelationshipRepository for FaultyStore {
    async fn create(&self, new: NewRelationship) -> Result<Relationship> {
        self.plan.check()?;
        RelationshipRepository::create(&self.inner, new).await
    }
    async fn get(&self, id: &RelationshipId) -> Result<Option<Relationship>> {
        RelationshipRepository::get(&self.inner, id).await
    }
    async fn list(&self) -> Result<Vec<Relationship>> {
        RelationshipRepository::list(&self.inner).await
    }
}

#[async_trait]
impl RepositoryReader for FaultyStore {
    async fn info(&self) -> Result<RepositoryInfo> {
        self.inner.info().await
    }
    async fn load(&self) -> Result<functor_core::SnapshotLoad> {
        self.inner.load().await
    }
}

struct FaultyBlobs {
    inner: FileBlobs,
    plan: FaultPlan,
}

#[async_trait]
impl EvidenceBlobs for FaultyBlobs {
    async fn put(&self, area: &str, digest: &ContentDigest, bytes: Vec<u8>) -> Result<ContentPath> {
        self.plan.check()?;
        self.inner.put(area, digest, bytes).await
    }
    async fn get(&self, path: &ContentPath) -> Result<Option<Vec<u8>>> {
        self.inner.get(path).await
    }
    async fn try_lease(&self, run: &ArtifactId) -> Result<Guard> {
        self.inner.try_lease(run).await
    }
    async fn is_leased(&self, run: &ArtifactId) -> Result<bool> {
        self.inner.is_leased(run).await
    }
    async fn lock_request(&self, request: &functor_model::RequestId) -> Result<Guard> {
        self.inner.lock_request(request).await
    }
}

type Faulty = EvidenceStore<FaultyStore, FaultyBlobs>;

fn faulty(env: &Env, evidence: &FileEvidence, plan: &FaultPlan) -> Faulty {
    let layout = evidence.store().layout().clone();
    EvidenceStore::new(
        FaultyStore {
            inner: evidence.store().clone(),
            plan: plan.clone(),
        },
        FaultyBlobs {
            inner: FileBlobs::new(layout, std::time::Duration::from_secs(5)),
            plan: plan.clone(),
        },
        env.clock(),
        env.ids(),
    )
}

#[derive(Debug, PartialEq, Eq)]
enum Step {
    Done,
    FailedAt(&'static str),
}

/// Runs the whole workflow, stopping at the first error, exactly as an orchestrator would.
async fn drive(
    e: &Faulty,
    env: &Env,
    wf: ArtifactId,
    input: ArtifactId,
) -> (Step, Option<ArtifactId>, bool) {
    let Ok(workflow) = e.capture(&ArtifactRef::Id(wf), CaptureRole::Workflow).await else {
        return (Step::FailedAt("capture workflow"), None, false);
    };
    let Ok(captured) = e.capture(&ArtifactRef::Id(input), CaptureRole::Input).await else {
        return (Step::FailedAt("capture input"), None, false);
    };
    let intent = sample_intent(
        "req-1",
        workflow.capture.artifact,
        &[("data", captured.capture.artifact)],
        env.now(),
    );
    let Ok(recorded) = e.record_intent(intent).await else {
        return (Step::FailedAt("record intent"), None, false);
    };
    let run = *recorded.record.run();
    let Ok(lease) = e.acquire_lease(&run).await else {
        return (Step::FailedAt("lease"), Some(run), false);
    };
    let reported = e.record_result(&run, success(b"out:hello")).await;
    drop(lease);
    match reported {
        Ok(_) => (Step::Done, Some(run), true),
        Err(_) => (Step::FailedAt("record result"), Some(run), false),
    }
}

#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn a_failure_at_every_write_never_reports_a_false_success_and_always_recovers() {
    // Count the writes of a clean run first.
    let clean_env = Env::new();
    let (_clean_dir, clean_repo) = init_repository(&clean_env);
    let wf = clean_repo.put_source("wf", "program").await.unwrap();
    let input = clean_repo.put_source("input", "hello").await.unwrap();
    let plan = FaultPlan::new(0);
    let clean = faulty(&clean_env, &clean_repo, &plan);
    let (step, _, committed) = drive(&clean, &clean_env, wf, input).await;
    assert_eq!(step, Step::Done);
    assert!(committed);
    let total = plan.writes.load(Ordering::SeqCst);
    assert!(
        total >= 15,
        "the workflow has many publication boundaries: {total}"
    );

    let mut outcomes = HashSet::new();
    for fail_at in 1..=total {
        let env = Env::new();
        let (dir, repo) = init_repository(&env);
        let wf = repo.put_source("wf", "program").await.unwrap();
        let input = repo.put_source("input", "hello").await.unwrap();
        let plan = FaultPlan::new(fail_at);
        let flaky = faulty(&env, &repo, &plan);

        let (step, _, reported_success) = drive(&flaky, &env, wf, input).await;
        let Step::FailedAt(failed) = step else {
            panic!("write {fail_at} should have failed the workflow");
        };
        assert!(
            !reported_success,
            "write {fail_at}: a failure must not report success"
        );
        outcomes.insert(failed);

        // The storage comes back. Nothing is rerun; recovery only completes and repairs.
        plan.heal();
        let healed = reopen(&env, dir.path());
        let before = healed.runs().await.unwrap();
        for record in &before {
            let status = healed.status(record.run()).await.unwrap();
            if let RunStatus::Completed(outcome) = status {
                // A run is complete only through its result, and only the commit point does it.
                assert_eq!(outcome, RunOutcome::Succeeded, "write {fail_at}");
            }
        }
        // Before the intent's run artifact exists there is no run; the intent step itself may
        // fail after the artifact was written (its edges), which recovery then completes.
        if matches!(failed, "capture workflow" | "capture input") {
            assert!(
                before.is_empty(),
                "write {fail_at}: no run before the intent exists"
            );
        }
        assert!(
            before.len() <= 1,
            "write {fail_at}: never more than one run"
        );

        healed.recover().await.unwrap();
        let after = healed.runs().await.unwrap();
        for record in &after {
            let (_, result) = record.result().expect("recovery completes every orphan");
            match result.outcome() {
                RunOutcome::Interrupted => assert_eq!(result.outputs().len(), 0),
                RunOutcome::Succeeded => {
                    for output in result.outputs() {
                        let artifact = ArtifactRepository::get(healed.store(), output)
                            .await
                            .unwrap();
                        assert!(
                            artifact.is_some(),
                            "write {fail_at}: a listed output exists"
                        );
                    }
                    let streams = healed.streams(record.run()).await.unwrap();
                    assert_eq!(streams.unwrap().stdout, b"ok\n");
                }
                other => panic!("write {fail_at}: unexpected outcome {other}"),
            }
        }

        // Every edge the records imply exists after recovery.
        let snapshot = RepositoryReader::snapshot(healed.store()).await.unwrap();
        let edges = edge_set(snapshot.relationships());
        for run in &after {
            let intent = run.intent();
            for capture in std::iter::once(&intent.workflow).chain(intent.inputs.values()) {
                assert!(
                    edges.contains(&(*run.run(), RelationshipType::DependsOn, *capture)),
                    "write {fail_at}: run depends on its capture"
                );
            }
        }
        for artifact in snapshot.artifacts() {
            if EvidenceKind::of(artifact) == Some(EvidenceKind::Capture) {
                let capture = super::codec::decode_capture(artifact).unwrap();
                assert!(
                    edges.contains(&(
                        *artifact.id(),
                        RelationshipType::DerivedFrom,
                        capture.source
                    )),
                    "write {fail_at}: capture derives from its source"
                );
            }
        }
        let load = RepositoryReader::load(healed.store()).await.unwrap();
        assert!(load.issues.is_empty(), "write {fail_at}: {:?}", load.issues);
    }
    for step in [
        "capture workflow",
        "capture input",
        "record intent",
        "record result",
    ] {
        assert!(
            outcomes.contains(step),
            "no failure landed in `{step}`: {outcomes:?}"
        );
    }
}

#[tokio::test]
async fn a_failure_after_the_commit_point_is_reported_and_repaired_by_recovery() {
    // Fail the last write of a clean run: an edge after the result artifact exists.
    let probe_env = Env::new();
    let (_d, probe_repo) = init_repository(&probe_env);
    let wf = probe_repo.put_source("wf", "p").await.unwrap();
    let input = probe_repo.put_source("input", "hello").await.unwrap();
    let probe_plan = FaultPlan::new(0);
    let probe = faulty(&probe_env, &probe_repo, &probe_plan);
    drive(&probe, &probe_env, wf, input).await;
    let total = probe_plan.writes.load(Ordering::SeqCst);

    let env = Env::new();
    let (dir, repo) = init_repository(&env);
    let wf = repo.put_source("wf", "p").await.unwrap();
    let input = repo.put_source("input", "hello").await.unwrap();
    let plan = FaultPlan::new(total);
    let flaky = faulty(&env, &repo, &plan);
    let (step, run, success) = drive(&flaky, &env, wf, input).await;
    assert_eq!(step, Step::FailedAt("record result"));
    assert!(!success, "a failed mandatory write is never a success");
    let run = run.unwrap();

    plan.heal();
    let healed = reopen(&env, dir.path());
    assert_eq!(
        healed.status(&run).await.unwrap(),
        RunStatus::Completed(RunOutcome::Succeeded),
        "the commit point was reached"
    );
    let before = edge_set(&RelationshipRepository::list(healed.store()).await.unwrap());
    assert!(
        healed.recover().await.unwrap().is_empty(),
        "no run needed completing"
    );
    let after = edge_set(&RelationshipRepository::list(healed.store()).await.unwrap());
    assert!(after.len() > before.len(), "the missing edge was repaired");
}

#[tokio::test]
async fn a_lease_taken_by_this_process_blocks_recovery_of_that_run() {
    let env = Env::new();
    let (_dir, evidence) = init_repository(&env);
    let source = evidence.put_source("wf", "p").await.unwrap();
    let capture = evidence
        .capture(&ArtifactRef::Id(source), CaptureRole::Workflow)
        .await
        .unwrap()
        .capture
        .artifact;
    let run = *evidence
        .record_intent(sample_intent("req-1", capture, &[], env.now()))
        .await
        .unwrap()
        .record
        .run();
    let lease: Box<dyn RunLease> = evidence.acquire_lease(&run).await.unwrap();
    assert_eq!(evidence.recover().await.unwrap().len(), 0);
    assert_eq!(evidence.status(&run).await.unwrap(), RunStatus::Running);
    drop(lease);
}

#[tokio::test]
async fn reports_are_saved_as_files_with_a_validates_edge_to_their_subject() {
    let env = Env::new();
    let (dir, evidence) = init_repository(&env);
    let source = evidence.put_source("claim", "text").await.unwrap();
    let report = functor_model::ValidationReport::new(vec![], 1, 0, false, env.now());
    let id = evidence
        .save_report(
            &report,
            ReportSubject::Artifact {
                id: source,
                revision: functor_model::Revision::FIRST,
                digest: Some(sha256(b"text")),
            },
            false,
        )
        .await
        .unwrap();
    let snapshot = RepositoryReader::snapshot(evidence.store()).await.unwrap();
    assert!(edge_set(snapshot.relationships()).contains(&(
        id,
        RelationshipType::Validates,
        source
    )));
    let reports = dir.path().join("content/evidence/reports");
    let files: Vec<_> = fs::read_dir(reports).unwrap().collect();
    assert_eq!(files.len(), 1);
    let text = fs::read_to_string(files[0].as_ref().unwrap().path()).unwrap();
    assert!(text.contains("\"schema_version\": 1") && text.contains("\"outcome\": \"passed\""));
}
