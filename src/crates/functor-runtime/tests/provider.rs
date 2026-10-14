//! Real-process tests of the `local-process` provider: the shared provider conformance suite
//! against a real child, then every safety control at its boundary.

// Tests assert; a failing unwrap is the failure.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use functor_core::{
    CancelToken, ErrorKind, ExecutionGrant, ExecutionPlan, ExecutionProvider, InputPayload,
    ProviderRun,
};
use functor_model::{
    ArtifactId, ArtifactType, ContentDigest, ExecutionLimits, InputSpec, LOCAL_PROCESS_PROVIDER,
    OutputSpec, RunOutcome, Workflow, WorkflowSpec, bounds,
};
use functor_runtime::LocalProcessProvider;
use functor_testkit::{ProviderScenario, ProviderUnderTest, run_provider_conformance};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const FIXTURE: &str = env!("CARGO_BIN_EXE_runtime-fixture");

fn digest(bytes: &[u8]) -> ContentDigest {
    ContentDigest::from_bytes(Sha256::digest(bytes).into())
}

/// A repository directory holding the fixture program at `tools/`.
struct Repo {
    dir: TempDir,
    provider: LocalProcessProvider,
    tool: String,
}

/// Where tests create their repositories: a git-ignored folder inside this repository
/// (`tests/scratch`, or `FUNCTOR_TEST_DIR`), so failed runs can be inspected and the folder can be
/// excluded from virus scanning. Each repository removes itself when the test ends.
fn scratch_root() -> PathBuf {
    let root = std::env::var_os("FUNCTOR_TEST_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/scratch"),
        PathBuf::from,
    );
    fs::create_dir_all(&root).unwrap();
    fs::canonicalize(root).unwrap()
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("runtime-")
            .tempdir_in(scratch_root())
            .unwrap();
        fs::create_dir_all(dir.path().join("tools")).unwrap();
        let tool = format!("tools/runtime-fixture{}", std::env::consts::EXE_SUFFIX);
        // A hard link means the operating system scans the freshly built program once, not once
        // per test; fall back to a copy across volumes.
        let target = dir.path().join(&tool);
        if fs::hard_link(FIXTURE, &target).is_err() {
            fs::copy(FIXTURE, &target).unwrap();
        }
        let provider = LocalProcessProvider::new(dir.path());
        Self {
            dir,
            provider,
            tool,
        }
    }

    fn granted(&self, program: &str) -> ExecutionGrant {
        let resolved = self
            .provider
            .resolve_program(program)
            .unwrap()
            .map_or_else(|| program.to_owned(), |p| p.to_string_lossy().into_owned());
        ExecutionGrant::new(resolved, ExecutionLimits::DEFAULT).with_env("LANG")
    }

    /// A plan in the standard shape (input `data`, required output `result`).
    fn plan_for(&self, program: &str, args: &[&str]) -> ExecutionPlan {
        self.custom(program, args, |_| {})
    }

    fn custom(
        &self,
        program: &str,
        args: &[&str],
        change: impl FnOnce(&mut WorkflowSpec),
    ) -> ExecutionPlan {
        let mut spec = WorkflowSpec {
            provider: LOCAL_PROCESS_PROVIDER.to_owned(),
            program: program.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
            inputs: vec![InputSpec {
                name: "data".to_owned(),
                required: true,
            }],
            outputs: vec![OutputSpec {
                name: "result".to_owned(),
                artifact_type: ArtifactType::new("dataset").unwrap(),
                format: "text".to_owned(),
                required: true,
            }],
            env_allow: vec!["LANG".to_owned()],
            ..WorkflowSpec::default()
        };
        change(&mut spec);
        ExecutionPlan {
            run: ArtifactId::from_bits(1),
            workflow: Workflow::new(spec).unwrap(),
            inputs: vec![InputPayload {
                name: "data".to_owned(),
                digest: digest(b"hello"),
                bytes: b"hello".to_vec(),
            }],
            limits: ExecutionLimits::DEFAULT,
            grant: self.granted(program),
        }
    }

    fn fixture(&self, args: &[&str]) -> ExecutionPlan {
        self.plan_for(&self.tool, args)
    }

    async fn run(&self, plan: ExecutionPlan) -> ProviderRun {
        let run = self
            .provider
            .execute(plan.clone(), CancelToken::new())
            .await
            .unwrap();
        run.check(&plan).unwrap();
        run
    }
}

impl ProviderUnderTest for Repo {
    fn provider(&self) -> &dyn ExecutionProvider {
        &self.provider
    }

    fn plan(&self, scenario: ProviderScenario) -> ExecutionPlan {
        match scenario {
            ProviderScenario::Succeeds => {
                self.fixture(&["echo-input", "inputs/data", "outputs/result"])
            }
            ProviderScenario::ExitsNonZero => self.fixture(&["exit", "3"]),
            ProviderScenario::ProgramMissing => {
                self.plan_for("definitely-missing-program-xyz", &[])
            }
            ProviderScenario::RequiredOutputMissing => self.fixture(&["exit", "0"]),
            ProviderScenario::TimesOut => {
                let mut plan = self.fixture(&["sleep", "60000"]);
                plan.limits = ExecutionLimits::new(1, plan.limits.stream_bytes()).unwrap();
                plan
            }
            ProviderScenario::FloodsOutput => self.fixture(&["flood", "65536"]),
            ProviderScenario::WaitsForCancel => self.fixture(&["sleep", "60000"]),
            _ => unreachable!("every scenario is mapped"),
        }
    }

    fn started(&self) -> usize {
        self.provider.started()
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[tokio::test]
async fn the_local_process_provider_passes_the_conformance_suite() {
    let repo = Repo::new();
    run_provider_conformance(&repo).await;
}

#[tokio::test]
async fn success_runs_the_real_program_with_inputs_staged_and_outputs_collected() {
    let repo = Repo::new();
    let run = repo
        .run(repo.fixture(&["echo-input", "inputs/data", "outputs/result"]))
        .await;
    assert_eq!(run.outcome, RunOutcome::Succeeded);
    assert_eq!(run.exit_code, Some(0));
    assert_eq!(run.outputs[0].bytes, b"out:hello");
    assert_eq!(text(&run.stdout.bytes), "ok\n");
    assert!(
        run.program
            .path
            .ends_with(&format!("runtime-fixture{}", std::env::consts::EXE_SUFFIX))
    );
    assert!(run.program.digest.is_some(), "the program is fingerprinted");
    assert_eq!(
        run.program.digest,
        Some(digest(&fs::read(repo.dir.path().join(&repo.tool)).unwrap()))
    );
}

#[tokio::test]
async fn a_nonzero_exit_keeps_status_and_stderr_and_publishes_nothing() {
    let repo = Repo::new();
    let run = repo.run(repo.fixture(&["stderr-exit", "7"])).await;
    assert_eq!(run.outcome, RunOutcome::Failed);
    assert_eq!(run.exit_code, Some(7));
    assert_eq!(text(&run.stderr.bytes), "boom\n");
    assert_eq!(run.outputs, Vec::new());
    assert_ne!(run.diagnostics, Vec::<String>::new());
}

#[tokio::test]
async fn a_program_that_exists_but_cannot_start_is_a_start_failure() {
    let repo = Repo::new();
    fs::write(
        repo.dir.path().join("tools/not-a-program.exe"),
        b"plain text",
    )
    .unwrap();
    let plan = repo.plan_for("tools/not-a-program.exe", &[]);
    let run = repo.run(plan).await;
    assert_eq!(run.outcome, RunOutcome::StartFailed);
    assert!(run.diagnostics[0].contains("could not be started"));
    assert_eq!(run.exit_code, None);
}

#[tokio::test]
async fn the_resolved_program_must_be_the_granted_one() {
    let repo = Repo::new();
    let before = repo.provider.started();
    let mut plan = repo.fixture(&["exit", "0"]);
    plan.grant = ExecutionGrant::new("some-other-program", ExecutionLimits::DEFAULT);
    let error = repo
        .provider
        .execute(plan, CancelToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::PermissionDenied, "{error}");
    assert_eq!(repo.provider.started(), before, "nothing ran");
    assert!(
        !repo.provider.work_dir(&ArtifactId::from_bits(1)).exists(),
        "nothing was staged"
    );
}

#[tokio::test]
async fn a_program_path_outside_the_repository_is_denied() {
    let repo = Repo::new();
    let outside = repo.dir.path().parent().unwrap().join("outside-tool.exe");
    fs::write(&outside, b"x").unwrap();
    let error = repo
        .provider
        .resolve_program("tools/../../outside-tool.exe")
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::PermissionDenied);
    fs::remove_file(outside).unwrap();
}

#[tokio::test]
async fn tampered_inputs_are_refused_before_anything_is_staged() {
    let repo = Repo::new();
    let mut plan = repo.fixture(&["echo-input", "inputs/data", "outputs/result"]);
    plan.inputs[0].bytes = b"tampered".to_vec();
    let error = repo
        .provider
        .execute(plan, CancelToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(repo.provider.started(), 0);
}

#[tokio::test]
async fn the_child_runs_in_a_fresh_work_directory_with_closed_stdin() {
    let repo = Repo::new();
    let plan = repo.fixture(&["cwd"]);
    let work = repo.provider.work_dir(&plan.run);
    let mut tolerant = plan.clone();
    tolerant.workflow = Workflow::new(WorkflowSpec {
        outputs: vec![],
        ..plan.workflow.spec().clone()
    })
    .unwrap();
    let run = repo.run(tolerant).await;
    assert_eq!(
        fs::canonicalize(text(&run.stdout.bytes)).unwrap(),
        fs::canonicalize(&work).unwrap(),
        "the working directory is the staging directory"
    );
    assert!(work.join("inputs/data").is_file() && work.join("outputs").is_dir());

    let eof = repo
        .run(repo.custom(&repo.tool.clone(), &["stdin-eof"], |s| s.outputs.clear()))
        .await;
    assert_eq!(
        text(&eof.stdout.bytes),
        "0",
        "stdin is closed, not inherited"
    );

    // A second execution of the same run starts from an empty directory.
    let first = repo
        .run(repo.fixture(&["echo-input", "inputs/data", "outputs/result"]))
        .await;
    assert_eq!(first.outcome, RunOutcome::Succeeded);
    let second = repo.run(repo.fixture(&["exit", "0"])).await;
    assert_eq!(
        second.outcome,
        RunOutcome::Failed,
        "the stale output is gone"
    );
    assert!(second.diagnostics[0].contains("`result` is missing"));
}

#[tokio::test]
async fn the_environment_is_cleared_to_an_allowlist() {
    let repo = Repo::new();
    let passed = if cfg!(windows) { "USERNAME" } else { "HOME" };
    let leaked_candidates: &[&str] = if cfg!(windows) {
        &[
            "USERPROFILE",
            "COMPUTERNAME",
            "HOMEPATH",
            "APPDATA",
            "USERDOMAIN",
        ]
    } else {
        &["USER", "SHELL", "LOGNAME", "PWD"]
    };
    let value = std::env::var(passed).expect("the test host defines the variable");

    let allowed = |plan_grant: bool| {
        let mut plan = repo.custom(&repo.tool.clone(), &["env"], |s| {
            s.outputs.clear();
            s.env_allow = vec![passed.to_owned()];
            s.env_set
                .insert("FUNCTOR_MODE".to_owned(), "fast".to_owned());
        });
        plan.grant = ExecutionGrant::new(plan.grant.program(), ExecutionLimits::DEFAULT);
        if plan_grant {
            plan.grant = plan.grant.with_env(passed);
        }
        plan
    };

    let run = repo.run(allowed(true)).await;
    let listing = text(&run.stdout.bytes);
    let names: Vec<&str> = listing
        .lines()
        .filter_map(|line| line.split_once('=').map(|(name, _)| name))
        .collect();
    assert!(listing.contains("FUNCTOR_MODE=fast"), "a literal is set");
    assert!(
        listing
            .lines()
            .any(|l| l.eq_ignore_ascii_case(&format!("{passed}={value}"))),
        "an allowed and granted variable passes through"
    );
    for name in leaked_candidates {
        assert!(
            !names.iter().any(|n| n.eq_ignore_ascii_case(name)),
            "`{name}` must not reach the child: {names:?}"
        );
    }
    let expected: Vec<String> = repo.provider.env_names(&allowed(true).workflow);
    assert!(expected.contains(&"FUNCTOR_MODE".to_owned()));
    assert!(
        expected
            .iter()
            .all(|e| names.iter().any(|n| n.eq_ignore_ascii_case(e)))
    );

    // Allowed by the workflow but not granted: refused before anything runs.
    let error = repo
        .provider
        .execute(allowed(false), CancelToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::PermissionDenied);
}

#[tokio::test]
async fn binary_output_is_preserved_exactly() {
    let repo = Repo::new();
    let run = repo
        .run(repo.custom(&repo.tool.clone(), &["binary"], |s| s.outputs.clear()))
        .await;
    assert_eq!(run.stdout.bytes, [0, 255, 254, 128, 10, 13, 0]);
}

#[tokio::test]
async fn stream_limits_hold_at_the_boundary() {
    let repo = Repo::new();
    for (written, expect_truncated) in [(15_u64, false), (16, false), (17, true)] {
        let mut plan = repo.custom(
            &repo.tool.clone(),
            &["flood", &written.to_string(), "stderr"],
            |s| s.outputs.clear(),
        );
        plan.limits = ExecutionLimits::new(60, 16).unwrap();
        let run = repo.run(plan).await;
        let kept = usize::try_from(written.min(16)).unwrap();
        assert_eq!(run.stdout.bytes.len(), kept, "stdout at {written}");
        assert_eq!(run.stderr.bytes.len(), kept, "stderr at {written}");
        assert_eq!(
            run.stdout.truncated, expect_truncated,
            "stdout at {written}"
        );
        assert_eq!(
            run.stderr.truncated, expect_truncated,
            "stderr at {written}"
        );
        assert_eq!(
            run.outcome,
            RunOutcome::Succeeded,
            "truncation is not a failure"
        );
    }
}

#[tokio::test]
async fn a_large_flood_is_drained_without_blocking_the_child() {
    let repo = Repo::new();
    let mut plan = repo.custom(&repo.tool.clone(), &["flood", "10000000", "stderr"], |s| {
        s.outputs.clear();
    });
    plan.limits = ExecutionLimits::new(60, 1024).unwrap();
    let started = Instant::now();
    let run = repo.run(plan).await;
    assert_eq!(run.outcome, RunOutcome::Succeeded);
    assert_eq!(run.stdout.bytes.len(), 1024);
    assert!(run.stdout.truncated && run.stderr.truncated);
    assert!(started.elapsed() < Duration::from_secs(120));
}

#[tokio::test]
async fn output_size_limits_hold_at_the_boundary() {
    let repo = Repo::new();
    let item = bounds::MAX_ITEM_BYTES.to_string();
    let over = (bounds::MAX_ITEM_BYTES + 1).to_string();
    let single = |size: &str| repo.fixture(&["big-output", "outputs/result", size]);
    let below = repo
        .run(single(&(bounds::MAX_ITEM_BYTES - 1).to_string()))
        .await;
    assert_eq!(
        below.outcome,
        RunOutcome::Succeeded,
        "{:?}",
        below.diagnostics
    );
    assert_eq!(
        below.outputs[0].bytes.len() as u64,
        bounds::MAX_ITEM_BYTES - 1
    );
    let at = repo.run(single(&item)).await;
    assert_eq!(at.outcome, RunOutcome::Succeeded, "{:?}", at.diagnostics);
    assert_eq!(at.outputs[0].bytes.len() as u64, bounds::MAX_ITEM_BYTES);
    let past = repo.run(single(&over)).await;
    assert_eq!(past.outcome, RunOutcome::Failed);
    assert!(
        past.diagnostics[0].contains("larger than"),
        "{:?}",
        past.diagnostics
    );
    assert_eq!(past.outputs, Vec::new());

    // Total output: four items at the item limit are exactly the total limit; one more byte in a
    // fifth output exceeds it.
    let four_and_small = |count: &str, last: u64| {
        let outputs: Vec<OutputSpec> = (0..5)
            .map(|n| OutputSpec {
                name: format!("o{n}"),
                artifact_type: ArtifactType::document(),
                format: "text".to_owned(),
                required: false,
            })
            .collect();
        repo.custom(
            &repo.tool.clone(),
            &["big-outputs", count, &item, &last.to_string()],
            |s| s.outputs = outputs,
        )
    };
    let below = repo
        .run(four_and_small("4", bounds::MAX_ITEM_BYTES - 1))
        .await;
    assert_eq!(
        below.outcome,
        RunOutcome::Succeeded,
        "{:?}",
        below.diagnostics
    );
    assert_eq!(
        below
            .outputs
            .iter()
            .map(|o| o.bytes.len() as u64)
            .sum::<u64>(),
        bounds::MAX_TOTAL_BYTES - 1
    );
    let fits = repo.run(four_and_small("5", 0)).await;
    assert_eq!(
        fits.outcome,
        RunOutcome::Succeeded,
        "{:?}",
        fits.diagnostics
    );
    assert_eq!(
        fits.outputs
            .iter()
            .map(|o| o.bytes.len() as u64)
            .sum::<u64>(),
        bounds::MAX_TOTAL_BYTES
    );
    let exceeds = repo.run(four_and_small("5", 1)).await;
    assert_eq!(exceeds.outcome, RunOutcome::Failed);
    assert!(
        exceeds.diagnostics[0].contains("in total"),
        "{:?}",
        exceeds.diagnostics
    );
}

#[tokio::test]
async fn missing_and_malformed_outputs_are_failures_with_diagnostics() {
    let repo = Repo::new();
    let missing = repo.run(repo.fixture(&["exit", "0"])).await;
    assert_eq!(missing.outcome, RunOutcome::Failed);
    assert_eq!(missing.exit_code, Some(0));
    assert!(missing.diagnostics[0].contains("required output `result` is missing"));

    let directory = repo
        .run(repo.fixture(&["dir-output", "outputs/result"]))
        .await;
    assert_eq!(directory.outcome, RunOutcome::Failed);
    assert!(directory.diagnostics[0].contains("not a regular file"));

    let optional = repo
        .run(repo.custom(&repo.tool.clone(), &["exit", "0"], |s| {
            s.outputs[0].required = false;
        }))
        .await;
    assert_eq!(
        optional.outcome,
        RunOutcome::Succeeded,
        "an optional output may be absent"
    );
    assert_eq!(optional.outputs, Vec::new());
}

#[tokio::test]
async fn the_timeout_fires_at_the_configured_threshold_and_keeps_partial_output() {
    let repo = Repo::new();
    let mut plan = repo.fixture(&["sleep", "60000"]);
    plan.limits = ExecutionLimits::new(1, plan.limits.stream_bytes()).unwrap();
    let work = repo.provider.work_dir(&plan.run);
    let run = repo.run(plan).await;
    assert_eq!(run.outcome, RunOutcome::TimedOut);
    assert_eq!(run.exit_code, None);
    // The clock starts when the child starts, however long the operating system took to launch a
    // freshly built program, so measure from the moment the child says it began.
    // The provider's clock starts when the child is spawned, a little before the child writes its
    // marker, so the child's own lifetime is slightly under the limit: it must have run for most
    // of it (not been killed early) and been stopped soon after (not left running).
    let began = fs::metadata(work.join("started"))
        .unwrap()
        .modified()
        .unwrap();
    let elapsed = began.elapsed().unwrap();
    assert!(
        elapsed >= Duration::from_millis(500),
        "not before the limit: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "promptly after it: {elapsed:?}"
    );
}

fn read_counter(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

async fn assert_stopped(path: &Path) {
    let before = read_counter(path);
    assert!(before.is_some(), "the grandchild was running");
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(
        read_counter(path),
        before,
        "the grandchild is still writing: it was not terminated"
    );
}

#[tokio::test]
async fn a_timeout_kills_the_whole_process_tree() {
    let repo = Repo::new();
    let heartbeat = repo.dir.path().join("heartbeat-timeout");
    let mut plan = repo.custom(
        &repo.tool.clone(),
        &["spawn-child", &heartbeat.to_string_lossy(), "60000"],
        |s| s.outputs.clear(),
    );
    plan.limits = ExecutionLimits::new(2, plan.limits.stream_bytes()).unwrap();
    let run = repo.run(plan).await;
    assert_eq!(run.outcome, RunOutcome::TimedOut);
    assert_stopped(&heartbeat).await;
}

#[tokio::test]
async fn cancelling_a_running_child_ends_it_promptly_and_cleans_up_its_children() {
    let repo = Repo::new();
    let heartbeat = repo.dir.path().join("heartbeat-cancel");
    let plan = repo.custom(
        &repo.tool.clone(),
        &["spawn-child", &heartbeat.to_string_lossy(), "60000"],
        |s| s.outputs.clear(),
    );
    let token = CancelToken::new();
    let task = {
        let provider = repo.provider.clone();
        let token = token.clone();
        let plan = plan.clone();
        tokio::spawn(async move { provider.execute(plan, token).await })
    };
    // Wait until the grandchild is demonstrably running, then cancel. A freshly built program can
    // take a long time to launch the first time the operating system sees it.
    let waited = Instant::now();
    while !heartbeat.exists() && waited.elapsed() < Duration::from_secs(120) {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(heartbeat.exists(), "the grandchild started");
    let cancelled_at = Instant::now();
    token.cancel();
    let run = task.await.unwrap().unwrap();
    assert!(
        cancelled_at.elapsed() < Duration::from_secs(5),
        "cancellation is prompt"
    );
    run.check(&plan).unwrap();
    assert_eq!(run.outcome, RunOutcome::Cancelled);
    assert_eq!(run.exit_code, None);
    assert_stopped(&heartbeat).await;
}

#[tokio::test]
async fn a_child_left_behind_by_a_finished_run_is_cleaned_up() {
    let repo = Repo::new();
    let heartbeat = repo.dir.path().join("heartbeat-orphan");
    let plan = repo.custom(
        &repo.tool.clone(),
        &["spawn-and-exit", &heartbeat.to_string_lossy()],
        |s| s.outputs.clear(),
    );
    let run = repo.run(plan).await;
    assert_eq!(run.outcome, RunOutcome::Succeeded);
    assert_stopped(&heartbeat).await;
}

#[tokio::test]
async fn a_cancelled_token_prevents_the_child_from_starting() {
    let repo = Repo::new();
    let token = CancelToken::new();
    token.cancel();
    let run = repo
        .provider
        .execute(repo.fixture(&["sleep", "60000"]), token)
        .await
        .unwrap();
    assert_eq!(run.outcome, RunOutcome::Cancelled);
    assert_eq!(repo.provider.started(), 0, "nothing was started");
}

#[tokio::test]
async fn replaying_the_same_captures_gives_identical_declared_outputs() {
    let repo = Repo::new();
    let mut outputs = Vec::new();
    for run_id in [1_u128, 2, 3] {
        let mut plan = repo.fixture(&["echo-input", "inputs/data", "outputs/result"]);
        plan.run = ArtifactId::from_bits(run_id);
        outputs.push(repo.run(plan).await.outputs);
    }
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(outputs[1], outputs[2]);
}

#[tokio::test]
async fn work_directories_are_kept_per_run() {
    let repo = Repo::new();
    let mut a = repo.fixture(&["echo-input", "inputs/data", "outputs/result"]);
    a.run = ArtifactId::from_bits(10);
    let mut b = a.clone();
    b.run = ArtifactId::from_bits(11);
    repo.run(a.clone()).await;
    repo.run(b.clone()).await;
    let dir = |plan: &ExecutionPlan| -> PathBuf { repo.provider.work_dir(&plan.run) };
    assert_ne!(dir(&a), dir(&b));
    assert!(dir(&a).join("outputs/result").is_file());
    assert!(dir(&b).join("outputs/result").is_file());
}
