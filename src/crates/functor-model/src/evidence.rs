//! Execution and evidence values (ADR-0012 to ADR-0015).
//!
//! Pure values: workflows, limits, captures, run intent and results, and the evidence artifact
//! kinds. Nothing here reads a file, starts a process or hashes bytes; persistence, providers and
//! digests live in the crates that implement the core traits. See
//! `docs/design/execution-and-evidence.md` for the numbers these types enforce.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};
use std::str::FromStr;

use crate::artifact::{Artifact, ArtifactType, MetadataKey, MetadataValue, Revision};
use crate::error::{ModelError, Result};
use crate::grammar::{check_name, is_lower, is_lower_digit};
use crate::id::{ArtifactId, ArtifactRef};
use crate::timestamp::Timestamp;

/// Hard and default bounds of Phase 1 execution (contract: "Limits").
pub mod bounds {
    /// Default wall-clock timeout in seconds.
    pub const DEFAULT_TIMEOUT_SECONDS: u32 = 60;
    /// Hard maximum wall-clock timeout in seconds.
    pub const MAX_TIMEOUT_SECONDS: u32 = 3600;
    /// Default captured bytes per output stream.
    pub const DEFAULT_STREAM_BYTES: u64 = 1024 * 1024;
    /// Hard maximum captured bytes per output stream.
    pub const MAX_STREAM_BYTES: u64 = 8 * 1024 * 1024;
    /// Maximum number of arguments.
    pub const MAX_ARGS: usize = 64;
    /// Maximum size of one argument in bytes.
    pub const MAX_ARG_BYTES: usize = 1024;
    /// Maximum number of environment variables allowed or set.
    pub const MAX_ENV_VARS: usize = 32;
    /// Maximum number of declared or bound inputs.
    pub const MAX_INPUTS: usize = 32;
    /// Maximum number of declared outputs.
    pub const MAX_OUTPUTS: usize = 16;
    /// Maximum size of one input or output in bytes.
    pub const MAX_ITEM_BYTES: u64 = 16 * 1024 * 1024;
    /// Maximum combined size of all inputs, and of all outputs, in bytes.
    pub const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
    /// Maximum size of workflow content in bytes.
    pub const MAX_WORKFLOW_BYTES: u64 = 1024 * 1024;
}

/// The only workflow schema of Phase 1.
pub const WORKFLOW_SCHEMA: &str = "functor.workflow/1";
/// The provider shipped in Phase 1 (ADR-0012).
pub const LOCAL_PROCESS_PROVIDER: &str = "local-process";
/// The network policy recorded as provenance: the local provider cannot enforce one (ADR-0012).
pub const NETWORK_NOT_ENFORCED: &str = "not_enforced";
/// The metadata key that marks an artifact as evidence and names its schema.
pub const EVIDENCE_KEY: &str = "evidence";

/// The kinds of evidence artifact (ADR-0013, ADR-0015). Evidence artifacts are immutable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum EvidenceKind {
    /// The exact bytes of a workflow or input at run time.
    Capture,
    /// Run intent, recorded before any side effect.
    Run,
    /// The terminal record of a run; its existence is the commit point.
    RunResult,
    /// A persisted validation report.
    ValidationReport,
}

impl EvidenceKind {
    /// Every kind.
    pub const ALL: [Self; 4] = [
        Self::Capture,
        Self::Run,
        Self::RunResult,
        Self::ValidationReport,
    ];

    /// The artifact `type` value.
    #[must_use]
    pub const fn artifact_type(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::Run => "run",
            Self::RunResult => "run_result",
            Self::ValidationReport => "validation_report",
        }
    }

    /// The value of the `evidence` metadata key.
    #[must_use]
    pub const fn schema(self) -> &'static str {
        match self {
            Self::Capture => "functor.capture/1",
            Self::Run => "functor.run/1",
            Self::RunResult => "functor.run_result/1",
            Self::ValidationReport => "functor.report/1",
        }
    }

    /// The prefix of generated slugs for this kind.
    #[must_use]
    pub const fn slug_prefix(self) -> &'static str {
        match self {
            Self::Capture => "capture-",
            Self::Run => "run-",
            Self::RunResult => "run-result-",
            Self::ValidationReport => "report-",
        }
    }

    /// The artifact type as a validated value.
    ///
    /// # Panics
    /// Never: the constants satisfy the artifact type grammar (pinned by a test).
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn to_artifact_type(self) -> ArtifactType {
        ArtifactType::new(self.artifact_type()).expect("evidence type names are valid")
    }

    /// The artifact kind of an evidence artifact, or `None`.
    ///
    /// An artifact is evidence only when its type **and** its `evidence` metadata agree; an
    /// ordinary artifact that happens to use a type name such as `run` is not (ADR-0013).
    #[must_use]
    pub fn of(artifact: &Artifact) -> Option<Self> {
        let key = MetadataKey::new(EVIDENCE_KEY).ok()?;
        let MetadataValue::String(schema) = artifact.metadata().get(&key)? else {
            return None;
        };
        Self::ALL.into_iter().find(|kind| {
            kind.artifact_type() == artifact.artifact_type().as_str() && kind.schema() == schema
        })
    }
}

impl fmt::Display for EvidenceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.artifact_type())
    }
}

/// A SHA-256 digest in the text form `sha256:<64 lowercase hex characters>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentDigest(String);

impl ContentDigest {
    const PREFIX: &'static str = "sha256:";

    /// Validates the text form.
    ///
    /// # Errors
    /// Fails unless the text is `sha256:` followed by exactly 64 lowercase hex digits.
    pub fn new(input: &str) -> Result<Self> {
        let Some(hex) = input.strip_prefix(Self::PREFIX) else {
            return Err(ModelError::new("digest", "must start with `sha256:`"));
        };
        if hex.len() != 64 || !hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
            return Err(ModelError::new(
                "digest",
                "must be 64 lowercase hexadecimal digits",
            ));
        }
        Ok(Self(input.to_owned()))
    }

    /// Builds the digest from the 32 raw bytes of a SHA-256 hash.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        let mut text = String::from(Self::PREFIX);
        for byte in bytes {
            let _ = write!(text, "{byte:02x}");
        }
        Self(text)
    }

    /// The full text, `sha256:<hex>`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The hexadecimal part, which names the content-addressed file of a capture.
    #[must_use]
    pub fn hex(&self) -> &str {
        &self.0[Self::PREFIX.len()..]
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ContentDigest {
    type Err = ModelError;

    fn from_str(input: &str) -> Result<Self> {
        Self::new(input)
    }
}

/// Why something was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum CaptureRole {
    /// The workflow artifact itself.
    Workflow,
    /// An input bound to a workflow input name.
    Input,
}

impl CaptureRole {
    /// The stable text form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::Input => "input",
        }
    }

    /// Parses the text form.
    ///
    /// # Errors
    /// Fails for any other text.
    pub fn parse(input: &str) -> Result<Self> {
        match input {
            "workflow" => Ok(Self::Workflow),
            "input" => Ok(Self::Input),
            _ => Err(ModelError::new(
                "capture role",
                "must be `workflow` or `input`",
            )),
        }
    }
}

/// The exact state of one artifact taken before a run: which artifact and revision it came from,
/// and the digest and size of the captured bytes. The `artifact` is the immutable `capture`
/// artifact that stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    /// The evidence artifact holding the captured bytes.
    pub artifact: ArtifactId,
    /// The live artifact the bytes were read from.
    pub source: ArtifactId,
    /// The revision of the live artifact at capture time (a counter, not a version).
    pub source_revision: Revision,
    /// What the capture is for.
    pub role: CaptureRole,
    /// The digest of the captured bytes.
    pub digest: ContentDigest,
    /// The size of the captured bytes.
    pub size: u64,
}

/// The outcome of a completed run (ADR-0013). An outcome is data, not an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RunOutcome {
    /// The child exited with status 0 and every required output exists.
    Succeeded,
    /// The child exited non-zero, or a required output was missing or invalid.
    Failed,
    /// The wall-clock timeout elapsed and the process tree was killed.
    TimedOut,
    /// The run was cancelled and the process tree was killed.
    Cancelled,
    /// The program or provider could not be started.
    StartFailed,
    /// Written only by recovery for a run whose owner is gone.
    Interrupted,
}

impl RunOutcome {
    /// Every outcome.
    pub const ALL: [Self; 6] = [
        Self::Succeeded,
        Self::Failed,
        Self::TimedOut,
        Self::Cancelled,
        Self::StartFailed,
        Self::Interrupted,
    ];

    /// The stable text form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
            Self::StartFailed => "start_failed",
            Self::Interrupted => "interrupted",
        }
    }

    /// Parses the text form.
    ///
    /// # Errors
    /// Fails for an unknown outcome.
    pub fn parse(input: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|outcome| outcome.as_str() == input)
            .ok_or_else(|| ModelError::new("run outcome", format!("unknown outcome `{input}`")))
    }

    /// Whether the run succeeded.
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Succeeded)
    }
}

impl fmt::Display for RunOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RunOutcome {
    type Err = ModelError;

    fn from_str(input: &str) -> Result<Self> {
        Self::parse(input)
    }
}

/// Where a run is, derived from which records exist (ADR-0013): never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RunStatus {
    /// Intent recorded, no result, and the per-run lock is held.
    Running,
    /// Intent recorded, no result, and nobody holds the lock: the owner is gone.
    InterruptedUnknown,
    /// A result exists.
    Completed(RunOutcome),
}

/// Numeric limits of one run (contract: "Limits"). A workflow or a grant may only lower the
/// defaults, never exceed the hard maximums.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionLimits {
    timeout_seconds: u32,
    stream_bytes: u64,
}

impl ExecutionLimits {
    /// The defaults: 60 s and 1 MiB per stream.
    pub const DEFAULT: Self = Self {
        timeout_seconds: bounds::DEFAULT_TIMEOUT_SECONDS,
        stream_bytes: bounds::DEFAULT_STREAM_BYTES,
    };

    /// The hard maximums: 3600 s and 8 MiB per stream.
    pub const MAX: Self = Self {
        timeout_seconds: bounds::MAX_TIMEOUT_SECONDS,
        stream_bytes: bounds::MAX_STREAM_BYTES,
    };

    /// Validates limits.
    ///
    /// # Errors
    /// Fails when a value is zero or above its hard maximum.
    pub fn new(timeout_seconds: u32, stream_bytes: u64) -> Result<Self> {
        if timeout_seconds == 0 || timeout_seconds > bounds::MAX_TIMEOUT_SECONDS {
            return Err(ModelError::new(
                "timeout",
                format!(
                    "must be between 1 and {} seconds",
                    bounds::MAX_TIMEOUT_SECONDS
                ),
            ));
        }
        if stream_bytes == 0 || stream_bytes > bounds::MAX_STREAM_BYTES {
            return Err(ModelError::new(
                "output limit",
                format!("must be between 1 and {} bytes", bounds::MAX_STREAM_BYTES),
            ));
        }
        Ok(Self {
            timeout_seconds,
            stream_bytes,
        })
    }

    /// The wall-clock timeout in seconds.
    #[must_use]
    pub const fn timeout_seconds(self) -> u32 {
        self.timeout_seconds
    }

    /// The captured bytes per output stream.
    #[must_use]
    pub const fn stream_bytes(self) -> u64 {
        self.stream_bytes
    }

    /// The stricter of two limit sets, field by field.
    #[must_use]
    pub fn lowered_by(self, other: Self) -> Self {
        Self {
            timeout_seconds: self.timeout_seconds.min(other.timeout_seconds),
            stream_bytes: self.stream_bytes.min(other.stream_bytes),
        }
    }
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

fn is_tag_rest(c: char) -> bool {
    is_lower_digit(c) || c == '_' || c == '-'
}

fn is_env_head(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_env_rest(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn check_binding_name(field: &'static str, name: &str) -> Result<()> {
    check_name(
        field,
        name,
        40,
        is_lower_digit,
        is_tag_rest,
        "[a-z0-9][a-z0-9_-]{0,39}",
    )
}

const SECRET_MARKERS: [&str; 5] = ["TOKEN", "SECRET", "PASSWORD", "KEY", "CREDENTIAL"];

fn check_env_name(name: &str) -> Result<()> {
    check_name(
        "environment variable",
        name,
        64,
        is_env_head,
        is_env_rest,
        "[A-Za-z_][A-Za-z0-9_]{0,63}",
    )?;
    let upper = name.to_ascii_uppercase();
    if SECRET_MARKERS.iter().any(|marker| upper.contains(marker)) {
        return Err(ModelError::new(
            "environment variable",
            format!("`{name}` looks like a secret and can never be passed to a run"),
        ));
    }
    Ok(())
}

fn check_program(program: &str) -> Result<()> {
    let field = "program";
    if program.is_empty() || program.len() > bounds::MAX_ARG_BYTES {
        return Err(ModelError::new(
            field,
            format!("must be 1 to {} bytes", bounds::MAX_ARG_BYTES),
        ));
    }
    if program.contains('\0') {
        return Err(ModelError::new(field, "must not contain NUL"));
    }
    let bytes = program.as_bytes();
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if program.starts_with('/') || program.starts_with('\\') || drive {
        return Err(ModelError::new(
            field,
            "must be a bare name or a repository-relative path, not an absolute path",
        ));
    }
    if program.split(['/', '\\']).any(|part| part == "..") {
        return Err(ModelError::new(field, "must not contain `..`"));
    }
    Ok(())
}

/// A declared workflow input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSpec {
    /// The binding name, `[a-z0-9][a-z0-9_-]{0,39}`.
    pub name: String,
    /// Whether a run must bind it.
    pub required: bool,
}

/// A declared workflow output, read from `outputs/<name>` in the run's work directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputSpec {
    /// The output name, `[a-z0-9][a-z0-9_-]{0,39}`.
    pub name: String,
    /// The type of the artifact that publishes it.
    pub artifact_type: ArtifactType,
    /// The content format of the published artifact.
    pub format: String,
    /// Whether the run fails when it is missing.
    pub required: bool,
}

/// The unvalidated parts of a workflow, as decoded from its artifact content.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowSpec {
    /// The provider name.
    pub provider: String,
    /// The program: a bare name or a repository-relative path.
    pub program: String,
    /// Literal arguments; no expansion, no globbing, no shell.
    pub args: Vec<String>,
    /// Declared inputs.
    pub inputs: Vec<InputSpec>,
    /// Declared outputs.
    pub outputs: Vec<OutputSpec>,
    /// Environment variable names passed through from the caller, if the grant lists them too.
    pub env_allow: Vec<String>,
    /// Literal environment values.
    pub env_set: BTreeMap<String, String>,
    /// Limits requested by the workflow; `None` keeps the defaults.
    pub limits: Option<ExecutionLimits>,
}

/// A validated workflow (ADR-0012): every bound of the contract holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workflow {
    spec: WorkflowSpec,
}

impl Workflow {
    /// Validates a workflow.
    ///
    /// # Errors
    /// Fails for an empty provider, a bad program, too many or oversized arguments, bad or
    /// duplicate input and output names, too many of either, a bad or secret-looking environment
    /// name, an environment name both allowed and set, or too many environment variables.
    pub fn new(spec: WorkflowSpec) -> Result<Self> {
        check_name(
            "provider",
            &spec.provider,
            32,
            is_lower,
            |c| is_lower_digit(c) || c == '-' || c == '_',
            "[a-z][a-z0-9_-]{0,31}",
        )?;
        check_program(&spec.program)?;
        if spec.args.len() > bounds::MAX_ARGS {
            return Err(ModelError::new(
                "args",
                format!("at most {} arguments", bounds::MAX_ARGS),
            ));
        }
        if spec
            .args
            .iter()
            .any(|arg| arg.len() > bounds::MAX_ARG_BYTES || arg.contains('\0'))
        {
            return Err(ModelError::new(
                "args",
                format!(
                    "each argument is at most {} bytes, without NUL",
                    bounds::MAX_ARG_BYTES
                ),
            ));
        }
        check_unique_names(
            "inputs",
            spec.inputs.iter().map(|i| i.name.as_str()),
            bounds::MAX_INPUTS,
        )?;
        check_unique_names(
            "outputs",
            spec.outputs.iter().map(|o| o.name.as_str()),
            bounds::MAX_OUTPUTS,
        )?;
        for output in &spec.outputs {
            crate::artifact::ContentFormat::new(&output.format)?;
        }
        if spec.env_allow.len() + spec.env_set.len() > bounds::MAX_ENV_VARS {
            return Err(ModelError::new(
                "env",
                format!("at most {} variables", bounds::MAX_ENV_VARS),
            ));
        }
        for name in spec.env_allow.iter().chain(spec.env_set.keys()) {
            check_env_name(name)?;
        }
        if let Some(name) = spec
            .env_allow
            .iter()
            .find(|n| spec.env_set.contains_key(*n))
        {
            return Err(ModelError::new(
                "env",
                format!("`{name}` is both allowed and set"),
            ));
        }
        let mut allowed: Vec<&String> = spec.env_allow.iter().collect();
        allowed.sort();
        if allowed.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ModelError::new("env", "duplicate allowed variable"));
        }
        Ok(Self { spec })
    }

    /// The validated parts, for encoding or for deriving a variant.
    #[must_use]
    pub fn spec(&self) -> &WorkflowSpec {
        &self.spec
    }

    /// The provider name.
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.spec.provider
    }

    /// The program.
    #[must_use]
    pub fn program(&self) -> &str {
        &self.spec.program
    }

    /// The literal arguments.
    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.spec.args
    }

    /// The declared inputs.
    #[must_use]
    pub fn inputs(&self) -> &[InputSpec] {
        &self.spec.inputs
    }

    /// The declared outputs.
    #[must_use]
    pub fn outputs(&self) -> &[OutputSpec] {
        &self.spec.outputs
    }

    /// Environment names passed through when the grant lists them too.
    #[must_use]
    pub fn env_allow(&self) -> &[String] {
        &self.spec.env_allow
    }

    /// Literal environment values.
    #[must_use]
    pub fn env_set(&self) -> &BTreeMap<String, String> {
        &self.spec.env_set
    }

    /// The limits requested by the workflow, never above the defaults.
    #[must_use]
    pub fn limits(&self) -> ExecutionLimits {
        self.spec.limits.map_or(ExecutionLimits::DEFAULT, |own| {
            own.lowered_by(ExecutionLimits::DEFAULT)
        })
    }

    /// Names of required outputs that are not in `produced`, in declaration order.
    #[must_use]
    pub fn missing_outputs(&self, produced: &[String]) -> Vec<String> {
        self.spec
            .outputs
            .iter()
            .filter(|output| output.required && !produced.contains(&output.name))
            .map(|output| output.name.clone())
            .collect()
    }
}

fn check_unique_names<'a>(
    field: &'static str,
    names: impl Iterator<Item = &'a str>,
    max: usize,
) -> Result<()> {
    let mut seen = Vec::new();
    for name in names {
        check_binding_name(field, name)?;
        if seen.contains(&name) {
            return Err(ModelError::new(
                field,
                format!("`{name}` is declared twice"),
            ));
        }
        seen.push(name);
    }
    if seen.len() > max {
        return Err(ModelError::new(field, format!("at most {max} allowed")));
    }
    Ok(())
}

/// A caller-chosen idempotency key for a run: `[a-z0-9][a-z0-9_-]{0,63}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestId(String);

impl RequestId {
    /// Validates the key.
    ///
    /// # Errors
    /// Fails when the grammar is violated.
    pub fn new(input: &str) -> Result<Self> {
        check_name(
            "request id",
            input,
            64,
            is_lower_digit,
            is_tag_rest,
            "[a-z0-9][a-z0-9_-]{0,63}",
        )?;
        Ok(Self(input.to_owned()))
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for RequestId {
    type Err = ModelError;

    fn from_str(input: &str) -> Result<Self> {
        Self::new(input)
    }
}

/// Binds a workflow input name to an artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBinding {
    /// The workflow input name.
    pub name: String,
    /// The artifact whose content is captured for it.
    pub artifact: ArtifactRef,
}

/// A request to execute a workflow. It carries no permission: the grant is separate (ADR-0012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRequest {
    workflow: ArtifactRef,
    bindings: Vec<InputBinding>,
    request_id: RequestId,
    validate_outputs: bool,
    limits: ExecutionLimits,
}

impl RunRequest {
    /// Builds a request.
    ///
    /// # Errors
    /// Fails for a bad or duplicate binding name or too many bindings.
    pub fn new(
        workflow: ArtifactRef,
        bindings: Vec<InputBinding>,
        request_id: RequestId,
    ) -> Result<Self> {
        check_unique_names(
            "inputs",
            bindings.iter().map(|b| b.name.as_str()),
            bounds::MAX_INPUTS,
        )?;
        Ok(Self {
            workflow,
            bindings,
            request_id,
            validate_outputs: false,
            limits: ExecutionLimits::MAX,
        })
    }

    /// Also validate and persist a report for every output (ADR-0015).
    #[must_use]
    pub fn with_output_validation(mut self) -> Self {
        self.validate_outputs = true;
        self
    }

    /// Lowers the limits for this run; the workflow's and the defaults still apply.
    #[must_use]
    pub fn with_limits(mut self, limits: ExecutionLimits) -> Self {
        self.limits = limits;
        self
    }

    /// The workflow to execute.
    #[must_use]
    pub fn workflow(&self) -> &ArtifactRef {
        &self.workflow
    }

    /// The input bindings.
    #[must_use]
    pub fn bindings(&self) -> &[InputBinding] {
        &self.bindings
    }

    /// The idempotency key.
    #[must_use]
    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    /// Whether outputs are validated and their reports persisted.
    #[must_use]
    pub fn validates_outputs(&self) -> bool {
        self.validate_outputs
    }

    /// The limits that apply to this run: the workflow's, lowered by the request's.
    #[must_use]
    pub fn effective_limits(&self, workflow: &Workflow) -> ExecutionLimits {
        workflow.limits().lowered_by(self.limits)
    }

    /// Checks the bindings against the workflow's declared inputs.
    ///
    /// # Errors
    /// Fails for a binding no input declares, or a required input left unbound.
    pub fn check_bindings(&self, workflow: &Workflow) -> Result<()> {
        for binding in &self.bindings {
            if !workflow.inputs().iter().any(|i| i.name == binding.name) {
                return Err(ModelError::new(
                    "inputs",
                    format!("the workflow declares no input `{}`", binding.name),
                ));
            }
        }
        for input in workflow.inputs().iter().filter(|i| i.required) {
            if !self.bindings.iter().any(|b| b.name == input.name) {
                return Err(ModelError::new(
                    "inputs",
                    format!("required input `{}` is not bound", input.name),
                ));
            }
        }
        Ok(())
    }
}

/// Provenance of the program that ran: the resolved path and, when it could be read, its digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramProvenance {
    /// The resolved path, as the provider saw it.
    pub path: String,
    /// The SHA-256 of the program file; `None` when it could not be read.
    pub digest: Option<ContentDigest>,
}

/// The run intent: everything decided and captured before any side effect (ADR-0013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunIntent {
    /// The idempotency key.
    pub request_id: RequestId,
    /// The provider name.
    pub provider: String,
    /// The provider version.
    pub provider_version: String,
    /// The `capture` artifact of the workflow.
    pub workflow: ArtifactId,
    /// The `capture` artifact of every bound input, by name.
    pub inputs: BTreeMap<String, ArtifactId>,
    /// The literal arguments.
    pub args: Vec<String>,
    /// The limits that applied.
    pub limits: ExecutionLimits,
    /// Names, never values, of the environment variables the child could see.
    pub env_names: Vec<String>,
    /// The program that was resolved.
    pub program: ProgramProvenance,
    /// The platform, for example `windows-x86_64`.
    pub platform: String,
    /// When the intent was recorded.
    pub started: Timestamp,
}

/// Truncation of captured output streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Truncation {
    /// Whether stdout was cut at the limit.
    pub stdout: bool,
    /// Whether stderr was cut at the limit.
    pub stderr: bool,
}

impl Truncation {
    /// Whether either stream was truncated.
    #[must_use]
    pub const fn any(self) -> bool {
        self.stdout || self.stderr
    }
}

/// The unvalidated parts of a run result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResultSpec {
    /// The outcome.
    pub outcome: RunOutcome,
    /// When the run reached the outcome.
    pub finished: Timestamp,
    /// The child exit status; only for `Succeeded` and `Failed`.
    pub exit_code: Option<i32>,
    /// Output truncation.
    pub truncated: Truncation,
    /// The published output artifacts; only for `Succeeded`.
    pub outputs: Vec<ArtifactId>,
    /// Human-readable reasons; required for `StartFailed` and for a `Failed` run that exited 0.
    pub diagnostics: Vec<String>,
}

/// The terminal record of a run. Its existence is the commit point; the constructor makes
/// inconsistent records, above all a false success, unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    run: ArtifactId,
    spec: RunResultSpec,
}

impl RunResult {
    /// Builds the result of `run`, which started at `started`.
    ///
    /// # Errors
    /// Fails when the record is inconsistent: `finished` before `started`; `Succeeded` without
    /// exit status 0 or with failure diagnostics; an exit status on `TimedOut`, `Cancelled`,
    /// `StartFailed` or `Interrupted`; outputs on anything but `Succeeded`; a `Failed` run that
    /// exited 0 without a diagnostic; `StartFailed` without one.
    pub fn new(run: ArtifactId, started: Timestamp, spec: RunResultSpec) -> Result<Self> {
        let field = "run result";
        let bad = |reason: &str| Err(ModelError::new(field, reason));
        if spec.finished < started {
            return bad("finished must not be before started");
        }
        if spec.outputs.len() > bounds::MAX_OUTPUTS {
            return bad("too many outputs");
        }
        match spec.outcome {
            RunOutcome::Succeeded => {
                if spec.exit_code != Some(0) {
                    return bad("a succeeded run must have exit status 0");
                }
                if !spec.diagnostics.is_empty() {
                    return bad("a succeeded run must not carry failure diagnostics");
                }
            }
            RunOutcome::Failed => {
                if spec.exit_code == Some(0) && spec.diagnostics.is_empty() {
                    return bad("a failed run that exited 0 must say why");
                }
            }
            RunOutcome::StartFailed => {
                if spec.diagnostics.is_empty() {
                    return bad("a start failure must say why");
                }
            }
            RunOutcome::TimedOut | RunOutcome::Cancelled | RunOutcome::Interrupted => {}
        }
        if !matches!(spec.outcome, RunOutcome::Succeeded | RunOutcome::Failed)
            && spec.exit_code.is_some()
        {
            return bad("only succeeded and failed runs have an exit status");
        }
        if !spec.outcome.is_success() && !spec.outputs.is_empty() {
            return bad("only a succeeded run publishes outputs");
        }
        Ok(Self { run, spec })
    }

    /// The `run` artifact this result belongs to.
    #[must_use]
    pub fn run(&self) -> &ArtifactId {
        &self.run
    }

    /// The outcome.
    #[must_use]
    pub fn outcome(&self) -> RunOutcome {
        self.spec.outcome
    }

    /// When the run reached the outcome.
    #[must_use]
    pub fn finished(&self) -> Timestamp {
        self.spec.finished
    }

    /// The child exit status, if any.
    #[must_use]
    pub fn exit_code(&self) -> Option<i32> {
        self.spec.exit_code
    }

    /// Output truncation.
    #[must_use]
    pub fn truncated(&self) -> Truncation {
        self.spec.truncated
    }

    /// The published output artifacts.
    #[must_use]
    pub fn outputs(&self) -> &[ArtifactId] {
        &self.spec.outputs
    }

    /// Human-readable reasons.
    #[must_use]
    pub fn diagnostics(&self) -> &[String] {
        &self.spec.diagnostics
    }
}

/// A run as stored: its intent and, once complete, its single result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    run: ArtifactId,
    intent: RunIntent,
    result: Option<(ArtifactId, RunResult)>,
}

impl RunRecord {
    /// A run whose intent is recorded and which has no result yet.
    #[must_use]
    pub fn new(run: ArtifactId, intent: RunIntent) -> Self {
        Self {
            run,
            intent,
            result: None,
        }
    }

    /// Attaches the terminal result. This is the only transition, and it happens once.
    ///
    /// # Errors
    /// Fails when the run already has a result or the result belongs to another run. A completed
    /// run therefore can never be turned into a different outcome.
    pub fn complete(self, result_artifact: ArtifactId, result: RunResult) -> Result<Self> {
        if self.result.is_some() {
            return Err(ModelError::new("run", "already has a result"));
        }
        if result.run() != &self.run {
            return Err(ModelError::new("run result", "belongs to another run"));
        }
        if result.finished() < self.intent.started {
            return Err(ModelError::new(
                "run result",
                "finished must not be before the run started",
            ));
        }
        Ok(Self {
            result: Some((result_artifact, result)),
            ..self
        })
    }

    /// The `run` artifact.
    #[must_use]
    pub fn run(&self) -> &ArtifactId {
        &self.run
    }

    /// The intent.
    #[must_use]
    pub fn intent(&self) -> &RunIntent {
        &self.intent
    }

    /// The `run_result` artifact and its value, once complete.
    #[must_use]
    pub fn result(&self) -> Option<&(ArtifactId, RunResult)> {
        self.result.as_ref()
    }

    /// The derived status; `lease_held` tells whether a live process owns the per-run lock.
    #[must_use]
    pub fn status(&self, lease_held: bool) -> RunStatus {
        match &self.result {
            Some((_, result)) => RunStatus::Completed(result.outcome()),
            None if lease_held => RunStatus::Running,
            None => RunStatus::InterruptedUnknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{Content, Metadata, NewArtifact, Title};
    use crate::id::Slug;

    fn ts(seconds: i64) -> Timestamp {
        Timestamp::from_unix_seconds(1_700_000_000 + seconds).unwrap()
    }

    fn id(n: u128) -> ArtifactId {
        ArtifactId::from_bits(n)
    }

    fn spec() -> WorkflowSpec {
        WorkflowSpec {
            provider: LOCAL_PROCESS_PROVIDER.to_owned(),
            program: "python".to_owned(),
            args: vec!["scripts/x.py".to_owned()],
            inputs: vec![InputSpec {
                name: "data".to_owned(),
                required: true,
            }],
            outputs: vec![OutputSpec {
                name: "summary".to_owned(),
                artifact_type: ArtifactType::new("dataset").unwrap(),
                format: "text".to_owned(),
                required: true,
            }],
            ..WorkflowSpec::default()
        }
    }

    fn intent() -> RunIntent {
        RunIntent {
            request_id: RequestId::new("r1").unwrap(),
            provider: LOCAL_PROCESS_PROVIDER.to_owned(),
            provider_version: "1".to_owned(),
            workflow: id(2),
            inputs: BTreeMap::from([("data".to_owned(), id(3))]),
            args: vec![],
            limits: ExecutionLimits::DEFAULT,
            env_names: vec![],
            program: ProgramProvenance {
                path: "python".to_owned(),
                digest: None,
            },
            platform: "test".to_owned(),
            started: ts(0),
        }
    }

    fn result_spec(outcome: RunOutcome) -> RunResultSpec {
        RunResultSpec {
            outcome,
            finished: ts(5),
            exit_code: None,
            truncated: Truncation::default(),
            outputs: vec![],
            diagnostics: vec![],
        }
    }

    #[test]
    fn evidence_kind_names_are_valid_artifact_types() {
        for kind in EvidenceKind::ALL {
            assert_eq!(kind.to_artifact_type().as_str(), kind.artifact_type());
            assert!(Slug::parse(&format!("{}x", kind.slug_prefix())).is_ok());
        }
    }

    #[test]
    fn evidence_needs_both_the_type_and_the_marker() {
        let now = ts(0);
        let make = |artifact_type: &str, marker: Option<&str>| {
            let mut metadata = Metadata::new();
            if let Some(marker) = marker {
                metadata.insert(
                    MetadataKey::new(EVIDENCE_KEY).unwrap(),
                    MetadataValue::from(marker),
                );
            }
            Artifact::create(
                id(1),
                NewArtifact::new(
                    Slug::parse("x").unwrap(),
                    ArtifactType::new(artifact_type).unwrap(),
                    Title::new("x").unwrap(),
                )
                .with_metadata(metadata)
                .with_content(Content::default()),
                now,
            )
        };
        for kind in EvidenceKind::ALL {
            let artifact = make(kind.artifact_type(), Some(kind.schema()));
            assert_eq!(EvidenceKind::of(&artifact), Some(kind));
            assert_eq!(EvidenceKind::of(&make(kind.artifact_type(), None)), None);
            assert_eq!(EvidenceKind::of(&make("note", Some(kind.schema()))), None);
        }
        assert_eq!(
            EvidenceKind::of(&make("run", Some("functor.capture/1"))),
            None,
            "type and schema must agree"
        );
    }

    #[test]
    fn digests_have_one_text_form() {
        let digest = ContentDigest::from_bytes([0xab; 32]);
        assert_eq!(digest.as_str(), format!("sha256:{}", "ab".repeat(32)));
        assert_eq!(digest.hex(), "ab".repeat(32));
        assert_eq!(ContentDigest::new(digest.as_str()).unwrap(), digest);
        for bad in [
            "",
            "sha256:",
            "sha1:abcd",
            &format!("sha256:{}", "AB".repeat(32)),
            &format!("sha256:{}", "ab".repeat(31)),
            &format!("sha256:{}g", "a".repeat(63)),
        ] {
            assert!(ContentDigest::new(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn limits_stay_within_the_hard_maximums() {
        assert!(ExecutionLimits::new(1, 1).is_ok());
        assert!(ExecutionLimits::new(3600, 8 * 1024 * 1024).is_ok());
        assert!(ExecutionLimits::new(0, 1).is_err());
        assert!(ExecutionLimits::new(3601, 1).is_err());
        assert!(ExecutionLimits::new(1, 0).is_err());
        assert!(ExecutionLimits::new(1, 8 * 1024 * 1024 + 1).is_err());
        let low = ExecutionLimits::new(5, 10).unwrap();
        let lowered = ExecutionLimits::DEFAULT.lowered_by(low);
        assert_eq!((lowered.timeout_seconds(), lowered.stream_bytes()), (5, 10));
        let still = low.lowered_by(ExecutionLimits::MAX);
        assert_eq!(still, low, "a higher limit never raises a lower one");
    }

    #[test]
    fn a_workflow_may_lower_but_not_raise_the_defaults() {
        let mut raised = spec();
        raised.limits = Some(ExecutionLimits::MAX);
        assert_eq!(
            Workflow::new(raised).unwrap().limits(),
            ExecutionLimits::DEFAULT
        );
        let mut lowered = spec();
        lowered.limits = Some(ExecutionLimits::new(5, 100).unwrap());
        assert_eq!(
            Workflow::new(lowered).unwrap().limits(),
            ExecutionLimits::new(5, 100).unwrap()
        );
    }

    #[test]
    fn workflows_enforce_the_contract_bounds() {
        assert!(Workflow::new(spec()).is_ok());
        let reject = |change: fn(&mut WorkflowSpec)| {
            let mut s = spec();
            change(&mut s);
            assert!(Workflow::new(s).is_err());
        };
        reject(|s| s.provider = String::new());
        reject(|s| s.provider = "Local".to_owned());
        reject(|s| s.program = String::new());
        reject(|s| s.program = "/usr/bin/python".to_owned());
        reject(|s| s.program = "C:\\py\\python.exe".to_owned());
        reject(|s| s.program = "\\\\host\\share\\p.exe".to_owned());
        reject(|s| s.program = "tools/../../x".to_owned());
        reject(|s| s.program = "a\0b".to_owned());
        reject(|s| s.args = vec!["x".to_owned(); bounds::MAX_ARGS + 1]);
        reject(|s| s.args = vec!["x".repeat(bounds::MAX_ARG_BYTES + 1)]);
        reject(|s| s.args = vec!["a\0".to_owned()]);
        reject(|s| s.inputs.push(s.inputs[0].clone()));
        reject(|s| s.inputs[0].name = "Bad Name".to_owned());
        reject(|s| s.outputs[0].format = "Bad Format".to_owned());
        reject(|s| {
            s.inputs = (0..=bounds::MAX_INPUTS)
                .map(|n| InputSpec {
                    name: format!("i{n}"),
                    required: false,
                })
                .collect();
        });
        reject(|s| {
            s.outputs = (0..=bounds::MAX_OUTPUTS)
                .map(|n| OutputSpec {
                    name: format!("o{n}"),
                    artifact_type: ArtifactType::document(),
                    format: "text".to_owned(),
                    required: false,
                })
                .collect();
        });
    }

    #[test]
    fn workflow_bounds_hold_exactly_at_the_limit() {
        let mut at_limit = spec();
        at_limit.args = vec!["x".repeat(bounds::MAX_ARG_BYTES); bounds::MAX_ARGS];
        at_limit.inputs = (0..bounds::MAX_INPUTS)
            .map(|n| InputSpec {
                name: format!("i{n}"),
                required: false,
            })
            .collect();
        at_limit.outputs = (0..bounds::MAX_OUTPUTS)
            .map(|n| OutputSpec {
                name: format!("o{n}"),
                artifact_type: ArtifactType::document(),
                format: "text".to_owned(),
                required: false,
            })
            .collect();
        at_limit.env_set = (0..bounds::MAX_ENV_VARS)
            .map(|n| (format!("V{n}"), "x".to_owned()))
            .collect();
        assert!(Workflow::new(at_limit.clone()).is_ok());
        at_limit
            .env_set
            .insert("ONE_MORE".to_owned(), "x".to_owned());
        assert!(Workflow::new(at_limit).is_err());
    }

    #[test]
    fn environment_rules_refuse_secrets_and_conflicts() {
        let with_env = |allow: &[&str], set: &[(&str, &str)]| {
            let mut s = spec();
            s.env_allow = allow.iter().map(|n| (*n).to_owned()).collect();
            s.env_set = set
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect();
            Workflow::new(s)
        };
        assert!(with_env(&["LANG"], &[("MODE", "fast")]).is_ok());
        for secret in [
            "API_TOKEN",
            "my_secret",
            "DB_PASSWORD",
            "SSH_KEY",
            "AwsCredentials",
        ] {
            assert!(with_env(&[secret], &[]).is_err(), "{secret}");
            assert!(with_env(&[], &[(secret, "x")]).is_err(), "{secret}");
        }
        assert!(with_env(&["LANG"], &[("LANG", "C")]).is_err());
        assert!(with_env(&["LANG", "LANG"], &[]).is_err());
        assert!(with_env(&["1BAD"], &[]).is_err());
    }

    #[test]
    fn missing_required_outputs_are_listed_in_declaration_order() {
        let mut s = spec();
        s.outputs.push(OutputSpec {
            name: "extra".to_owned(),
            artifact_type: ArtifactType::document(),
            format: "text".to_owned(),
            required: false,
        });
        let workflow = Workflow::new(s).unwrap();
        assert_eq!(workflow.missing_outputs(&[]), ["summary"]);
        assert_eq!(
            workflow.missing_outputs(&["summary".to_owned()]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn bindings_must_match_the_declared_inputs() {
        let workflow = Workflow::new(spec()).unwrap();
        let request = |names: &[&str]| {
            RunRequest::new(
                ArtifactRef::parse("wf").unwrap(),
                names
                    .iter()
                    .map(|n| InputBinding {
                        name: (*n).to_owned(),
                        artifact: ArtifactRef::parse("input").unwrap(),
                    })
                    .collect(),
                RequestId::new("r1").unwrap(),
            )
        };
        assert!(
            request(&["data"])
                .unwrap()
                .check_bindings(&workflow)
                .is_ok()
        );
        assert!(
            request(&[]).unwrap().check_bindings(&workflow).is_err(),
            "unbound"
        );
        assert!(
            request(&["data", "other"])
                .unwrap()
                .check_bindings(&workflow)
                .is_err(),
            "unknown"
        );
        assert!(request(&["data", "data"]).is_err(), "duplicate");
        assert!(request(&["Bad Name"]).is_err());
    }

    #[test]
    fn request_limits_only_ever_lower() {
        let workflow = Workflow::new(spec()).unwrap();
        let base = RunRequest::new(
            ArtifactRef::parse("wf").unwrap(),
            vec![],
            RequestId::new("r1").unwrap(),
        )
        .unwrap();
        assert_eq!(base.effective_limits(&workflow), ExecutionLimits::DEFAULT);
        let lowered = base.with_limits(ExecutionLimits::new(3, 7).unwrap());
        let effective = lowered.effective_limits(&workflow);
        assert_eq!(
            (effective.timeout_seconds(), effective.stream_bytes()),
            (3, 7)
        );
    }

    #[test]
    fn request_ids_follow_their_grammar() {
        assert!(RequestId::new("run-1_a").is_ok());
        assert!(RequestId::new(&"a".repeat(64)).is_ok());
        for bad in ["", "-a", "A", "a b", &"a".repeat(65)] {
            assert!(RequestId::new(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn outcome_text_round_trips() {
        for outcome in RunOutcome::ALL {
            assert_eq!(RunOutcome::parse(outcome.as_str()).unwrap(), outcome);
            assert_eq!(outcome.is_success(), outcome == RunOutcome::Succeeded);
        }
        assert!(RunOutcome::parse("ok").is_err());
    }

    #[test]
    fn a_success_must_be_a_real_success() {
        let run = id(10);
        let make = |change: fn(&mut RunResultSpec)| {
            let mut s = result_spec(RunOutcome::Succeeded);
            s.exit_code = Some(0);
            change(&mut s);
            RunResult::new(run, ts(0), s)
        };
        assert!(make(|_| {}).is_ok());
        assert!(make(|s| s.exit_code = Some(1)).is_err());
        assert!(make(|s| s.exit_code = None).is_err());
        assert!(make(|s| s.diagnostics = vec!["output missing".to_owned()]).is_err());
        assert!(make(|s| s.finished = ts(-1)).is_err());
        assert!(
            make(|s| s.truncated.stdout = true).is_ok(),
            "truncation is not a failure"
        );
        assert!(make(|s| s.outputs = vec![id(11)]).is_ok());
    }

    #[test]
    fn non_success_outcomes_have_consistent_records() {
        let run = id(10);
        let build = |outcome: RunOutcome, change: fn(&mut RunResultSpec)| {
            let mut s = result_spec(outcome);
            change(&mut s);
            RunResult::new(run, ts(0), s)
        };
        assert!(build(RunOutcome::Failed, |s| s.exit_code = Some(2)).is_ok());
        assert!(build(RunOutcome::Failed, |s| s.exit_code = Some(0)).is_err());
        assert!(
            build(RunOutcome::Failed, |s| {
                s.exit_code = Some(0);
                s.diagnostics = vec!["required output `summary` is missing".to_owned()];
            })
            .is_ok()
        );
        assert!(build(RunOutcome::Failed, |s| s.outputs = vec![id(1)]).is_err());
        for outcome in [
            RunOutcome::TimedOut,
            RunOutcome::Cancelled,
            RunOutcome::Interrupted,
        ] {
            assert!(build(outcome, |_| {}).is_ok(), "{outcome}");
            assert!(
                build(outcome, |s| s.exit_code = Some(1)).is_err(),
                "{outcome}"
            );
            assert!(
                build(outcome, |s| s.outputs = vec![id(1)]).is_err(),
                "{outcome}"
            );
        }
        assert!(build(RunOutcome::StartFailed, |_| {}).is_err());
        assert!(
            build(RunOutcome::StartFailed, |s| s.diagnostics =
                vec!["program not found".to_owned()])
            .is_ok()
        );
    }

    #[test]
    fn a_run_completes_exactly_once_and_only_with_its_own_result() {
        let run = id(10);
        let record = RunRecord::new(run, intent());
        assert_eq!(record.status(true), RunStatus::Running);
        assert_eq!(record.status(false), RunStatus::InterruptedUnknown);

        let mut ok = result_spec(RunOutcome::Succeeded);
        ok.exit_code = Some(0);
        let result = RunResult::new(run, ts(0), ok).unwrap();

        let foreign = RunResult::new(id(99), ts(0), {
            let mut s = result_spec(RunOutcome::Succeeded);
            s.exit_code = Some(0);
            s
        })
        .unwrap();
        assert!(record.clone().complete(id(20), foreign).is_err());

        let earlier = RunResult::new(run, ts(-10), {
            let mut s = result_spec(RunOutcome::Cancelled);
            s.finished = ts(-5);
            s
        })
        .unwrap();
        assert!(
            record.clone().complete(id(20), earlier).is_err(),
            "a result cannot predate the intent"
        );

        let done = record.complete(id(20), result.clone()).unwrap();
        assert_eq!(
            done.status(true),
            RunStatus::Completed(RunOutcome::Succeeded)
        );
        assert_eq!(
            done.status(false),
            RunStatus::Completed(RunOutcome::Succeeded)
        );
        let cancelled = RunResult::new(run, ts(0), result_spec(RunOutcome::Cancelled)).unwrap();
        assert!(
            done.complete(id(21), cancelled).is_err(),
            "a completed run cannot change outcome"
        );
    }
}
