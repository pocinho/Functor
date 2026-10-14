//! The `local-process` execution provider (ADR-0012).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use functor_core::{
    CancelToken, ExecutionGrant, ExecutionPlan, ExecutionProvider, FunctorError, OutputPayload,
    ProviderRun, Result, StreamCapture, check_grant,
};
use functor_model::{
    ArtifactId, ContentDigest, ExecutionLimits, LOCAL_PROCESS_PROVIDER, ProgramProvenance,
    RunOutcome, Workflow, bounds,
};
use sha2::{Digest, Sha256};

use crate::program::{file_digest, plain, resolve, same_file};
use crate::supervise::{Failure, Interrupt, Spawn, Supervised, run};

const VERSION: &str = concat!("functor-runtime ", env!("CARGO_PKG_VERSION"));

/// Variables every child receives when the caller has them: enough for a program to find its
/// system libraries and temporary directory, and nothing else.
#[cfg(windows)]
const DEFAULT_ENV: [&str; 4] = ["SystemRoot", "PATH", "TEMP", "TMP"];
#[cfg(not(windows))]
const DEFAULT_ENV: [&str; 2] = ["PATH", "TMPDIR"];

/// Runs a workflow's program as a child process: no shell, a cleared environment, bounded output,
/// a timeout and cancellation that kill the whole process tree.
///
/// This is **not a sandbox**. The child runs with the caller's privileges and can read, write and
/// connect wherever the caller can; the work directory is a convention for inputs and outputs. The
/// provider never opens the repository, persists, validates or publishes (ADR-0012); it only
/// touches the staging directory `<work root>/<run id>/work`.
#[derive(Debug, Clone)]
pub struct LocalProcessProvider {
    root: PathBuf,
    work_root: PathBuf,
    path_var: Option<OsString>,
    started: Arc<AtomicUsize>,
}

impl LocalProcessProvider {
    /// A provider for the repository rooted at `root`; staging goes under `.functor/runs`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            work_root: root.join(".functor").join("runs"),
            root,
            path_var: None,
            started: Arc::default(),
        }
    }

    /// Stages runs under `work_root` instead.
    #[must_use]
    pub fn with_work_root(mut self, work_root: impl Into<PathBuf>) -> Self {
        self.work_root = work_root.into();
        self
    }

    /// Looks bare program names up on `path` instead of the process `PATH`.
    #[must_use]
    pub fn with_search_path(mut self, path: impl Into<OsString>) -> Self {
        self.path_var = Some(path.into());
        self
    }

    /// The staging directory of a run: the child's working directory, with `inputs/` and
    /// `outputs/` inside. It is left in place after the run for the orchestrator to keep or
    /// remove, and recreated empty by the next execution of the same run.
    #[must_use]
    pub fn work_dir(&self, run: &ArtifactId) -> PathBuf {
        self.work_root.join(run.to_string()).join("work")
    }

    /// Resolves a workflow's program to the absolute path a grant must name.
    ///
    /// # Errors
    /// `PermissionDenied` when a repository-relative program leaves the repository.
    pub fn resolve_program(&self, program: &str) -> Result<Option<PathBuf>> {
        resolve(&self.root, program, self.path_var.as_deref())
    }

    /// How many executions have started a child process.
    #[must_use]
    pub fn started(&self) -> usize {
        self.started.load(Ordering::SeqCst)
    }

    fn execute_blocking(&self, plan: &ExecutionPlan, cancel: &CancelToken) -> Result<ProviderRun> {
        let workflow = &plan.workflow;
        verify_inputs(plan)?;

        // Resolution and the grant check come before any side effect.
        let Some(program) = resolve(&self.root, workflow.program(), self.path_var.as_deref())?
        else {
            return Ok(refused_start(
                workflow.program(),
                format!("program `{}` was not found", workflow.program()),
            ));
        };
        if !same_file(&program, Path::new(plan.grant.program())) {
            return Err(FunctorError::permission_denied(
                "execute",
                format!(
                    "the program `{}` is not the granted program `{}`",
                    program.display(),
                    plan.grant.program()
                ),
            ));
        }
        let provenance = ProgramProvenance {
            path: program.to_string_lossy().into_owned(),
            digest: file_digest(&program),
        };
        if cancel.is_cancelled() {
            return Ok(outcome(RunOutcome::Cancelled, provenance));
        }

        let work = self.stage(plan)?;
        self.started.fetch_add(1, Ordering::SeqCst);
        let spawn = Spawn {
            program,
            args: workflow.args().to_vec(),
            cwd: work.clone(),
            env: child_env(workflow),
        };
        let timeout = Duration::from_secs(u64::from(plan.limits.timeout_seconds()));
        let supervised = match run(&spawn, timeout, plan.limits.stream_bytes(), cancel) {
            Ok(supervised) => supervised,
            Err(Failure::Spawn(error)) => {
                return Ok(ProviderRun {
                    diagnostics: vec![format!(
                        "program `{}` could not be started ({:?})",
                        workflow.program(),
                        error.kind()
                    )],
                    ..outcome(RunOutcome::StartFailed, provenance)
                });
            }
            Err(Failure::Wait(error)) => {
                return Err(FunctorError::storage("wait", &spawn.program, error));
            }
        };
        Ok(classify(supervised, plan, &work, provenance))
    }

    /// Creates a fresh staging directory with the inputs in place.
    fn stage(&self, plan: &ExecutionPlan) -> Result<PathBuf> {
        let work = self.work_dir(&plan.run);
        match fs::remove_dir_all(&work) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(FunctorError::storage("remove directory", &work, error)),
        }
        let inputs = work.join("inputs");
        for dir in [&inputs, &work.join("outputs")] {
            fs::create_dir_all(dir)
                .map_err(|error| FunctorError::storage("create directory", dir, error))?;
        }
        for input in &plan.inputs {
            let file = inputs.join(&input.name);
            fs::write(&file, &input.bytes)
                .map_err(|error| FunctorError::storage("write", &file, error))?;
        }
        Ok(plain(work))
    }
}

/// The child's environment: the default allowlist, the workflow's allowed names the caller has,
/// then the workflow's literal values. Nothing else is inherited.
fn child_env(workflow: &Workflow) -> BTreeMap<OsString, OsString> {
    let mut env = BTreeMap::new();
    let allowed = workflow.env_allow().iter().map(String::as_str);
    for name in DEFAULT_ENV.into_iter().chain(allowed) {
        if let Some(value) = std::env::var_os(name) {
            env.insert(OsString::from(name), value);
        }
    }
    for (name, value) in workflow.env_set() {
        env.insert(OsString::from(name), OsString::from(value));
    }
    env
}

/// The captured inputs must still be what was captured: refuse before anything is staged.
fn verify_inputs(plan: &ExecutionPlan) -> Result<()> {
    for input in &plan.inputs {
        let actual = ContentDigest::from_bytes(Sha256::digest(&input.bytes).into());
        if actual != input.digest {
            return Err(FunctorError::invalid_input(
                "inputs",
                format!("input `{}` does not match its captured digest", input.name),
            ));
        }
    }
    Ok(())
}

fn outcome(outcome: RunOutcome, program: ProgramProvenance) -> ProviderRun {
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

fn refused_start(program: &str, reason: String) -> ProviderRun {
    ProviderRun {
        diagnostics: vec![reason],
        ..outcome(
            RunOutcome::StartFailed,
            ProgramProvenance {
                path: program.to_owned(),
                digest: None,
            },
        )
    }
}

fn classify(
    supervised: Supervised,
    plan: &ExecutionPlan,
    work: &Path,
    program: ProgramProvenance,
) -> ProviderRun {
    let Supervised {
        exit_code,
        exited,
        interrupt,
        stdout,
        stderr,
    } = supervised;
    let base = ProviderRun {
        stdout,
        stderr,
        ..outcome(RunOutcome::Failed, program)
    };
    match (interrupt, exit_code) {
        (Some(Interrupt::TimedOut), _) => ProviderRun {
            outcome: RunOutcome::TimedOut,
            diagnostics: vec![format!(
                "the run exceeded {} seconds",
                plan.limits.timeout_seconds()
            )],
            ..base
        },
        (Some(Interrupt::Cancelled), _) => ProviderRun {
            outcome: RunOutcome::Cancelled,
            ..base
        },
        (None, Some(0)) => finish_success(base, plan, work),
        (None, Some(code)) => ProviderRun {
            exit_code: Some(code),
            diagnostics: vec![format!("the program exited with status {code}")],
            ..base
        },
        (None, None) => ProviderRun {
            diagnostics: vec![if exited {
                "the program was terminated by a signal".to_owned()
            } else {
                "the program did not report an exit status".to_owned()
            }],
            ..base
        },
    }
}

/// Reads the declared outputs of a run that exited 0. A missing required output, a non-file, an
/// oversized file or too much output in total turns the run into a failure with a diagnostic.
fn finish_success(mut run: ProviderRun, plan: &ExecutionPlan, work: &Path) -> ProviderRun {
    let outputs_dir = work.join("outputs");
    let mut produced = Vec::new();
    let mut problems = Vec::new();
    let mut total: u64 = 0;
    for spec in plan.workflow.outputs() {
        let path = outputs_dir.join(&spec.name);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if spec.required {
                    problems.push(format!("required output `{}` is missing", spec.name));
                }
                continue;
            }
            Err(error) => {
                problems.push(format!(
                    "output `{}` cannot be read ({:?})",
                    spec.name,
                    error.kind()
                ));
                continue;
            }
        };
        if !metadata.is_file() {
            problems.push(format!("output `{}` is not a regular file", spec.name));
            continue;
        }
        if metadata.len() > bounds::MAX_ITEM_BYTES {
            problems.push(format!(
                "output `{}` is larger than {} bytes",
                spec.name,
                bounds::MAX_ITEM_BYTES
            ));
            continue;
        }
        total += metadata.len();
        if total > bounds::MAX_TOTAL_BYTES {
            problems.push(format!(
                "outputs are larger than {} bytes in total",
                bounds::MAX_TOTAL_BYTES
            ));
            break;
        }
        match fs::read(&path) {
            Ok(bytes) => produced.push(OutputPayload {
                name: spec.name.clone(),
                bytes,
            }),
            Err(error) => problems.push(format!(
                "output `{}` cannot be read ({:?})",
                spec.name,
                error.kind()
            )),
        }
    }
    run.exit_code = Some(0);
    if problems.is_empty() {
        run.outcome = RunOutcome::Succeeded;
        run.outputs = produced;
    } else {
        run.outcome = RunOutcome::Failed;
        run.diagnostics = problems;
    }
    run
}

#[async_trait]
impl ExecutionProvider for LocalProcessProvider {
    fn name(&self) -> &'static str {
        LOCAL_PROCESS_PROVIDER
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    async fn preflight(
        &self,
        workflow: &Workflow,
        limits: ExecutionLimits,
        grant: &ExecutionGrant,
    ) -> Result<ProgramProvenance> {
        check_grant(self.name(), workflow, limits, grant)?;
        let provider = self.clone();
        let program = workflow.program().to_owned();
        let granted = grant.program().to_owned();
        tokio::task::spawn_blocking(move || {
            // A program that cannot be found is described as named: the run will record a start
            // failure. One that is found must be exactly the granted one.
            let Some(path) = provider.resolve_program(&program)? else {
                return Ok(ProgramProvenance {
                    path: program,
                    digest: None,
                });
            };
            if !same_file(&path, Path::new(&granted)) {
                return Err(FunctorError::permission_denied(
                    "execute",
                    format!(
                        "the program `{}` is not the granted program `{granted}`",
                        path.display()
                    ),
                ));
            }
            Ok(ProgramProvenance {
                digest: file_digest(&path),
                path: path.to_string_lossy().into_owned(),
            })
        })
        .await
        .map_err(|error| FunctorError::Internal(format!("preflight task failed: {error}")))?
    }

    fn env_names(&self, workflow: &Workflow) -> Vec<String> {
        child_env(workflow)
            .keys()
            .map(|name| name.to_string_lossy().into_owned())
            .collect()
    }

    async fn discard_staging(&self, run: &ArtifactId) -> Result<()> {
        let dir = self.work_root.join(run.to_string());
        tokio::task::spawn_blocking(move || match fs::remove_dir_all(&dir) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(FunctorError::storage("remove directory", &dir, error)),
        })
        .await
        .map_err(|error| FunctorError::Internal(format!("cleanup task failed: {error}")))?
    }

    async fn execute(&self, plan: ExecutionPlan, cancel: CancelToken) -> Result<ProviderRun> {
        plan.check(self.name())?;
        let provider = self.clone();
        tokio::task::spawn_blocking(move || provider.execute_blocking(&plan, &cancel))
            .await
            .map_err(|error| FunctorError::Internal(format!("execution task failed: {error}")))?
    }
}
