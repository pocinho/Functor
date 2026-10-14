//! A fake execution provider and the provider conformance suite (ADR-0012).
//!
//! [`run_provider_conformance`] is the behavioral contract every [`ExecutionProvider`] must pass:
//! outcomes for success, failure, missing program, missing output, timeout, truncation and
//! cancellation; refusals before any side effect; and deterministic replay. The real
//! `local-process` provider runs the same scenarios against fixture programs.
//! [`FakeProvider`] is the in-process implementation used to test the suite itself and the
//! orchestration built on the provider contract.
//!
//! The suite is runtime-agnostic: it returns futures and the calling test drives them.

// The scenarios are assertions: they panic on failure by design.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::missing_panics_doc)]

use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use functor_core::{
    CancelToken, ErrorKind, ExecutionGrant, ExecutionPlan, ExecutionProvider, FunctorError,
    InputPayload, OutputPayload, ProviderRun, Result, StreamCapture,
};
use functor_model::{
    ArtifactId, ArtifactType, ExecutionLimits, InputSpec, OutputSpec, ProgramProvenance,
    RunOutcome, Workflow, WorkflowSpec,
};

use crate::evidence::fake_digest;

/// The provider name the fake and the standard plan use.
pub const FAKE_PROVIDER: &str = "fake";

/// The behaviors the suite asks a provider to produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderScenario {
    /// Exit status 0 and the declared output `result`.
    Succeeds,
    /// A non-zero exit status.
    ExitsNonZero,
    /// The program cannot be started.
    ProgramMissing,
    /// Exit status 0 but the required output `result` is not produced.
    RequiredOutputMissing,
    /// Runs past the wall-clock timeout.
    TimesOut,
    /// Writes more to standard output than the stream limit.
    FloodsOutput,
    /// Runs until cancelled.
    WaitsForCancel,
}

impl ProviderScenario {
    /// The mode argument the fake provider's workflow carries.
    #[must_use]
    pub const fn mode(self) -> &'static str {
        match self {
            Self::Succeeds => "succeed",
            Self::ExitsNonZero => "exit-nonzero",
            Self::ProgramMissing => "missing-program",
            Self::RequiredOutputMissing => "no-output",
            Self::TimesOut => "timeout",
            Self::FloodsOutput => "flood",
            Self::WaitsForCancel => "wait-for-cancel",
        }
    }
}

/// The standard plan shape the suite relies on: a workflow with the required input `data`, the
/// required output `result`, `env_allow = ["LANG"]`, and a grant that permits `LANG` and the
/// default limits, with the effective limits equal to the defaults.
///
/// # Panics
/// Never for the constants used here.
#[must_use]
pub fn fake_plan(mode: &str) -> ExecutionPlan {
    let workflow = Workflow::new(WorkflowSpec {
        provider: FAKE_PROVIDER.to_owned(),
        program: "fake-tool".to_owned(),
        args: vec![mode.to_owned()],
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
    })
    .unwrap();
    ExecutionPlan {
        run: ArtifactId::from_bits(1),
        workflow,
        inputs: vec![InputPayload {
            name: "data".to_owned(),
            digest: fake_digest(b"hello"),
            bytes: b"hello".to_vec(),
        }],
        limits: ExecutionLimits::DEFAULT,
        grant: ExecutionGrant::new("fake-tool", ExecutionLimits::DEFAULT).with_env("LANG"),
    }
}

/// Deliberate defects the suite must detect.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Defect {
    SkipPreflight,
    OutputsOnFailure,
    SuccessOnNonZeroExit,
    IgnoreStreamLimit,
    IgnoreCancel,
    OutputDependsOnRunId,
}

/// An in-process provider whose behavior is chosen by the first workflow argument (see
/// [`ProviderScenario::mode`]). Its outputs depend only on the captured inputs.
#[derive(Debug, Default)]
pub struct FakeProvider {
    started: AtomicUsize,
    defect: Option<Defect>,
}

impl FakeProvider {
    /// A correct fake provider.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many executions got past the preflight and started.
    #[must_use]
    pub fn started(&self) -> usize {
        self.started.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub(crate) fn with_defect(defect: Defect) -> Self {
        Self {
            started: AtomicUsize::new(0),
            defect: Some(defect),
        }
    }

    fn has(&self, defect: Defect) -> bool {
        self.defect == Some(defect)
    }

    fn outputs(&self, plan: &ExecutionPlan) -> Vec<OutputPayload> {
        let mut inputs: Vec<&InputPayload> = plan.inputs.iter().collect();
        inputs.sort_by(|a, b| a.name.cmp(&b.name));
        let mut body = b"out:".to_vec();
        for input in inputs {
            body.extend_from_slice(&input.bytes);
        }
        if self.has(Defect::OutputDependsOnRunId) {
            body.extend_from_slice(plan.run.to_string().as_bytes());
        }
        plan.workflow
            .outputs()
            .iter()
            .map(|output| OutputPayload {
                name: output.name.clone(),
                bytes: body.clone(),
            })
            .collect()
    }
}

fn bare(outcome: RunOutcome, program: ProgramProvenance) -> ProviderRun {
    ProviderRun {
        outcome,
        exit_code: None,
        stdout: StreamCapture::default(),
        stderr: StreamCapture::default(),
        outputs: Vec::new(),
        diagnostics: Vec::new(),
        program,
    }
}

#[async_trait]
impl ExecutionProvider for FakeProvider {
    fn name(&self) -> &'static str {
        FAKE_PROVIDER
    }

    fn version(&self) -> &'static str {
        "fake-1"
    }

    async fn execute(&self, plan: ExecutionPlan, cancel: CancelToken) -> Result<ProviderRun> {
        if !self.has(Defect::SkipPreflight) {
            plan.check(self.name())?;
        }
        self.started.fetch_add(1, Ordering::SeqCst);
        let program = ProgramProvenance {
            path: plan.workflow.program().to_owned(),
            digest: Some(fake_digest(plan.workflow.program().as_bytes())),
        };
        let mode = plan
            .workflow
            .args()
            .first()
            .map_or("succeed", String::as_str);
        if cancel.is_cancelled() && !self.has(Defect::IgnoreCancel) {
            return Ok(bare(RunOutcome::Cancelled, program));
        }
        let run = match mode {
            "succeed" => ProviderRun {
                exit_code: Some(0),
                stdout: StreamCapture {
                    bytes: b"ok\n".to_vec(),
                    truncated: false,
                },
                outputs: self.outputs(&plan),
                ..bare(RunOutcome::Succeeded, program)
            },
            "exit-nonzero" => {
                let mut run = ProviderRun {
                    exit_code: Some(3),
                    stderr: StreamCapture {
                        bytes: b"boom\n".to_vec(),
                        truncated: false,
                    },
                    ..bare(RunOutcome::Failed, program)
                };
                if self.has(Defect::SuccessOnNonZeroExit) {
                    run.outcome = RunOutcome::Succeeded;
                    run.outputs = self.outputs(&plan);
                }
                if self.has(Defect::OutputsOnFailure) {
                    run.outputs = self.outputs(&plan);
                }
                run
            }
            "missing-program" => ProviderRun {
                diagnostics: vec![format!(
                    "program `{}` could not be started",
                    plan.workflow.program()
                )],
                ..bare(
                    RunOutcome::StartFailed,
                    ProgramProvenance {
                        digest: None,
                        ..program
                    },
                )
            },
            "no-output" => ProviderRun {
                exit_code: Some(0),
                diagnostics: vec!["required output `result` is missing".to_owned()],
                ..bare(RunOutcome::Failed, program)
            },
            "timeout" | "wait-for-cancel" => bare(RunOutcome::TimedOut, program),
            "flood" => {
                let limit = usize::try_from(plan.limits.stream_bytes()).unwrap_or(usize::MAX);
                let ignore = self.has(Defect::IgnoreStreamLimit);
                ProviderRun {
                    exit_code: Some(0),
                    stdout: StreamCapture {
                        bytes: vec![b'x'; if ignore { limit + 1 } else { limit }],
                        truncated: !ignore,
                    },
                    outputs: self.outputs(&plan),
                    ..bare(RunOutcome::Succeeded, program)
                }
            }
            other => {
                return Err(FunctorError::unsupported(format!("fake mode `{other}`")));
            }
        };
        Ok(run)
    }
}

/// What a provider must offer to be tested.
pub trait ProviderUnderTest {
    /// The provider.
    fn provider(&self) -> &dyn ExecutionProvider;

    /// A plan in the standard shape (see [`fake_plan`]) that makes the provider produce
    /// `scenario`. Real providers return plans for fixture programs.
    fn plan(&self, scenario: ProviderScenario) -> ExecutionPlan;

    /// How many executions have started so far; refusals must not increase it.
    fn started(&self) -> usize;
}

impl ProviderUnderTest for FakeProvider {
    fn provider(&self) -> &dyn ExecutionProvider {
        self
    }

    fn plan(&self, scenario: ProviderScenario) -> ExecutionPlan {
        fake_plan(scenario.mode())
    }

    fn started(&self) -> usize {
        FakeProvider::started(self)
    }
}

fn rebuilt(plan: &ExecutionPlan, change: impl FnOnce(&mut WorkflowSpec)) -> ExecutionPlan {
    let mut spec = plan.workflow.spec().clone();
    change(&mut spec);
    ExecutionPlan {
        workflow: Workflow::new(spec).unwrap(),
        ..plan.clone()
    }
}

async fn execute<P: ProviderUnderTest>(subject: &P, plan: ExecutionPlan) -> Result<ProviderRun> {
    subject.provider().execute(plan, CancelToken::new()).await
}

async fn run_ok<P: ProviderUnderTest>(subject: &P, plan: ExecutionPlan) -> ProviderRun {
    let run = execute(subject, plan.clone())
        .await
        .expect("the provider must report a child outcome as data, not as an error");
    run.check(&plan).unwrap();
    run
}

fn expect_err<T: Debug>(result: Result<T>, kind: ErrorKind, context: &str) {
    match result {
        Ok(value) => panic!("{context}: expected a {kind:?} error, got Ok({value:?})"),
        Err(error) => assert_eq!(error.kind(), kind, "{context}: got `{error}`"),
    }
}

/// Runs every provider scenario against `subject`.
pub async fn run_provider_conformance<P: ProviderUnderTest>(subject: &P) {
    success_produces_declared_outputs(subject).await;
    failures_are_outcomes_not_errors(subject).await;
    output_is_bounded_and_truncation_is_reported(subject).await;
    cancellation_is_an_outcome(subject).await;
    refusals_happen_before_any_side_effect(subject).await;
    replay_is_deterministic(subject).await;
}

/// A success has exit status 0, the declared outputs and program provenance.
pub async fn success_produces_declared_outputs<P: ProviderUnderTest>(subject: &P) {
    let run = run_ok(subject, subject.plan(ProviderScenario::Succeeds)).await;
    assert_eq!(run.outcome, RunOutcome::Succeeded);
    assert_eq!(run.exit_code, Some(0));
    let names: Vec<&str> = run.outputs.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(names, ["result"]);
    assert!(!run.program.path.is_empty(), "the program is recorded");
}

/// A bad child result is an outcome with the right shape: no outputs, a status or diagnostics.
pub async fn failures_are_outcomes_not_errors<P: ProviderUnderTest>(subject: &P) {
    let failed = run_ok(subject, subject.plan(ProviderScenario::ExitsNonZero)).await;
    assert_eq!(failed.outcome, RunOutcome::Failed);
    assert!(matches!(failed.exit_code, Some(code) if code != 0));
    assert!(failed.outputs.is_empty(), "a failed run publishes nothing");

    let missing = run_ok(subject, subject.plan(ProviderScenario::ProgramMissing)).await;
    assert_eq!(missing.outcome, RunOutcome::StartFailed);
    assert_eq!(missing.exit_code, None);
    assert!(!missing.diagnostics.is_empty(), "a start failure says why");

    let no_output = run_ok(
        subject,
        subject.plan(ProviderScenario::RequiredOutputMissing),
    )
    .await;
    assert_eq!(no_output.outcome, RunOutcome::Failed);
    assert!(
        !no_output.diagnostics.is_empty(),
        "a missing output is explained"
    );
    assert_eq!(no_output.outputs, Vec::new());

    let timed_out = run_ok(subject, subject.plan(ProviderScenario::TimesOut)).await;
    assert_eq!(timed_out.outcome, RunOutcome::TimedOut);
    assert_eq!(timed_out.exit_code, None);
}

/// Captured output never exceeds the stream limit and truncation is indicated.
pub async fn output_is_bounded_and_truncation_is_reported<P: ProviderUnderTest>(subject: &P) {
    let mut plan = subject.plan(ProviderScenario::FloodsOutput);
    plan.limits = ExecutionLimits::new(plan.limits.timeout_seconds(), 16).unwrap();
    plan.grant = ExecutionGrant::new(plan.grant.program(), ExecutionLimits::MAX).with_env("LANG");
    let run = run_ok(subject, plan).await;
    assert_eq!(
        run.stdout.bytes.len(),
        16,
        "stdout is cut exactly at the limit"
    );
    assert!(run.stdout.truncated, "truncation is indicated");
    assert!(!run.stderr.truncated);
}

/// A cancelled run ends as `Cancelled`, with no exit status and no outputs.
pub async fn cancellation_is_an_outcome<P: ProviderUnderTest>(subject: &P) {
    let plan = subject.plan(ProviderScenario::WaitsForCancel);
    let token = CancelToken::new();
    token.cancel();
    let run = subject
        .provider()
        .execute(plan.clone(), token)
        .await
        .expect("cancellation is an outcome");
    run.check(&plan).unwrap();
    assert_eq!(run.outcome, RunOutcome::Cancelled);
    assert_eq!(run.exit_code, None);
    assert_eq!(run.outputs, Vec::new());
}

/// Unsupported provider, ungranted environment, limits above the grant and unbound inputs are
/// refused with the documented error kinds, and nothing is started.
pub async fn refusals_happen_before_any_side_effect<P: ProviderUnderTest>(subject: &P) {
    let base = subject.plan(ProviderScenario::Succeeds);
    let before = subject.started();

    expect_err(
        execute(
            subject,
            rebuilt(&base, |s| "other-provider".clone_into(&mut s.provider)),
        )
        .await,
        ErrorKind::Unsupported,
        "another provider",
    );

    let mut no_env = base.clone();
    no_env.grant = ExecutionGrant::new(base.grant.program(), base.grant.limits());
    expect_err(
        execute(subject, no_env).await,
        ErrorKind::PermissionDenied,
        "environment variable not granted",
    );

    let mut too_much = base.clone();
    too_much.limits = ExecutionLimits::MAX;
    expect_err(
        execute(subject, too_much).await,
        ErrorKind::PermissionDenied,
        "limits above the grant",
    );

    let mut unbound = base.clone();
    unbound.inputs.clear();
    expect_err(
        execute(subject, unbound).await,
        ErrorKind::InvalidInput,
        "required input not bound",
    );

    let mut stray = base.clone();
    stray.inputs.push(InputPayload {
        name: "stray".to_owned(),
        digest: fake_digest(b"x"),
        bytes: b"x".to_vec(),
    });
    expect_err(
        execute(subject, stray).await,
        ErrorKind::InvalidInput,
        "undeclared input",
    );

    assert_eq!(
        subject.started(),
        before,
        "a refused plan must not start anything"
    );
}

/// The same captured inputs give the same declared outputs, whatever the run identity.
pub async fn replay_is_deterministic<P: ProviderUnderTest>(subject: &P) {
    let first_plan = subject.plan(ProviderScenario::Succeeds);
    let mut second_plan = first_plan.clone();
    second_plan.run = ArtifactId::from_bits(777);
    let first = run_ok(subject, first_plan).await;
    let second = run_ok(subject, second_plan).await;
    assert_eq!(
        first.outputs, second.outputs,
        "declared outputs replay exactly"
    );
}

#[cfg(test)]
mod tests {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::*;
    use crate::block_on::block_on;

    #[test]
    fn a_correct_provider_passes_the_suite() {
        let provider = FakeProvider::new();
        block_on(run_provider_conformance(&provider));
    }

    #[test]
    fn the_suite_rejects_every_deliberately_broken_provider() {
        for defect in [
            Defect::SkipPreflight,
            Defect::OutputsOnFailure,
            Defect::SuccessOnNonZeroExit,
            Defect::IgnoreStreamLimit,
            Defect::IgnoreCancel,
            Defect::OutputDependsOnRunId,
        ] {
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                let provider = FakeProvider::with_defect(defect);
                block_on(run_provider_conformance(&provider));
            }));
            assert!(outcome.is_err(), "the suite did not detect {defect:?}");
        }
    }

    #[test]
    fn the_fake_chooses_its_behavior_from_the_workflow() {
        let provider = FakeProvider::new();
        let plan = fake_plan("exit-nonzero");
        let run = block_on(provider.execute(plan, CancelToken::new())).unwrap();
        assert_eq!(run.outcome, RunOutcome::Failed);
        assert_eq!(provider.started(), 1);
        let unknown = block_on(provider.execute(fake_plan("nonsense"), CancelToken::new()));
        assert_eq!(unknown.unwrap_err().kind(), ErrorKind::Unsupported);
    }

    #[test]
    fn the_fake_is_dyn_compatible_and_named() {
        let provider: Box<dyn ExecutionProvider> = Box::new(FakeProvider::new());
        assert_eq!(provider.name(), FAKE_PROVIDER);
        assert_eq!(provider.version(), "fake-1");
    }
}
