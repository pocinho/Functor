//! Shared test support: fakes for the orchestrator's collaborators and a harness that wires them.

// Support code asserts and unwraps freely, and not every test file uses every helper.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::unused_self
)]

use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use functor_core::{
    ArtifactRepository, CancelToken, CapturedBytes, Clock, EventPublisher, EvidenceRecorder,
    ExecutionGrant, ExecutionPlan, ExecutionProvider, FunctorError, IntentRecord, ReportRecorder,
    ReportSubject, Result, RunFinish, RunLease, RunStreams, ValidationEngine, ValidationTarget,
};
use functor_model::{
    ArtifactId, ArtifactRef, ArtifactType, CaptureRole, Content, ContentFormat, Event, EventId,
    ExecutionLimits, Finding, InputBinding, NewArtifact, NewEvent, Outcome, ProgramProvenance,
    RepositoryId, RequestId, RuleCode, RunIntent, RunRecord, RunRequest, RunStatus, Severity, Slug,
    Subject, Timestamp, Title, ValidationReport, Workflow,
};
use functor_runtime::{RunFailure, RunOrchestrator, RunReport};
use functor_store::InMemoryEvidence;
use functor_testkit::{Env, FakeProvider};

/// A workflow the fake provider understands: input `data`, required output `result`.
pub const WORKFLOW: &str = r#"
schema = "functor.workflow/1"
provider = "fake"
program = "fake-tool"
args = ["succeed"]

[inputs]
data = { required = true }

[outputs]
result = { type = "dataset", format = "text", required = true }

[env]
allow = ["LANG"]
"#;

/// Counts the dash-separated parts of a text.
pub fn tokens(text: &str) -> usize {
    text.split('-').count()
}

/// A request to run the standard workflow and input by slug.
pub fn request(id: &str) -> RunRequest {
    RunRequest::new(
        ArtifactRef::Slug(Slug::parse("workflow").unwrap()),
        vec![InputBinding {
            name: "data".to_owned(),
            artifact: ArtifactRef::Slug(Slug::parse("input").unwrap()),
        }],
        RequestId::new(id).unwrap(),
    )
    .unwrap()
}

fn injected(what: &str) -> FunctorError {
    FunctorError::storage(
        "write",
        PathBuf::from(what),
        io::Error::other(format!("injected failure: {what}")),
    )
}

fn at() -> Timestamp {
    Timestamp::parse("2026-10-12T18:00:00Z").unwrap()
}

// ---------------------------------------------------------------------------------------------
// events
// ---------------------------------------------------------------------------------------------

/// An event publisher that remembers what it was given and can be made to fail.
#[derive(Default)]
pub struct Recording {
    events: Mutex<Vec<NewEvent>>,
    seq: AtomicU64,
    pub fail: AtomicBool,
}

impl Recording {
    /// Everything published so far, in order.
    pub fn taken(&self) -> Vec<NewEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl EventPublisher for Recording {
    async fn publish(&self, event: NewEvent) -> Result<Event> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(injected("event log"));
        }
        self.events.lock().unwrap().push(event.clone());
        let seq = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(Event::new(
            EventId::from_bits(u128::from(seq)),
            seq,
            at(),
            RepositoryId::from_bits(1),
            event,
        )?)
    }
}

// ---------------------------------------------------------------------------------------------
// provider
// ---------------------------------------------------------------------------------------------

/// The testkit's fake provider plus counters and a switch to make execution fail.
pub struct CountingProvider {
    pub inner: FakeProvider,
    pub discards: AtomicUsize,
    pub fail_execute: AtomicBool,
}

impl CountingProvider {
    pub fn new() -> Self {
        Self {
            inner: FakeProvider::new(),
            discards: AtomicUsize::new(0),
            fail_execute: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl ExecutionProvider for CountingProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn version(&self) -> &str {
        self.inner.version()
    }

    async fn preflight(
        &self,
        workflow: &Workflow,
        limits: ExecutionLimits,
        grant: &ExecutionGrant,
    ) -> Result<ProgramProvenance> {
        self.inner.preflight(workflow, limits, grant).await
    }

    fn env_names(&self, workflow: &Workflow) -> Vec<String> {
        self.inner.env_names(workflow)
    }

    async fn discard_staging(&self, _run: &ArtifactId) -> Result<()> {
        self.discards.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn execute(
        &self,
        plan: ExecutionPlan,
        cancel: CancelToken,
    ) -> Result<functor_core::ProviderRun> {
        if self.fail_execute.load(Ordering::SeqCst) {
            return Err(injected("staging"));
        }
        self.inner.execute(plan, cancel).await
    }
}

// ---------------------------------------------------------------------------------------------
// recorder
// ---------------------------------------------------------------------------------------------

/// Delegates to the in-memory evidence, failing the selected writes.
pub struct FailingRecorder {
    pub inner: Arc<InMemoryEvidence>,
    pub fail_intent: AtomicBool,
    pub fail_result: AtomicBool,
    pub fail_reports: AtomicBool,
}

#[async_trait]
impl EvidenceRecorder for FailingRecorder {
    async fn capture(&self, source: &ArtifactRef, role: CaptureRole) -> Result<CapturedBytes> {
        self.inner.capture(source, role).await
    }

    async fn record_intent(&self, intent: RunIntent) -> Result<IntentRecord> {
        if self.fail_intent.load(Ordering::SeqCst) {
            return Err(injected("run intent"));
        }
        self.inner.record_intent(intent).await
    }

    async fn acquire_lease(&self, run: &ArtifactId) -> Result<Box<dyn RunLease>> {
        self.inner.acquire_lease(run).await
    }

    async fn record_result(&self, run: &ArtifactId, finish: RunFinish) -> Result<RunRecord> {
        if self.fail_result.load(Ordering::SeqCst) {
            return Err(injected("run result"));
        }
        self.inner.record_result(run, finish).await
    }

    async fn run(&self, run: &ArtifactRef) -> Result<Option<RunRecord>> {
        self.inner.run(run).await
    }

    async fn runs(&self) -> Result<Vec<RunRecord>> {
        self.inner.runs().await
    }

    async fn status(&self, run: &ArtifactId) -> Result<RunStatus> {
        self.inner.status(run).await
    }

    async fn streams(&self, run: &ArtifactId) -> Result<Option<RunStreams>> {
        self.inner.streams(run).await
    }

    async fn read_capture(&self, capture: &ArtifactId) -> Result<Vec<u8>> {
        self.inner.read_capture(capture).await
    }

    async fn recover(&self) -> Result<Vec<RunRecord>> {
        self.inner.recover().await
    }
}

#[async_trait]
impl ReportRecorder for FailingRecorder {
    async fn save_report(
        &self,
        report: &ValidationReport,
        subject: ReportSubject,
        strict: bool,
        rules: &str,
    ) -> Result<ArtifactId> {
        if self.fail_reports.load(Ordering::SeqCst) {
            return Err(injected("report"));
        }
        self.inner.save_report(report, subject, strict, rules).await
    }

    async fn load_report(
        &self,
        id: &ArtifactId,
    ) -> Result<Option<(ValidationReport, ReportSubject)>> {
        self.inner.load_report(id).await
    }
}

// ---------------------------------------------------------------------------------------------
// validation
// ---------------------------------------------------------------------------------------------

/// A validation engine that returns a fixed outcome and remembers what it was asked about.
pub struct StubEngine {
    outcome: Outcome,
    targets: Mutex<Vec<ArtifactRef>>,
}

impl StubEngine {
    pub fn new(outcome: Outcome) -> Self {
        Self {
            outcome,
            targets: Mutex::new(Vec::new()),
        }
    }

    pub fn targets(&self) -> Vec<ArtifactRef> {
        self.targets.lock().unwrap().clone()
    }
}

#[async_trait]
impl ValidationEngine for StubEngine {
    async fn validate(&self, target: ValidationTarget) -> Result<ValidationReport> {
        if let ValidationTarget::Artifact(reference) = target {
            self.targets.lock().unwrap().push(reference);
        }
        let finding = |severity| {
            vec![Finding::new(
                RuleCode::new("V0012"),
                severity,
                Subject::Repository,
                "a stub finding",
            )]
        };
        let (findings, incomplete) = match self.outcome {
            Outcome::Warning => (finding(Severity::Warning), false),
            Outcome::Failed => (finding(Severity::Error), false),
            Outcome::Unknown => (Vec::new(), true),
            _ => (Vec::new(), false),
        };
        Ok(ValidationReport::new(findings, 2, 1, incomplete, at()))
    }
}

// ---------------------------------------------------------------------------------------------
// harness
// ---------------------------------------------------------------------------------------------

/// A fake provider, in-memory evidence, a recording publisher and an orchestrator wired to them,
/// with a standard workflow and input already in the repository.
pub struct Fast {
    pub env: Env,
    pub evidence: Arc<InMemoryEvidence>,
    pub recorder: Arc<FailingRecorder>,
    pub provider: Arc<CountingProvider>,
    pub events: Arc<Recording>,
    pub orchestrator: RunOrchestrator,
    pub workflow: ArtifactId,
    pub input: ArtifactId,
}

impl Fast {
    pub async fn new() -> Self {
        Self::with_workflow(WORKFLOW).await
    }

    pub async fn with_workflow(workflow_text: &str) -> Self {
        let env = Env::new();
        let evidence = Arc::new(InMemoryEvidence::in_memory(env.clock(), env.ids()));
        let recorder = Arc::new(FailingRecorder {
            inner: Arc::clone(&evidence),
            fail_intent: AtomicBool::new(false),
            fail_result: AtomicBool::new(false),
            fail_reports: AtomicBool::new(false),
        });
        let provider = Arc::new(CountingProvider::new());
        let events = Arc::new(Recording::default());
        let orchestrator = RunOrchestrator::new(
            recorder.clone(),
            recorder.clone(),
            provider.clone(),
            env.clock(),
        )
        .with_events(events.clone());
        let mut fast = Self {
            env,
            evidence,
            recorder,
            provider,
            events,
            orchestrator,
            workflow: ArtifactId::from_bits(0),
            input: ArtifactId::from_bits(0),
        };
        fast.workflow = fast
            .typed_source("workflow", "workflow", "toml", workflow_text)
            .await;
        fast.input = fast.source("input", "text", "hello").await;
        fast
    }

    pub fn store(&self) -> &functor_store::MemoryStore {
        self.evidence.store()
    }

    pub fn clock(&self) -> Arc<dyn Clock> {
        self.env.clock()
    }

    pub fn grant(&self) -> ExecutionGrant {
        ExecutionGrant::new("fake-tool", ExecutionLimits::DEFAULT).with_env("LANG")
    }

    async fn typed_source(&self, slug: &str, kind: &str, format: &str, text: &str) -> ArtifactId {
        let new = NewArtifact::new(
            Slug::parse(slug).unwrap(),
            ArtifactType::new(kind).unwrap(),
            Title::new(slug).unwrap(),
        )
        .with_content(Content::Inline {
            format: ContentFormat::new(format).unwrap(),
            text: text.to_owned(),
        });
        *ArtifactRepository::create(self.store(), new)
            .await
            .unwrap()
            .id()
    }

    /// Creates a document artifact with inline content.
    pub async fn source(&self, slug: &str, format: &str, text: &str) -> ArtifactId {
        self.typed_source(slug, "document", format, text).await
    }

    pub async fn run(
        &self,
        request: &RunRequest,
        grant: ExecutionGrant,
    ) -> std::result::Result<RunReport, Box<RunFailure>> {
        self.orchestrator
            .execute(request, grant, CancelToken::new(), None)
            .await
    }

    pub async fn report(&self, id: &ArtifactId) -> (ValidationReport, ReportSubject) {
        self.evidence.load_report(id).await.unwrap().unwrap()
    }
}
