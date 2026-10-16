//! Stable run documents and terminal rendering. Streams are byte arrays in JSON, never lossy
//! strings; terminal output escapes control characters.

use std::collections::BTreeMap;

use functor_core::{FunctorError, RunStreams};
use functor_model::{ExecutionLimits, RunRecord, RunStatus};
use functor_runtime::OutputValidation;
use serde::Serialize;

use crate::output::SCHEMA_VERSION;
use crate::render::{ValidateDocument, sanitize, table};

#[derive(Debug, Serialize)]
pub struct LimitsDto {
    timeout_seconds: u32,
    stream_bytes: u64,
}

impl From<ExecutionLimits> for LimitsDto {
    fn from(limits: ExecutionLimits) -> Self {
        Self {
            timeout_seconds: limits.timeout_seconds(),
            stream_bytes: limits.stream_bytes(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ProgramDto {
    path: String,
    sha256: Option<String>,
}

#[derive(Debug, Serialize)]
struct TruncationDto {
    stdout: bool,
    stderr: bool,
}

#[derive(Debug, Serialize)]
struct ResultDto {
    id: String,
    outcome: &'static str,
    finished: String,
    exit_code: Option<i32>,
    truncated: TruncationDto,
    outputs: Vec<String>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Serialize)]
struct StreamsDto {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct RunDto {
    id: String,
    r#ref: String,
    state: &'static str,
    request_id: String,
    provider: String,
    provider_version: String,
    workflow_capture: String,
    inputs: BTreeMap<String, String>,
    args: Vec<String>,
    limits: LimitsDto,
    env_names: Vec<String>,
    program: ProgramDto,
    platform: String,
    network: &'static str,
    started: String,
    result: Option<ResultDto>,
    streams: Option<StreamsDto>,
}

impl RunDto {
    pub fn new(
        record: &RunRecord,
        status: RunStatus,
        streams: Option<RunStreams>,
    ) -> Result<Self, FunctorError> {
        let intent = record.intent();
        let state = match status {
            RunStatus::Running => "running",
            RunStatus::InterruptedUnknown => "interrupted_unknown",
            RunStatus::Completed(outcome) => outcome.as_str(),
            _ => {
                return Err(FunctorError::unsupported(
                    "this run state cannot be rendered by this CLI",
                ));
            }
        };
        Ok(Self {
            id: record.run().to_string(),
            r#ref: record.run().canonical(),
            state,
            request_id: intent.request_id.to_string(),
            provider: intent.provider.clone(),
            provider_version: intent.provider_version.clone(),
            workflow_capture: intent.workflow.to_string(),
            inputs: intent
                .inputs
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
            args: intent.args.clone(),
            limits: intent.limits.into(),
            env_names: intent.env_names.clone(),
            program: ProgramDto {
                path: intent.program.path.clone(),
                sha256: intent.program.digest.as_ref().map(ToString::to_string),
            },
            platform: intent.platform.clone(),
            network: "not_enforced",
            started: intent.started.to_string(),
            result: record.result().map(|(id, result)| ResultDto {
                id: id.to_string(),
                outcome: result.outcome().as_str(),
                finished: result.finished().to_string(),
                exit_code: result.exit_code(),
                truncated: TruncationDto {
                    stdout: result.truncated().stdout,
                    stderr: result.truncated().stderr,
                },
                outputs: result.outputs().iter().map(ToString::to_string).collect(),
                diagnostics: result.diagnostics().to_vec(),
            }),
            streams: streams.map(|s| StreamsDto {
                stdout: s.stdout,
                stderr: s.stderr,
            }),
        })
    }

    pub fn text(&self) -> String {
        let mut lines = vec![
            format!("run: {}", self.id),
            format!("state: {}", self.state),
            format!("request_id: {}", sanitize(&self.request_id)),
            format!(
                "provider: {} ({})",
                sanitize(&self.provider),
                sanitize(&self.provider_version)
            ),
            format!("workflow_capture: {}", self.workflow_capture),
            format!("program: {}", sanitize(&self.program.path)),
            format!(
                "args: {:?}",
                self.args.iter().map(|a| sanitize(a)).collect::<Vec<_>>()
            ),
            format!(
                "timeout: {} s; streams: {} bytes each",
                self.limits.timeout_seconds, self.limits.stream_bytes
            ),
            "network: not_enforced (not a sandbox)".to_owned(),
            format!("environment names: {}", self.env_names.join(", ")),
            format!("platform: {}", sanitize(&self.platform)),
            format!("started: {}", self.started),
        ];
        for (name, capture) in &self.inputs {
            lines.push(format!("input {}: {capture}", sanitize(name)));
        }
        if let Some(result) = &self.result {
            lines.push(format!("result: {}", result.id));
            lines.push(format!("finished: {}", result.finished));
            if let Some(code) = result.exit_code {
                lines.push(format!("exit_code: {code}"));
            }
            lines.push(format!(
                "truncated: stdout={}, stderr={}",
                result.truncated.stdout, result.truncated.stderr
            ));
            for output in &result.outputs {
                lines.push(format!("output: {output}"));
            }
            for diagnostic in &result.diagnostics {
                lines.push(format!("diagnostic: {}", sanitize(diagnostic)));
            }
        }
        if let Some(streams) = &self.streams {
            for (name, bytes) in [("stdout", &streams.stdout), ("stderr", &streams.stderr)] {
                lines.push(format!(
                    "{name}: {}",
                    sanitize(&String::from_utf8_lossy(bytes))
                ));
            }
        }
        lines.join("\n")
    }
}

#[derive(Debug, Serialize)]
pub struct OutputValidationDto {
    output: String,
    report: ValidateDocument,
    saved_report: Option<String>,
    save_error: Option<String>,
}

impl OutputValidationDto {
    pub fn new(validation: &OutputValidation) -> Result<Self, FunctorError> {
        Ok(Self {
            output: validation.output.to_string(),
            report: ValidateDocument::new(&validation.report, false)?,
            saved_report: validation.saved.as_ref().ok().map(ToString::to_string),
            save_error: validation.saved.as_ref().err().map(ToString::to_string),
        })
    }

    pub fn text(&self) -> String {
        format!(
            "validation {}: {}{}",
            self.output,
            self.report.outcome(),
            self.saved_report
                .as_ref()
                .map_or_else(String::new, |id| format!(" (report {id})"))
        )
    }
}

#[derive(Debug, Serialize)]
pub struct RunDocument {
    pub schema_version: u32,
    pub run: RunDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validations: Option<Vec<OutputValidationDto>>,
}

#[derive(Debug, Serialize)]
pub struct RunsDocument {
    pub schema_version: u32,
    pub runs: Vec<RunDto>,
}

impl RunsDocument {
    pub fn new(runs: Vec<RunDto>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            runs,
        }
    }

    pub fn text(&self, recovered: bool) -> String {
        if self.runs.is_empty() {
            return if recovered {
                "no interrupted runs recovered"
            } else {
                "no runs found"
            }
            .to_owned();
        }
        let rows = self
            .runs
            .iter()
            .map(|run| {
                vec![
                    run.id.clone(),
                    run.state.to_owned(),
                    sanitize(&run.request_id),
                    sanitize(&run.provider),
                ]
            })
            .collect::<Vec<_>>();
        table(&["RUN", "STATE", "REQUEST", "PROVIDER"], &rows)
    }
}

#[derive(Debug, Serialize)]
pub struct PlanDocument {
    pub schema_version: u32,
    pub program: String,
    pub args: Vec<String>,
    pub limits: LimitsDto,
    pub env_names: Vec<String>,
    pub network: &'static str,
    pub granted: bool,
}

impl PlanDocument {
    pub fn text(&self) -> String {
        format!(
            "execution plan (not a sandbox)\nprogram: {}\nargs: {:?}\ntimeout: {} s; streams: {} bytes each\nenvironment names: {}\nnetwork: not_enforced\npermission: {}",
            sanitize(&self.program),
            self.args.iter().map(|a| sanitize(a)).collect::<Vec<_>>(),
            self.limits.timeout_seconds,
            self.limits.stream_bytes,
            self.env_names.join(", "),
            if self.granted {
                "confirmation required"
            } else {
                "denied; pass --grant-exec"
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use functor_model::{ArtifactId, RunOutcome, RunResult, RunResultSpec, Timestamp, Truncation};

    #[test]
    fn every_run_state_has_stable_text_and_json_including_binary_streams() {
        let at = Timestamp::parse("2026-10-12T18:00:00Z").unwrap();
        let id = ArtifactId::from_bits(1);
        let intent = functor_testkit::sample_intent(
            "fixture-request",
            ArtifactId::from_bits(2),
            &[("data", ArtifactId::from_bits(3))],
            at,
        );
        for outcome in [
            RunOutcome::Succeeded,
            RunOutcome::Failed,
            RunOutcome::TimedOut,
            RunOutcome::Cancelled,
            RunOutcome::StartFailed,
            RunOutcome::Interrupted,
        ] {
            let success = outcome.is_success();
            let result = RunResult::new(
                id,
                at,
                RunResultSpec {
                    outcome,
                    finished: at,
                    exit_code: match outcome {
                        RunOutcome::Succeeded => Some(0),
                        RunOutcome::Failed => Some(3),
                        _ => None,
                    },
                    truncated: Truncation {
                        stdout: true,
                        stderr: false,
                    },
                    outputs: if success {
                        vec![ArtifactId::from_bits(5)]
                    } else {
                        vec![]
                    },
                    diagnostics: if success {
                        vec![]
                    } else {
                        vec!["fixture failure".to_owned()]
                    },
                },
            )
            .unwrap();
            let record = RunRecord::new(id, intent.clone())
                .complete(ArtifactId::from_bits(4), result)
                .unwrap();
            let run = RunDto::new(
                &record,
                record.status(false),
                Some(RunStreams {
                    stdout: vec![0, 255, 10],
                    stderr: b"boom\n".to_vec(),
                }),
            )
            .unwrap();
            let document = RunDocument {
                schema_version: SCHEMA_VERSION,
                run,
                created: Some(true),
                validations: Some(Vec::new()),
            };
            insta::assert_snapshot!(
                format!("run_{}", outcome.as_str()),
                format!(
                    "--- text ---\n{}\n--- json ---\n{}",
                    document.run.text(),
                    serde_json::to_string_pretty(&document).unwrap()
                )
            );
        }
        let record = RunRecord::new(id, intent);
        for (held, name) in [(true, "running"), (false, "interrupted_unknown")] {
            let run = RunDto::new(&record, record.status(held), None).unwrap();
            insta::assert_snapshot!(
                format!("run_{name}"),
                format!(
                    "--- text ---\n{}\n--- json ---\n{}",
                    run.text(),
                    serde_json::to_string_pretty(&run).unwrap()
                )
            );
        }
    }
}
