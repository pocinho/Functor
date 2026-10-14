//! Execution is orchestrated in `functor-runtime`; these handlers only project CLI arguments,
//! obtain explicit permission and render the library's records.

use std::future::Future;
use std::io::{self, BufRead, IsTerminal, Write};
use std::pin::pin;
use std::sync::Arc;

use functor_core::{
    CancelToken, EvidenceRecorder, ExecutionGrant, ExecutionProvider, FunctorError, Result,
};
use functor_model::{
    ArtifactRef, EntityKind, ExecutionLimits, InputBinding, RequestId, RunRequest, RunStatus,
};
use functor_runtime::{OutputChecks, parse_workflow};
use functor_validation::RuleValidationEngine;
use sha2::{Digest, Sha256};

use crate::cli::{RunCommand, RunStartArgs};
use crate::context::{self, ExecutionContext, Globals, Services};
use crate::exit;
use crate::output::{SCHEMA_VERSION, print_result, report_run_failure};
use crate::resolve;
use crate::run_render::{OutputValidationDto, PlanDocument, RunDocument, RunDto, RunsDocument};

use super::watch::{Either, race};

pub const RULES: &str = "V0001-V0023";

fn output_error(error: io::Error) -> FunctorError {
    FunctorError::storage("write output", "stdout", error)
}

pub fn warnings(items: &[String]) {
    for warning in items {
        eprintln!("warning: {}", crate::render::sanitize(warning));
    }
}

async fn view(
    context: &ExecutionContext,
    record: &functor_model::RunRecord,
    streams: bool,
) -> Result<RunDto> {
    let status = if record.result().is_some() {
        record.status(false)
    } else {
        context.evidence.status(record.run()).await?
    };
    // A result may commit between the record read and the lease/status check.
    let refreshed;
    let record = if matches!(status, RunStatus::Completed(_)) && record.result().is_none() {
        refreshed = context
            .evidence
            .run(&ArtifactRef::Id(*record.run()))
            .await?
            .ok_or_else(|| {
                FunctorError::not_found(EntityKind::Artifact, record.run().to_string())
            })?;
        &refreshed
    } else {
        record
    };
    let streams = if streams && record.result().is_some() {
        context.evidence.streams(record.run()).await?
    } else {
        None
    };
    RunDto::new(record, status, streams)
}

/// Executes a run subcommand. Inspection reads authoritative evidence, never the event log.
pub async fn run(command: RunCommand, globals: &Globals, services: &Services) -> Result<u8> {
    let store = context::open_unchecked(globals, services)?;
    let context = context::execution(store, services).await?;
    match command {
        RunCommand::Start(args) => start(args, &context, globals, services).await,
        RunCommand::Show { run } => {
            let artifact = resolve::artifact(context.evidence.store(), &run).await?;
            let record = context
                .evidence
                .run(&ArtifactRef::Id(*artifact.id()))
                .await?
                .ok_or_else(|| {
                    FunctorError::invalid_input("run", format!("`{run}` is not a run artifact"))
                })?;
            let document = RunDocument {
                schema_version: SCHEMA_VERSION,
                run: view(&context, &record, true).await?,
                created: None,
                validations: None,
            };
            print_result(&document.run.text(), &document, globals.output).map_err(output_error)?;
            Ok(exit::SUCCESS)
        }
        RunCommand::List | RunCommand::Recover => {
            let recovered = matches!(command, RunCommand::Recover);
            let records = if recovered {
                let result = context.orchestrator.recover().await?;
                warnings(&result.warnings);
                result.runs
            } else {
                context.evidence.runs().await?
            };
            let mut runs = Vec::new();
            for record in records {
                runs.push(view(&context, &record, false).await?);
            }
            let document = RunsDocument::new(runs);
            print_result(&document.text(recovered), &document, globals.output)
                .map_err(output_error)?;
            Ok(exit::SUCCESS)
        }
    }
}

fn bindings(values: &[String]) -> Result<Vec<InputBinding>> {
    values
        .iter()
        .map(|value| {
            let (name, reference) = value.split_once('=').ok_or_else(|| {
                FunctorError::invalid_input("input", format!("`{value}` must be NAME=ARTIFACT"))
            })?;
            Ok(InputBinding {
                name: name.to_owned(),
                artifact: ArtifactRef::parse(reference)?,
            })
        })
        .collect()
}

/// The terminal confirmation is deliberately independent of `--quiet` and the output mode.
fn confirm(plan: &PlanDocument) -> Result<()> {
    confirm_with(plan, &mut io::stdin().lock(), &mut io::stderr().lock())
}

fn confirm_with(
    plan: &PlanDocument,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<()> {
    writeln!(output, "{}\nType yes to execute:", plan.text())
        .and_then(|()| output.flush())
        .map_err(|error| FunctorError::storage("write confirmation", "stderr", error))?;
    let mut answer = String::new();
    input
        .read_line(&mut answer)
        .map_err(|error| FunctorError::storage("read confirmation", "stdin", error))?;
    if answer.trim() != "yes" {
        return Err(FunctorError::permission_denied(
            "execute",
            "confirmation was not `yes`; no run was started",
        ));
    }
    Ok(())
}

/// Never drop an in-flight execution on interruption: cancel, then await terminal publication.
async fn cancellable<T>(
    execution: impl Future<Output = T>,
    token: CancelToken,
    signal: impl Future<Output = io::Result<()>>,
) -> T {
    let mut execution = pin!(execution);
    match race(execution.as_mut(), signal).await {
        Either::Left(result) => result,
        Either::Right(signal) => {
            token.cancel();
            if let Err(error) = signal {
                eprintln!("warning: could not listen for Ctrl-C; cancelling execution: {error}");
            }
            execution.await
        }
    }
}

async fn prepare(
    args: &RunStartArgs,
    context: &ExecutionContext,
    services: &Services,
) -> Result<(RunRequest, ExecutionGrant, PlanDocument)> {
    let workflow_artifact = resolve::artifact(context.evidence.store(), &args.workflow).await?;
    let bytes = context::content(&context.evidence, &workflow_artifact)
        .await?
        .ok_or_else(|| FunctorError::invalid_input("workflow", "the artifact has no content"))?;
    let workflow = parse_workflow(&bytes)?;
    let request_id = match &args.request_id {
        Some(id) => RequestId::new(id)?,
        None => RequestId::new(&services.ids.artifact_id().to_string().to_ascii_lowercase())?,
    };
    let mut request = RunRequest::new(
        ArtifactRef::Id(*workflow_artifact.id()),
        bindings(&args.inputs)?,
        request_id,
    )?
    .with_workflow_digest(functor_model::ContentDigest::from_bytes(
        Sha256::digest(&bytes).into(),
    ));
    if let Some(timeout) = args.timeout {
        request = request.with_limits(ExecutionLimits::new(
            timeout,
            workflow.limits().stream_bytes(),
        )?);
    }
    if args.validate_outputs {
        request = request.with_output_validation();
    }
    request.check_bindings(&workflow)?;
    // Resolve input references while still read-only, so typos cannot leave captures behind.
    for binding in request.bindings() {
        let _ = resolve::artifact(context.evidence.store(), &binding.artifact.to_string()).await?;
    }
    let program = context
        .provider
        .resolve_program(workflow.program())?
        .map_or_else(
            || workflow.program().to_owned(),
            |path| path.to_string_lossy().into_owned(),
        );
    let limits = request.effective_limits(&workflow);
    let mut grant = ExecutionGrant::new(&program, limits);
    for name in workflow.env_allow() {
        grant = grant.with_env(name);
    }
    let plan = PlanDocument {
        schema_version: SCHEMA_VERSION,
        program,
        args: workflow.args().to_vec(),
        limits: limits.into(),
        env_names: context.provider.env_names(&workflow),
        network: "not_enforced",
        granted: args.grant_exec,
    };
    Ok((request, grant, plan))
}

async fn start(
    args: RunStartArgs,
    context: &ExecutionContext,
    globals: &Globals,
    services: &Services,
) -> Result<u8> {
    let (request, grant, plan) = prepare(&args, context, services).await?;
    if !args.grant_exec {
        print_result(&plan.text(), &plan, globals.output).map_err(output_error)?;
        return Err(FunctorError::permission_denied(
            "execute",
            "no permission; review the plan and pass --grant-exec",
        ));
    }
    if io::stdin().is_terminal() {
        confirm(&plan)?;
    }

    let engine = RuleValidationEngine::new(context.evidence.store(), Arc::clone(&services.clock));
    let checks = args.validate_outputs.then_some(OutputChecks {
        engine: &engine,
        artifacts: context.evidence.store(),
        rules: RULES,
    });
    let token = CancelToken::new();
    let result = cancellable(
        context
            .orchestrator
            .execute(&request, grant, token.clone(), checks),
        token,
        tokio::signal::ctrl_c(),
    )
    .await;
    let report = match result {
        Ok(report) => report,
        Err(failure) => return Ok(report_run_failure(&failure, globals)),
    };
    let run = *report.record.run();
    match print_report(report, context, globals).await {
        Ok(code) => Ok(code),
        Err(error) => Ok(report_run_failure(
            &functor_runtime::RunFailure {
                run: Some(run),
                error,
                warnings: Vec::new(),
            },
            globals,
        )),
    }
}

async fn print_report(
    report: functor_runtime::RunReport,
    context: &ExecutionContext,
    globals: &Globals,
) -> Result<u8> {
    warnings(&report.warnings);
    let mut code = if report
        .record
        .result()
        .is_some_and(|(_, result)| result.outcome().is_success())
    {
        exit::SUCCESS
    } else {
        exit::RUN_FAILED
    };
    for validation in &report.validations {
        if let Err(error) = &validation.saved {
            eprintln!(
                "error: output {} report was not saved: {error}",
                validation.output
            );
            code = exit::STORAGE;
        }
    }
    let validations = report
        .validations
        .iter()
        .map(OutputValidationDto::new)
        .collect::<Result<Vec<_>>>()?;
    let document = RunDocument {
        schema_version: SCHEMA_VERSION,
        run: view(context, &report.record, true).await?,
        created: Some(report.created),
        validations: Some(validations),
    };
    let mut text = document.run.text();
    for validation in document.validations.as_ref().into_iter().flatten() {
        text.push('\n');
        text.push_str(&validation.text());
    }
    print_result(&text, &document, globals.output).map_err(output_error)?;
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use functor_core::ErrorKind;

    #[test]
    fn terminal_permission_requires_an_explicit_yes_and_always_shows_the_plan() {
        let plan = PlanDocument {
            schema_version: SCHEMA_VERSION,
            program: "tool".to_owned(),
            args: vec!["literal".to_owned()],
            limits: ExecutionLimits::DEFAULT.into(),
            env_names: Vec::new(),
            network: "not_enforced",
            granted: true,
        };
        for (answer, accepted) in [
            ("yes\n", true),
            ("no\n", false),
            ("y\n", false),
            ("YES\n", false),
            ("", false),
        ] {
            let mut output = Vec::new();
            let result = confirm_with(&plan, &mut io::Cursor::new(answer), &mut output);
            assert_eq!(result.is_ok(), accepted);
            if let Err(error) = result {
                assert_eq!(error.kind(), ErrorKind::PermissionDenied);
            }
            let text = String::from_utf8(output).unwrap();
            assert!(text.contains("not a sandbox"));
            assert!(text.contains("Type yes to execute:"));
        }
    }

    #[tokio::test]
    async fn interruption_cancels_and_awaits_the_terminal_record_instead_of_dropping_execution() {
        let token = CancelToken::new();
        let inner = token.clone();
        let execution = async {
            std::future::poll_fn(|cx| {
                if inner.is_cancelled() {
                    std::task::Poll::Ready("terminal evidence committed")
                } else {
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                }
            })
            .await
        };
        let result = cancellable(execution, token, async { Ok(()) }).await;
        assert_eq!(result, "terminal evidence committed");
    }
}
