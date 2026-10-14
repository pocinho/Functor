//! The execution provider contract (ADR-0012).
//!
//! A provider turns a prepared [`ExecutionPlan`] into a [`ProviderRun`]. It never opens the
//! repository, persists anything, validates or publishes events: capture, durable intent,
//! publication and events belong to the orchestrator (ADR-0013). A bad outcome of the child is
//! data in the result; `Err` is reserved for refusals and infrastructure faults.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use functor_model::{
    ArtifactId, ContentDigest, ExecutionLimits, ProgramProvenance, RunOutcome, Workflow, bounds,
};

use crate::error::{FunctorError, Result};

/// Cooperative cancellation: clones share one flag. Providers poll [`is_cancelled`] while the
/// child runs and must stop the whole process tree promptly once it is set.
///
/// [`is_cancelled`]: CancelToken::is_cancelled
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token that is not cancelled.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation. Idempotent.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether cancellation was requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Explicit permission to run one program with at most these limits, for one invocation.
///
/// Grants are supplied by the caller and never read from repository content, so untrusted
/// artifacts cannot grant themselves anything (ADR-0012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionGrant {
    program: String,
    limits: ExecutionLimits,
    env: BTreeSet<String>,
}

impl ExecutionGrant {
    /// Grants running `program`, the resolved path the user approved, within `limits`.
    #[must_use]
    pub fn new(program: impl Into<String>, limits: ExecutionLimits) -> Self {
        Self {
            program: program.into(),
            limits,
            env: BTreeSet::new(),
        }
    }

    /// Also permits passing the environment variable `name` through to the child.
    #[must_use]
    pub fn with_env(mut self, name: impl Into<String>) -> Self {
        self.env.insert(name.into());
        self
    }

    /// The approved program.
    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The most the run may use.
    #[must_use]
    pub fn limits(&self) -> ExecutionLimits {
        self.limits
    }

    /// Whether the variable may be passed through.
    #[must_use]
    pub fn permits_env(&self, name: &str) -> bool {
        self.env.contains(name)
    }
}

/// One captured input handed to the provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputPayload {
    /// The workflow input name.
    pub name: String,
    /// The digest of `bytes` as captured.
    pub digest: ContentDigest,
    /// The captured bytes.
    pub bytes: Vec<u8>,
}

/// Everything a provider needs to run once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    /// The `run` artifact this execution belongs to.
    pub run: ArtifactId,
    /// The captured, validated workflow.
    pub workflow: Workflow,
    /// The captured inputs.
    pub inputs: Vec<InputPayload>,
    /// The effective limits for this run.
    pub limits: ExecutionLimits,
    /// The permission to execute.
    pub grant: ExecutionGrant,
}

impl ExecutionPlan {
    /// The checks every provider performs before doing anything with side effects.
    ///
    /// # Errors
    /// - `Unsupported` when the workflow names another provider;
    /// - `PermissionDenied` when the limits exceed the grant or the workflow allows an
    ///   environment variable the grant does not;
    /// - `InvalidInput` for an undeclared, duplicate, missing-required or oversized input.
    pub fn check(&self, provider: &str) -> Result<()> {
        if self.workflow.provider() != provider {
            return Err(FunctorError::unsupported(format!(
                "provider `{}`",
                self.workflow.provider()
            )));
        }
        let granted = self.grant.limits();
        if self.limits.timeout_seconds() > granted.timeout_seconds()
            || self.limits.stream_bytes() > granted.stream_bytes()
        {
            return Err(FunctorError::permission_denied(
                "execute",
                "the requested limits exceed the grant",
            ));
        }
        if let Some(name) = self
            .workflow
            .env_allow()
            .iter()
            .find(|name| !self.grant.permits_env(name))
        {
            return Err(FunctorError::permission_denied(
                "execute",
                format!("environment variable `{name}` is not granted"),
            ));
        }
        let mut seen = BTreeSet::new();
        let mut total: u64 = 0;
        for input in &self.inputs {
            if !seen.insert(input.name.as_str()) {
                return Err(FunctorError::invalid_input(
                    "inputs",
                    format!("input `{}` is bound twice", input.name),
                ));
            }
            if !self.workflow.inputs().iter().any(|i| i.name == input.name) {
                return Err(FunctorError::invalid_input(
                    "inputs",
                    format!("the workflow declares no input `{}`", input.name),
                ));
            }
            let size = input.bytes.len() as u64;
            if size > bounds::MAX_ITEM_BYTES {
                return Err(FunctorError::invalid_input(
                    "inputs",
                    format!(
                        "input `{}` is larger than {} bytes",
                        input.name,
                        bounds::MAX_ITEM_BYTES
                    ),
                ));
            }
            total = total.saturating_add(size);
        }
        if total > bounds::MAX_TOTAL_BYTES {
            return Err(FunctorError::invalid_input(
                "inputs",
                format!(
                    "inputs are larger than {} bytes in total",
                    bounds::MAX_TOTAL_BYTES
                ),
            ));
        }
        if let Some(missing) = self
            .workflow
            .inputs()
            .iter()
            .find(|i| i.required && !seen.contains(i.name.as_str()))
        {
            return Err(FunctorError::invalid_input(
                "inputs",
                format!("required input `{}` is not bound", missing.name),
            ));
        }
        Ok(())
    }
}

/// A bounded capture of one output stream.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StreamCapture {
    /// At most the stream limit of bytes.
    pub bytes: Vec<u8>,
    /// Whether more was written than fit.
    pub truncated: bool,
}

/// One declared output read from the work directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputPayload {
    /// The declared output name.
    pub name: String,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// What a provider reports about one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRun {
    /// How the run ended. Never `Interrupted`: only recovery writes that.
    pub outcome: RunOutcome,
    /// The child's exit status, for `Succeeded` and `Failed` only.
    pub exit_code: Option<i32>,
    /// Captured standard output.
    pub stdout: StreamCapture,
    /// Captured standard error.
    pub stderr: StreamCapture,
    /// Declared outputs, present only for `Succeeded`.
    pub outputs: Vec<OutputPayload>,
    /// Reasons for a failure, for example a missing required output or a program not found.
    pub diagnostics: Vec<String>,
    /// The program that was resolved and, when readable, its digest.
    pub program: ProgramProvenance,
}

impl ProviderRun {
    /// Checks that the report honours the provider contract for `plan`. The orchestrator calls
    /// this before publishing anything, and the conformance suite calls it on every scenario.
    ///
    /// # Errors
    /// `Internal`, because a violation is a provider bug: an `Interrupted` outcome; a stream over
    /// the limit; an undeclared, duplicate or oversized output; outputs on a run that did not
    /// succeed; a success without exit status 0, with diagnostics or without a required output;
    /// or a `Failed` run with exit status 0 and no diagnostic.
    pub fn check(&self, plan: &ExecutionPlan) -> Result<()> {
        let broken = |reason: String| {
            Err(FunctorError::Internal(format!(
                "provider broke its contract: {reason}"
            )))
        };
        if self.outcome == RunOutcome::Interrupted {
            return broken("only recovery may report `interrupted`".to_owned());
        }
        let limit = plan.limits.stream_bytes();
        for (name, stream) in [("stdout", &self.stdout), ("stderr", &self.stderr)] {
            if stream.bytes.len() as u64 > limit {
                return broken(format!("{name} exceeds the {limit} byte limit"));
            }
        }
        let mut names = BTreeSet::new();
        let mut total: u64 = 0;
        for output in &self.outputs {
            if !plan
                .workflow
                .outputs()
                .iter()
                .any(|o| o.name == output.name)
            {
                return broken(format!("output `{}` is not declared", output.name));
            }
            if !names.insert(output.name.as_str()) {
                return broken(format!("output `{}` is reported twice", output.name));
            }
            let size = output.bytes.len() as u64;
            if size > bounds::MAX_ITEM_BYTES {
                return broken(format!("output `{}` exceeds the size limit", output.name));
            }
            total = total.saturating_add(size);
        }
        if total > bounds::MAX_TOTAL_BYTES {
            return broken("outputs exceed the total size limit".to_owned());
        }
        if !self.outcome.is_success() && !self.outputs.is_empty() {
            return broken("outputs on a run that did not succeed".to_owned());
        }
        if self.outcome.is_success() {
            if self.exit_code != Some(0) {
                return broken("a success without exit status 0".to_owned());
            }
            if !self.diagnostics.is_empty() {
                return broken("a success with failure diagnostics".to_owned());
            }
            let produced: Vec<String> = self.outputs.iter().map(|o| o.name.clone()).collect();
            if let Some(missing) = plan.workflow.missing_outputs(&produced).first() {
                return broken(format!("a success without required output `{missing}`"));
            }
        }
        if self.outcome == RunOutcome::Failed
            && self.exit_code == Some(0)
            && self.diagnostics.is_empty()
        {
            return broken("a failure with exit status 0 and no diagnostic".to_owned());
        }
        Ok(())
    }
}

/// Runs a prepared plan with one execution mechanism (ADR-0012).
///
/// Implementations must: refuse with `Unsupported`, `PermissionDenied` or `InvalidInput` (see
/// [`ExecutionPlan::check`]) **before** any side effect; report a child failure, timeout,
/// cancellation or missing program as an outcome, not an error; stop the whole process tree on
/// timeout and when `cancel` is set; leave nothing running when the future returns; and honour the
/// limits in the plan.
#[async_trait]
pub trait ExecutionProvider: Send + Sync {
    /// The provider name a workflow selects, for example `local-process`.
    fn name(&self) -> &str;

    /// The provider version, recorded as provenance.
    fn version(&self) -> &str;

    /// Executes the plan once.
    async fn execute(&self, plan: ExecutionPlan, cancel: CancelToken) -> Result<ProviderRun>;
}

#[cfg(test)]
mod tests {
    use functor_model::ArtifactType;
    use functor_model::{InputSpec, LOCAL_PROCESS_PROVIDER, OutputSpec, WorkflowSpec};

    use super::*;
    use crate::error::ErrorKind;

    fn workflow() -> Workflow {
        Workflow::new(WorkflowSpec {
            provider: LOCAL_PROCESS_PROVIDER.to_owned(),
            program: "tool".to_owned(),
            inputs: vec![InputSpec {
                name: "data".to_owned(),
                required: true,
            }],
            outputs: vec![OutputSpec {
                name: "out".to_owned(),
                artifact_type: ArtifactType::document(),
                format: "text".to_owned(),
                required: true,
            }],
            env_allow: vec!["LANG".to_owned()],
            ..WorkflowSpec::default()
        })
        .unwrap()
    }

    fn digest() -> ContentDigest {
        ContentDigest::from_bytes([1; 32])
    }

    fn plan() -> ExecutionPlan {
        ExecutionPlan {
            run: ArtifactId::from_bits(1),
            workflow: workflow(),
            inputs: vec![InputPayload {
                name: "data".to_owned(),
                digest: digest(),
                bytes: b"x".to_vec(),
            }],
            limits: ExecutionLimits::DEFAULT,
            grant: ExecutionGrant::new("tool", ExecutionLimits::DEFAULT).with_env("LANG"),
        }
    }

    fn run() -> ProviderRun {
        ProviderRun {
            outcome: RunOutcome::Succeeded,
            exit_code: Some(0),
            stdout: StreamCapture::default(),
            stderr: StreamCapture::default(),
            outputs: vec![OutputPayload {
                name: "out".to_owned(),
                bytes: b"y".to_vec(),
            }],
            diagnostics: vec![],
            program: ProgramProvenance {
                path: "tool".to_owned(),
                digest: None,
            },
        }
    }

    #[test]
    fn cancel_tokens_share_one_flag() {
        let token = CancelToken::new();
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        token.cancel();
        assert!(clone.is_cancelled());
    }

    #[test]
    fn a_valid_plan_passes_the_preflight() {
        plan().check(LOCAL_PROCESS_PROVIDER).unwrap();
    }

    #[test]
    fn the_preflight_refuses_with_the_documented_error_kinds() {
        let kind = |change: fn(&mut ExecutionPlan), provider: &str| {
            let mut p = plan();
            change(&mut p);
            p.check(provider).unwrap_err().kind()
        };
        assert_eq!(kind(|_| {}, "other"), ErrorKind::Unsupported);
        assert_eq!(
            kind(
                |p| p.grant =
                    ExecutionGrant::new("tool", ExecutionLimits::new(10, 1 << 20).unwrap())
                        .with_env("LANG"),
                LOCAL_PROCESS_PROVIDER
            ),
            ErrorKind::PermissionDenied,
            "timeout above the grant"
        );
        assert_eq!(
            kind(
                |p| p.grant = ExecutionGrant::new("tool", ExecutionLimits::new(60, 10).unwrap())
                    .with_env("LANG"),
                LOCAL_PROCESS_PROVIDER
            ),
            ErrorKind::PermissionDenied,
            "stream limit above the grant"
        );
        assert_eq!(
            kind(
                |p| p.grant = ExecutionGrant::new("tool", ExecutionLimits::DEFAULT),
                LOCAL_PROCESS_PROVIDER
            ),
            ErrorKind::PermissionDenied,
            "environment variable not granted"
        );
        assert_eq!(
            kind(|p| p.inputs.clear(), LOCAL_PROCESS_PROVIDER),
            ErrorKind::InvalidInput,
            "required input missing"
        );
        assert_eq!(
            kind(
                |p| p.inputs[0].name = "other".to_owned(),
                LOCAL_PROCESS_PROVIDER
            ),
            ErrorKind::InvalidInput,
            "undeclared input"
        );
        assert_eq!(
            kind(
                |p| p.inputs.push(p.inputs[0].clone()),
                LOCAL_PROCESS_PROVIDER
            ),
            ErrorKind::InvalidInput,
            "duplicate input"
        );
    }

    #[test]
    fn input_size_limits_hold_at_the_boundary() {
        let mut at_limit = plan();
        at_limit.inputs[0].bytes = vec![0; usize::try_from(bounds::MAX_ITEM_BYTES).unwrap()];
        at_limit.check(LOCAL_PROCESS_PROVIDER).unwrap();
        at_limit.inputs[0].bytes.push(0);
        assert_eq!(
            at_limit.check(LOCAL_PROCESS_PROVIDER).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
    }

    #[test]
    fn the_total_input_size_holds_at_the_boundary() {
        let item = usize::try_from(bounds::MAX_ITEM_BYTES).unwrap();
        let mut spec = plan().workflow.spec().clone();
        spec.inputs = (0..5)
            .map(|n| InputSpec {
                name: format!("i{n}"),
                required: false,
            })
            .collect();
        let mut at_limit = plan();
        at_limit.workflow = Workflow::new(spec).unwrap();
        at_limit.inputs = (0..4)
            .map(|n| InputPayload {
                name: format!("i{n}"),
                digest: digest(),
                bytes: vec![0; item],
            })
            .collect();
        at_limit.check(LOCAL_PROCESS_PROVIDER).unwrap();

        let mut over = at_limit.clone();
        over.inputs.push(InputPayload {
            name: "i4".to_owned(),
            digest: digest(),
            bytes: vec![0],
        });
        assert_eq!(
            over.check(LOCAL_PROCESS_PROVIDER).unwrap_err().kind(),
            ErrorKind::InvalidInput,
            "one byte over the total"
        );
    }

    #[test]
    fn a_well_behaved_run_passes_the_contract_check() {
        run().check(&plan()).unwrap();
    }

    #[test]
    fn broken_provider_reports_are_rejected() {
        let rejected = |change: fn(&mut ProviderRun)| {
            let mut r = run();
            change(&mut r);
            let error = r.check(&plan()).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Internal);
        };
        rejected(|r| r.outcome = RunOutcome::Interrupted);
        rejected(|r| r.stdout.bytes = vec![0; 1024 * 1024 + 1]);
        rejected(|r| r.stderr.bytes = vec![0; 1024 * 1024 + 1]);
        rejected(|r| r.outputs[0].name = "undeclared".to_owned());
        rejected(|r| r.outputs.push(r.outputs[0].clone()));
        rejected(|r| r.outputs.clear());
        rejected(|r| r.exit_code = Some(1));
        rejected(|r| r.exit_code = None);
        rejected(|r| r.diagnostics = vec!["x".to_owned()]);
        rejected(|r| {
            r.outcome = RunOutcome::Failed;
            r.exit_code = Some(1);
        });
        rejected(|r| {
            r.outcome = RunOutcome::Failed;
            r.exit_code = Some(0);
            r.outputs.clear();
        });
    }

    #[test]
    fn failures_are_valid_reports_without_outputs() {
        let mut failed = run();
        failed.outcome = RunOutcome::Failed;
        failed.exit_code = Some(3);
        failed.outputs.clear();
        failed.check(&plan()).unwrap();

        let mut timed_out = run();
        timed_out.outcome = RunOutcome::TimedOut;
        timed_out.exit_code = None;
        timed_out.outputs.clear();
        timed_out.stdout = StreamCapture {
            bytes: vec![0; 1024 * 1024],
            truncated: true,
        };
        timed_out.check(&plan()).unwrap();
    }

    #[test]
    fn providers_are_dyn_compatible() {
        struct Nop;
        #[async_trait]
        impl ExecutionProvider for Nop {
            fn name(&self) -> &'static str {
                "nop"
            }
            fn version(&self) -> &'static str {
                "0"
            }
            async fn execute(&self, _: ExecutionPlan, _: CancelToken) -> Result<ProviderRun> {
                Err(FunctorError::unsupported("nothing"))
            }
        }
        let provider: Box<dyn ExecutionProvider> = Box::new(Nop);
        assert_eq!(provider.name(), "nop");
    }
}
