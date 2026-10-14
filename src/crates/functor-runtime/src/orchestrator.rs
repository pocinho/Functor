//! The run orchestrator: the single owner of the execution workflow and its events (ADR-0013,
//! ADR-0015).
//!
//! It wires capture, authorization, durable intent, execution, publication of outputs and the
//! result, relationships, optional output validation and events, using only core traits: the
//! evidence and report recorders, an execution provider, an optional validation engine and an
//! optional event publisher. It never opens a store itself; composition roots wire the concrete
//! pieces. In order:
//!
//! 1. capture the workflow, parse the captured bytes, check bindings, authorize (`preflight`);
//! 2. capture the inputs, record the run intent and take the run's lease;
//! 3. publish `run.started`, then run the provider;
//! 4. publish the outputs and the result (the commit point) and its relationships;
//! 5. publish `run.completed`, remove the staging of a successful run, optionally validate and
//!    persist a report for each output.
//!
//! A failed mandatory write is returned as an error and never followed by a success event. Events
//! are informational: a failed append is a warning in the report and never changes the outcome.
//! Evidence is never rerun: a repeated request id returns the existing run without starting a
//! child.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use functor_core::{
    ArtifactRepository, CancelToken, Clock, ErrorKind, EventPublisher, EvidenceRecorder,
    ExecutionGrant, ExecutionPlan, ExecutionProvider, FunctorError, InputPayload, PublishedOutput,
    ReportRecorder, ReportSubject, Result, RunFinish, ValidationEngine, ValidationTarget,
};
use functor_model::{
    ArtifactId, ArtifactRef, CaptureRole, ContentDigest, EventData, EventKind, EventSubject,
    NewEvent, RunIntent, RunOutcome, RunRecord, RunRequest, Truncation, ValidationReport,
};
use sha2::{Digest, Sha256};

use crate::workflow::parse_workflow;

fn digest_of(bytes: &[u8]) -> ContentDigest {
    ContentDigest::from_bytes(Sha256::digest(bytes).into())
}

/// What an orchestrator needs to validate and persist a report for each output of a run.
#[derive(Clone, Copy)]
pub struct OutputChecks<'a> {
    /// Validates one output artifact at a time (structural rules, ADR-0015).
    pub engine: &'a dyn ValidationEngine,
    /// Reads the output artifacts to identify the state that was validated.
    pub artifacts: &'a dyn ArtifactRepository,
    /// The rule-set identity stored with each report, for example `V0001-V0023`.
    pub rules: &'a str,
}

impl fmt::Debug for OutputChecks<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputChecks")
            .field("rules", &self.rules)
            .finish_non_exhaustive()
    }
}

/// The validation of one output of a completed run.
#[derive(Debug)]
pub struct OutputValidation {
    /// The output artifact that was validated.
    pub output: ArtifactId,
    /// The validation report. A `Failed` outcome never changes the outcome of the run.
    pub report: ValidationReport,
    /// The report artifact, or why it could not be saved (the report above is still valid).
    pub saved: Result<ArtifactId>,
}

/// What a finished (or repeated) run request produced.
#[derive(Debug)]
pub struct RunReport {
    /// The run and, once complete, its result.
    pub record: RunRecord,
    /// `false` when the request id had already been recorded: nothing ran and nothing was
    /// written or published by this call.
    pub created: bool,
    /// One entry per validated output, when output validation was requested.
    pub validations: Vec<OutputValidation>,
    /// Problems that did not change the outcome: events that could not be appended, staging that
    /// could not be removed, an output that could not be validated.
    pub warnings: Vec<String>,
}

/// A run request that failed with an error, with what is known about the run.
///
/// The run id is present once an intent exists; the run may then be incomplete (recover it), or
/// already complete when only its relationships could not be written.
#[derive(Debug)]
pub struct RunFailure {
    /// The run, if its intent was recorded.
    pub run: Option<ArtifactId>,
    /// What went wrong.
    pub error: FunctorError,
    /// Warnings collected before the failure.
    pub warnings: Vec<String>,
}

impl RunFailure {
    /// The kind of the underlying error, for projections that map kinds to exit codes.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        self.error.kind()
    }
}

impl fmt::Display for RunFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.run {
            Some(run) => write!(f, "{} (run {run})", self.error),
            None => self.error.fmt(f),
        }
    }
}

impl std::error::Error for RunFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// What recovering interrupted runs did.
#[derive(Debug)]
pub struct RecoveryReport {
    /// The runs that were completed as `interrupted`.
    pub runs: Vec<RunRecord>,
    /// Events that could not be appended.
    pub warnings: Vec<String>,
}

/// A saved validation report.
#[derive(Debug)]
pub struct SavedReport {
    /// The report artifact.
    pub id: ArtifactId,
    /// Events that could not be appended.
    pub warnings: Vec<String>,
}

/// Runs workflows and records their evidence. See the module documentation.
pub struct RunOrchestrator {
    recorder: Arc<dyn EvidenceRecorder>,
    reports: Arc<dyn ReportRecorder>,
    provider: Arc<dyn ExecutionProvider>,
    events: Option<Arc<dyn EventPublisher>>,
    clock: Arc<dyn Clock>,
    platform: String,
}

impl fmt::Debug for RunOrchestrator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunOrchestrator")
            .field("provider", &self.provider.name())
            .finish_non_exhaustive()
    }
}

/// Where a failure happened: the run id once known, and the warnings so far.
#[derive(Default)]
struct Progress {
    run: Option<ArtifactId>,
    warnings: Vec<String>,
}

impl RunOrchestrator {
    /// An orchestrator over the given collaborators, without events.
    #[must_use]
    pub fn new(
        recorder: Arc<dyn EvidenceRecorder>,
        reports: Arc<dyn ReportRecorder>,
        provider: Arc<dyn ExecutionProvider>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            recorder,
            reports,
            provider,
            events: None,
            clock,
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        }
    }

    /// Publishes `run.started`, `run.completed` and `validation.recorded` through `events`.
    #[must_use]
    pub fn with_events(mut self, events: Arc<dyn EventPublisher>) -> Self {
        self.events = Some(events);
        self
    }

    /// Publishes one event; a failure is a warning, never an error (ADR-0003).
    async fn emit(&self, event: NewEvent, warnings: &mut Vec<String>) {
        if let Some(publisher) = &self.events
            && let Err(error) = publisher.publish(event).await
        {
            warnings.push(format!("event not recorded: {error}"));
        }
    }

    /// Executes the workflow named by `request` once, with explicit permission.
    ///
    /// Without a configured engine in `checks`, a request that asks for output validation is
    /// refused (`InvalidInput`) before anything is written.
    ///
    /// # Errors
    /// A [`RunFailure`] carrying the run id when an intent exists. A child that fails, times out
    /// or is cancelled is **not** an error: it is the outcome in the returned record.
    pub async fn execute(
        &self,
        request: &RunRequest,
        grant: ExecutionGrant,
        cancel: CancelToken,
        checks: Option<OutputChecks<'_>>,
    ) -> std::result::Result<RunReport, Box<RunFailure>> {
        let mut progress = Progress::default();
        match self
            .flow(request, grant, cancel, checks, &mut progress)
            .await
        {
            Ok(report) => Ok(report),
            Err(error) => Err(Box::new(RunFailure {
                run: progress.run,
                error,
                warnings: progress.warnings,
            })),
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn flow(
        &self,
        request: &RunRequest,
        grant: ExecutionGrant,
        cancel: CancelToken,
        checks: Option<OutputChecks<'_>>,
        progress: &mut Progress,
    ) -> Result<RunReport> {
        if request.validates_outputs() && checks.is_none() {
            return Err(FunctorError::invalid_input(
                "validate outputs",
                "output validation needs a validation engine",
            ));
        }

        // 1. The workflow is captured first and the captured bytes are authoritative.
        let captured = self
            .recorder
            .capture(request.workflow(), CaptureRole::Workflow)
            .await?;
        let workflow = parse_workflow(&captured.bytes)?;
        request.check_bindings(&workflow)?;
        let limits = request.effective_limits(&workflow);
        let program = self.provider.preflight(&workflow, limits, &grant).await?;

        // 2. Inputs, then the durable intent. Nothing has run yet.
        let mut inputs = BTreeMap::new();
        let mut payloads = Vec::new();
        for binding in request.bindings() {
            let input = self
                .recorder
                .capture(&binding.artifact, CaptureRole::Input)
                .await?;
            inputs.insert(binding.name.clone(), input.capture.artifact);
            payloads.push(InputPayload {
                name: binding.name.clone(),
                digest: input.capture.digest.clone(),
                bytes: input.bytes,
            });
        }
        let intent = RunIntent {
            request_id: request.request_id().clone(),
            provider: self.provider.name().to_owned(),
            provider_version: self.provider.version().to_owned(),
            workflow: captured.capture.artifact,
            inputs,
            args: workflow.args().to_vec(),
            limits,
            env_names: self.provider.env_names(&workflow),
            program,
            platform: self.platform.clone(),
            started: self.clock.now(),
        };
        let recorded = self.recorder.record_intent(intent).await?;
        let run = *recorded.record.run();
        progress.run = Some(run);
        if !recorded.created {
            // The request was already recorded: never start a second child for it.
            return Ok(RunReport {
                record: recorded.record,
                created: false,
                validations: Vec::new(),
                warnings: Vec::new(),
            });
        }
        let _lease = self.recorder.acquire_lease(&run).await?;

        // 3. Everything a restart needs is durable; announce it and run.
        let started = NewEvent::new(
            EventKind::RunStarted,
            EventSubject::Artifact(run),
            EventData::new()
                .with("workflow", captured.capture.source.to_string())
                .with("provider", self.provider.name())
                .with("request_id", request.request_id().as_str()),
        );
        self.emit(started, &mut progress.warnings).await;

        let plan = ExecutionPlan {
            run,
            workflow: workflow.clone(),
            inputs: payloads,
            limits,
            grant,
        };
        let (finish, published, provider_error) =
            match self.provider.execute(plan.clone(), cancel).await {
                Ok(provider_run) => match provider_run.check(&plan) {
                    Ok(()) => {
                        let (finish, published) = finish_of(provider_run, &workflow);
                        (finish, published, None)
                    }
                    Err(error) => (
                        failure_finish(RunOutcome::Failed, &error),
                        Vec::new(),
                        Some(error),
                    ),
                },
                Err(error) => (
                    failure_finish(RunOutcome::StartFailed, &error),
                    Vec::new(),
                    Some(error),
                ),
            };

        // 4. The commit point. A failed write is an error and no completion event follows.
        let record = match self.recorder.record_result(&run, finish).await {
            Ok(record) => record,
            // Prefer the cause of a provider failure over a secondary failure to record it.
            Err(error) => return Err(provider_error.unwrap_or(error)),
        };

        // 5. Announce, clean up, validate.
        let (result_id, result) = record
            .result()
            .ok_or_else(|| FunctorError::Internal("a completed run has no result".to_owned()))?;
        let outputs = result.outputs().to_vec();
        let completed = NewEvent::new(
            EventKind::RunCompleted,
            EventSubject::Artifact(run),
            EventData::new()
                .with("outcome", result.outcome().as_str())
                .with("result", result_id.to_string())
                .with(
                    "outputs",
                    outputs.iter().map(ToString::to_string).collect::<Vec<_>>(),
                )
                .with("truncated", result.truncated().any()),
        );
        self.emit(completed, &mut progress.warnings).await;
        if let Some(error) = provider_error {
            // The run is complete as a failure; the provider's error is still the answer.
            return Err(error);
        }

        let succeeded = result.outcome().is_success();
        if succeeded && let Err(error) = self.provider.discard_staging(&run).await {
            progress
                .warnings
                .push(format!("staging could not be removed: {error}"));
        }

        let validations = match (&checks, succeeded && request.validates_outputs()) {
            (Some(checks), true) => {
                self.validate_outputs(checks, &outputs, &published, &mut progress.warnings)
                    .await
            }
            _ => Vec::new(),
        };
        Ok(RunReport {
            record,
            created: true,
            validations,
            warnings: std::mem::take(&mut progress.warnings),
        })
    }

    /// Validates each output as a scoped subject and persists a report for it. Reports are never
    /// validated in turn, and a failing or unsaved validation never changes the run.
    async fn validate_outputs(
        &self,
        checks: &OutputChecks<'_>,
        outputs: &[ArtifactId],
        published: &[PublishedOutput],
        warnings: &mut Vec<String>,
    ) -> Vec<OutputValidation> {
        let mut validations = Vec::new();
        for (output, bytes) in outputs.iter().zip(published) {
            let artifact = match checks.artifacts.get(output).await {
                Ok(Some(artifact)) => artifact,
                Ok(None) => {
                    warnings.push(format!("output {output} could not be found to validate"));
                    continue;
                }
                Err(error) => {
                    warnings.push(format!("output {output} could not be read: {error}"));
                    continue;
                }
            };
            let report = match checks
                .engine
                .validate(ValidationTarget::Artifact(ArtifactRef::Id(*output)))
                .await
            {
                Ok(report) => report,
                Err(error) => {
                    warnings.push(format!("output {output} could not be validated: {error}"));
                    continue;
                }
            };
            let subject = ReportSubject::Artifact {
                id: *output,
                revision: artifact.revision(),
                digest: Some(digest_of(&bytes.bytes)),
            };
            let saved = self
                .reports
                .save_report(&report, subject, false, checks.rules)
                .await;
            if let Ok(id) = &saved {
                let event = recorded_event(*id, &report, output.to_string());
                self.emit(event, warnings).await;
            }
            validations.push(OutputValidation {
                output: *output,
                report,
                saved,
            });
        }
        validations
    }

    /// Persists a validation report the caller computed, then publishes `validation.recorded`.
    /// The report is the caller's and is never modified or revalidated.
    ///
    /// # Errors
    /// The recorder's error (`NotFound` for a missing subject, `Storage`, `Inconsistent`); the
    /// caller keeps its report either way.
    pub async fn record_validation(
        &self,
        report: &ValidationReport,
        subject: ReportSubject,
        strict: bool,
        rules: &str,
    ) -> Result<SavedReport> {
        let id = self
            .reports
            .save_report(report, subject.clone(), strict, rules)
            .await?;
        let mut warnings = Vec::new();
        let label = match &subject {
            ReportSubject::Artifact { id, .. } => id.to_string(),
            _ => "repository".to_owned(),
        };
        self.emit(recorded_event(id, report, label), &mut warnings)
            .await;
        Ok(SavedReport { id, warnings })
    }

    /// Completes runs whose owner is gone as `interrupted`, repairs missing relationships and
    /// publishes `run.completed` for each completed run. It never reruns a child.
    ///
    /// # Errors
    /// The recorder's error.
    pub async fn recover(&self) -> Result<RecoveryReport> {
        let runs = self.recorder.recover().await?;
        let mut warnings = Vec::new();
        for record in &runs {
            let Some((result_id, result)) = record.result() else {
                continue;
            };
            let event = NewEvent::new(
                EventKind::RunCompleted,
                EventSubject::Artifact(*record.run()),
                EventData::new()
                    .with("outcome", result.outcome().as_str())
                    .with("result", result_id.to_string())
                    .with("outputs", Vec::<String>::new())
                    .with("truncated", false),
            );
            self.emit(event, &mut warnings).await;
        }
        Ok(RecoveryReport { runs, warnings })
    }
}

/// `validation.recorded`: the subject is the report artifact; `subject` names what was validated.
fn recorded_event(report_id: ArtifactId, report: &ValidationReport, subject: String) -> NewEvent {
    let counts = report.counts();
    NewEvent::new(
        EventKind::ValidationRecorded,
        EventSubject::Artifact(report_id),
        EventData::new()
            .with("outcome", report.outcome().as_str())
            .with("subject", subject)
            .with("errors", i64::try_from(counts.errors).unwrap_or(i64::MAX))
            .with(
                "warnings",
                i64::try_from(counts.warnings).unwrap_or(i64::MAX),
            )
            .with("infos", i64::try_from(counts.infos).unwrap_or(i64::MAX)),
    )
}

/// A finish for a run that never produced a trustworthy provider report.
fn failure_finish(outcome: RunOutcome, error: &FunctorError) -> RunFinish {
    RunFinish {
        outcome,
        exit_code: None,
        truncated: Truncation::default(),
        diagnostics: vec![error.to_string()],
        stdout: Vec::new(),
        stderr: Vec::new(),
        outputs: Vec::new(),
    }
}

/// The finish the recorder publishes for a provider report that passed its contract check, and
/// the outputs in the order they are published.
fn finish_of(
    run: functor_core::ProviderRun,
    workflow: &functor_model::Workflow,
) -> (RunFinish, Vec<PublishedOutput>) {
    let published: Vec<PublishedOutput> = run
        .outputs
        .into_iter()
        .filter_map(|output| {
            workflow
                .outputs()
                .iter()
                .find(|spec| spec.name == output.name)
                .map(|spec| PublishedOutput {
                    spec: spec.clone(),
                    bytes: output.bytes,
                })
        })
        .collect();
    let finish = RunFinish {
        outcome: run.outcome,
        exit_code: run.exit_code,
        truncated: Truncation {
            stdout: run.stdout.truncated,
            stderr: run.stderr.truncated,
        },
        diagnostics: run.diagnostics,
        stdout: run.stdout.bytes,
        stderr: run.stderr.bytes,
        outputs: published.clone(),
    };
    (finish, published)
}
