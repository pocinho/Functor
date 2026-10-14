//! End-to-end tests of the run flow with everything real: a repository on disk, the file-backed
//! evidence and event log, and the `local-process` provider running a real child program.

// Tests assert; a failing unwrap is the failure.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::assert_is_empty
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use functor_core::{
    ArtifactRepository, CancelToken, ErrorKind, EvidenceRecorder, ExecutionGrant, InitOptions,
};
use functor_events::{EventLogOptions, FileEventLog};
use functor_model::{
    ArtifactId, ArtifactRef, ArtifactType, Content, ContentFormat, ExecutionLimits, InputBinding,
    NewArtifact, RequestId, RunOutcome, RunRequest, RunStatus, Slug, Title,
};
use functor_repository::{FileRepositoryManager, Layout};
use functor_runtime::{LocalProcessProvider, RunOrchestrator, RunReport};
use functor_store::{FileEvidence, FileStore, StoreOptions};
use functor_testkit::Env;
use tempfile::TempDir;

const FIXTURE: &str = env!("CARGO_BIN_EXE_runtime-fixture");

fn scratch_root() -> PathBuf {
    let root = std::env::var_os("FUNCTOR_TEST_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/scratch"),
        PathBuf::from,
    );
    fs::create_dir_all(&root).unwrap();
    fs::canonicalize(root).unwrap()
}

fn request(id: &str, workflow: ArtifactId, input: ArtifactId) -> RunRequest {
    RunRequest::new(
        ArtifactRef::Id(workflow),
        vec![InputBinding {
            name: "data".to_owned(),
            artifact: ArtifactRef::Id(input),
        }],
        RequestId::new(id).unwrap(),
    )
    .unwrap()
}

/// A repository on disk with the fixture program installed and every collaborator real.
struct World {
    dir: TempDir,
    env: Env,
    evidence: Arc<FileEvidence>,
    provider: Arc<LocalProcessProvider>,
    orchestrator: RunOrchestrator,
    tool: String,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("e2e-")
            .tempdir_in(scratch_root())
            .unwrap();
        let env = Env::new();
        let manager = FileRepositoryManager::new(env.clock(), env.ids());
        let handle = manager
            .init_blocking(dir.path(), &InitOptions::new().with_name("End to end"))
            .unwrap();
        fs::create_dir_all(dir.path().join("tools")).unwrap();
        let tool = format!("tools/runtime-fixture{}", std::env::consts::EXE_SUFFIX);
        let target = dir.path().join(&tool);
        if fs::hard_link(FIXTURE, &target).is_err() {
            fs::copy(FIXTURE, &target).unwrap();
        }
        let store = FileStore::with_options(
            &handle,
            env.clock(),
            env.ids(),
            StoreOptions::default().with_lock_timeout(Duration::from_secs(20)),
        );
        let evidence = Arc::new(FileEvidence::open(store, env.clock(), env.ids()));
        let provider = Arc::new(LocalProcessProvider::new(dir.path()));
        let log = Arc::new(FileEventLog::new(
            dir.path(),
            *handle.info.id(),
            env.clock(),
            env.ids(),
            EventLogOptions::default(),
        ));
        let orchestrator = RunOrchestrator::new(
            evidence.clone(),
            evidence.clone(),
            provider.clone(),
            env.clock(),
        )
        .with_events(log);
        Self {
            dir,
            env,
            evidence,
            provider,
            orchestrator,
            tool,
        }
    }

    fn layout(&self) -> Layout {
        Layout::new(self.dir.path())
    }

    /// The granted program is the resolved path of the workflow's program.
    fn grant(&self) -> ExecutionGrant {
        let resolved = self
            .provider
            .resolve_program(&self.tool)
            .unwrap()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        ExecutionGrant::new(resolved, ExecutionLimits::DEFAULT).with_env("LANG")
    }

    async fn document(&self, slug: &str, kind: &str, format: &str, text: &str) -> ArtifactId {
        let new = NewArtifact::new(
            Slug::parse(slug).unwrap(),
            ArtifactType::new(kind).unwrap(),
            Title::new(slug).unwrap(),
        )
        .with_content(Content::Inline {
            format: ContentFormat::new(format).unwrap(),
            text: text.to_owned(),
        });
        *ArtifactRepository::create(self.evidence.store(), new)
            .await
            .unwrap()
            .id()
    }

    async fn workflow(&self, slug: &str, args: &[&str]) -> ArtifactId {
        let args: Vec<String> = args.iter().map(|a| format!("{a:?}")).collect();
        let text = format!(
            r#"schema = "functor.workflow/1"
provider = "local-process"
program = "{}"
args = [{}]

[inputs]
data = {{ required = true }}

[outputs]
result = {{ type = "dataset", format = "text", required = true }}

[env]
allow = ["LANG"]
"#,
            self.tool,
            args.join(", ")
        );
        self.document(slug, "workflow", "toml", &text).await
    }

    async fn echo_workflow(&self) -> ArtifactId {
        self.workflow("echo", &["echo-input", "inputs/data", "outputs/result"])
            .await
    }

    async fn run(
        &self,
        request: &RunRequest,
    ) -> Result<RunReport, Box<functor_runtime::RunFailure>> {
        self.orchestrator
            .execute(request, self.grant(), CancelToken::new(), None)
            .await
    }

    /// The bytes of the file behind an evidence artifact.
    async fn bytes(&self, id: &ArtifactId) -> Vec<u8> {
        let artifact = ArtifactRepository::get(self.evidence.store(), id)
            .await
            .unwrap()
            .unwrap();
        let Content::Path { path, .. } = artifact.content() else {
            panic!("an artifact with a file");
        };
        fs::read(self.layout().content_file(path)).unwrap()
    }

    fn event_log(&self) -> String {
        fs::read_to_string(self.layout().events_log()).unwrap_or_default()
    }

    fn reopened(&self) -> FileEvidence {
        let manager = FileRepositoryManager::new(self.env.clock(), self.env.ids());
        let handle = manager.open_blocking(self.dir.path()).unwrap();
        FileEvidence::open(
            FileStore::new(&handle, self.env.clock(), self.env.ids()),
            self.env.clock(),
            self.env.ids(),
        )
    }
}

#[tokio::test]
async fn a_real_run_is_recorded_durably_and_survives_a_restart() {
    let world = World::new();
    let workflow = world.echo_workflow().await;
    let input = world.document("input", "document", "text", "hello").await;

    let report = world.run(&request("req-1", workflow, input)).await.unwrap();
    assert!(report.created);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    let (result_id, result) = report.record.result().unwrap();
    assert_eq!(result.outcome(), RunOutcome::Succeeded);
    assert_eq!(result.exit_code(), Some(0));
    assert_eq!(world.bytes(&result.outputs()[0]).await, b"out:hello");

    let log = world.event_log();
    assert!(log.contains("run.started"), "{log}");
    assert!(log.contains("run.completed"), "{log}");

    // A new process sees the same run, complete, with its streams and no evidence lost.
    let reopened = world.reopened();
    let again = reopened
        .run(&ArtifactRef::Id(*report.record.run()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(again.result().unwrap().0, *result_id);
    assert_eq!(
        reopened.status(report.record.run()).await.unwrap(),
        RunStatus::Completed(RunOutcome::Succeeded)
    );
    let streams = reopened
        .streams(report.record.run())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(streams.stdout, b"ok\n");

    // The live input can change; the captured evidence cannot.
    let capture = again.intent().inputs["data"];
    assert_eq!(world.bytes(&capture).await, b"hello");
}

#[tokio::test]
async fn replaying_a_captured_input_reproduces_the_output_in_a_new_run() {
    let world = World::new();
    let workflow = world.echo_workflow().await;
    let input = world.document("input", "document", "text", "hello").await;
    let first = world.run(&request("req-1", workflow, input)).await.unwrap();

    // The capture is an artifact: binding it re-runs against exactly what the first run saw.
    let capture = first.record.intent().inputs["data"];
    let second = world
        .run(&request("req-2", workflow, capture))
        .await
        .unwrap();
    assert_ne!(first.record.run(), second.record.run());
    let output = |report: &RunReport| report.record.result().unwrap().1.outputs()[0];
    assert_eq!(
        world.bytes(&output(&first)).await,
        world.bytes(&output(&second)).await
    );
    assert_eq!(world.evidence.runs().await.unwrap().len(), 2);
}

#[tokio::test]
async fn a_failing_program_is_an_outcome_with_diagnostics_and_no_outputs() {
    let world = World::new();
    let workflow = world.workflow("fails", &["stderr-exit", "3"]).await;
    let input = world.document("input", "document", "text", "hello").await;
    let report = world.run(&request("req-1", workflow, input)).await.unwrap();
    let (_, result) = report.record.result().unwrap();
    assert_eq!(result.outcome(), RunOutcome::Failed);
    assert_eq!(result.exit_code(), Some(3));
    assert!(result.outputs().is_empty());
    let streams = world
        .evidence
        .streams(report.record.run())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(streams.stderr, b"boom\n");
}

#[tokio::test]
async fn a_timeout_is_recorded_as_an_outcome() {
    let world = World::new();
    let workflow = world.workflow("slow", &["sleep", "60000"]).await;
    let input = world.document("input", "document", "text", "hello").await;
    let limited = request("req-1", workflow, input)
        .with_limits(ExecutionLimits::new(1, ExecutionLimits::DEFAULT.stream_bytes()).unwrap());
    let begun = Instant::now();
    let report = world.run(&limited).await.unwrap();
    assert_eq!(
        report.record.result().unwrap().1.outcome(),
        RunOutcome::TimedOut
    );
    assert!(begun.elapsed() < Duration::from_secs(50), "bounded wait");
}

#[tokio::test]
async fn cancelling_a_running_child_records_a_cancelled_run() {
    let world = World::new();
    let workflow = world.workflow("slow", &["sleep", "60000"]).await;
    let input = world.document("input", "document", "text", "hello").await;
    let token = CancelToken::new();
    let canceller = token.clone();
    let req = request("req-1", workflow, input);
    let run = world.orchestrator.execute(&req, world.grant(), token, None);
    let cancel = async move {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        canceller.cancel();
    };
    let (report, ()) = tokio::join!(run, cancel);
    let report = report.unwrap();
    assert_eq!(
        report.record.result().unwrap().1.outcome(),
        RunOutcome::Cancelled
    );
    assert_eq!(
        world.evidence.status(report.record.run()).await.unwrap(),
        RunStatus::Completed(RunOutcome::Cancelled)
    );
}

#[tokio::test]
async fn a_grant_for_another_program_is_refused_and_leaves_no_run() {
    let world = World::new();
    let workflow = world.echo_workflow().await;
    let input = world.document("input", "document", "text", "hello").await;
    let wrong = ExecutionGrant::new("tools/other", ExecutionLimits::DEFAULT).with_env("LANG");
    let failure = world
        .orchestrator
        .execute(
            &request("req-1", workflow, input),
            wrong,
            CancelToken::new(),
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(failure.kind(), ErrorKind::PermissionDenied);
    assert!(world.evidence.runs().await.unwrap().is_empty());
    assert!(!world.event_log().contains("run.started"));
}

#[tokio::test]
async fn an_unwritable_event_log_costs_warnings_not_evidence() {
    let world = World::new();
    let workflow = world.echo_workflow().await;
    let input = world.document("input", "document", "text", "hello").await;
    // A file where the events directory belongs: every append fails.
    let events = world.layout().events_dir();
    if events.exists() {
        fs::remove_dir_all(&events).unwrap();
    }
    fs::write(events, b"in the way").unwrap();

    let report = world.run(&request("req-1", workflow, input)).await.unwrap();
    assert_eq!(
        report.record.result().unwrap().1.outcome(),
        RunOutcome::Succeeded
    );
    assert!(!report.warnings.is_empty(), "the lost events are reported");
}

#[tokio::test]
async fn deleting_the_event_log_does_not_hinder_inspecting_evidence() {
    let world = World::new();
    let workflow = world.echo_workflow().await;
    let input = world.document("input", "document", "text", "hello").await;
    let report = world.run(&request("req-1", workflow, input)).await.unwrap();
    let events = world.layout().events_dir();
    if events.exists() {
        fs::remove_dir_all(events).unwrap();
    }
    let reopened = world.reopened();
    let record = reopened
        .run(&ArtifactRef::Id(*report.record.run()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(record.result().unwrap().1.outcome(), RunOutcome::Succeeded);
    assert_eq!(
        world.bytes(&record.result().unwrap().1.outputs()[0]).await,
        b"out:hello"
    );
}

#[tokio::test]
async fn the_same_request_id_started_twice_runs_one_child() {
    let world = World::new();
    let workflow = world.echo_workflow().await;
    let input = world.document("input", "document", "text", "hello").await;
    let req = request("req-1", workflow, input);
    let (a, b) = tokio::join!(world.run(&req), world.run(&req));
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(a.record.run(), b.record.run());
    assert_eq!(
        usize::from(a.created) + usize::from(b.created),
        1,
        "exactly one request creates the run"
    );
    assert_eq!(world.evidence.runs().await.unwrap().len(), 1);
    assert_eq!(world.provider.started(), 1);
}

#[tokio::test]
async fn the_repository_validates_cleanly_after_runs() {
    use functor_core::{ValidationEngine, ValidationTarget};
    let world = World::new();
    let workflow = world.echo_workflow().await;
    let input = world.document("input", "document", "text", "hello").await;
    world.run(&request("req-1", workflow, input)).await.unwrap();
    let engine =
        functor_validation::RuleValidationEngine::new(world.evidence.store(), world.env.clock());
    let report = engine.validate(ValidationTarget::Repository).await.unwrap();
    assert_eq!(
        report.outcome(),
        functor_model::Outcome::Passed,
        "{:?}",
        report.findings()
    );
}
