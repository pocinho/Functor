//! Persistence contracts for execution and validation evidence (ADR-0013, ADR-0015).
//!
//! Evidence is stored as immutable artifacts, so it stays inspectable through the ordinary
//! artifact and query APIs. These traits add what those cannot express: consistent capture,
//! idempotent run intent, a per-run lease, the single commit point of a run, and recovery.
//! Persisting is separate from running: a recorder never starts a process.

use async_trait::async_trait;
use functor_model::{
    ArtifactId, ArtifactRef, Capture, CaptureRole, ContentDigest, OutputSpec, Revision, RunIntent,
    RunOutcome, RunRecord, RunStatus, Truncation, ValidationReport,
};

use crate::error::Result;

/// A capture and the exact bytes it describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedBytes {
    /// The recorded capture.
    pub capture: Capture,
    /// The captured bytes, which hash to `capture.digest`.
    pub bytes: Vec<u8>,
}

/// The answer to recording a run intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentRecord {
    /// The run, new or existing.
    pub record: RunRecord,
    /// `false` when the `request_id` was already recorded: nothing was written and the caller must
    /// not start another child.
    pub created: bool,
}

/// One declared output ready to be published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedOutput {
    /// The declaration the output satisfies.
    pub spec: OutputSpec,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// What the orchestrator knows when a run ends; the recorder turns it into evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFinish {
    /// The outcome.
    pub outcome: RunOutcome,
    /// The child exit status, for `Succeeded` and `Failed` only.
    pub exit_code: Option<i32>,
    /// Which streams were truncated.
    pub truncated: Truncation,
    /// Reasons for a failure.
    pub diagnostics: Vec<String>,
    /// Captured standard output.
    pub stdout: Vec<u8>,
    /// Captured standard error.
    pub stderr: Vec<u8>,
    /// Outputs to publish; only for `Succeeded`.
    pub outputs: Vec<PublishedOutput>,
}

/// The stored standard streams of a finished run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStreams {
    /// Standard output.
    pub stdout: Vec<u8>,
    /// Standard error.
    pub stderr: Vec<u8>,
}

/// Proof that this process owns a run. Dropping it releases the run, so a crash releases it too.
pub trait RunLease: Send + Sync + std::fmt::Debug {
    /// The run this lease covers.
    fn run(&self) -> &ArtifactId;
}

/// Durable evidence for runs.
///
/// The write lock is never held across a child process: every method is one short critical
/// section, and a run is complete if and only if its `run_result` exists.
#[async_trait]
pub trait EvidenceRecorder: Send + Sync {
    /// Captures the exact bytes of `source` (an artifact's inline text or referenced file).
    ///
    /// The record, the bytes and the digest are read consistently: if the artifact changes while
    /// being captured, the capture is retried once and then fails with `Conflict`. Identical bytes
    /// share one stored file. Fails with `NotFound` for an unknown artifact and `InvalidInput`
    /// for an artifact without content or over the size limit.
    async fn capture(&self, source: &ArtifactRef, role: CaptureRole) -> Result<CapturedBytes>;

    /// Records the run intent. Idempotent by `request_id`: an existing intent is returned with
    /// `created = false` and nothing is written. All referenced captures must exist (`NotFound`).
    async fn record_intent(&self, intent: RunIntent) -> Result<IntentRecord>;

    /// Takes ownership of an incomplete run. `Locked` when another owner holds it, `NotFound` for
    /// an unknown run, `Conflict` when the run is already complete.
    async fn acquire_lease(&self, run: &ArtifactId) -> Result<Box<dyn RunLease>>;

    /// Publishes the outputs and then the `run_result`, the commit point. Fails with `NotFound`
    /// for an unknown run and `Conflict` when it already has a result, so a completed run never
    /// changes outcome. An inconsistent `finish` (for example a success with a non-zero exit
    /// status) is `InvalidInput` and writes nothing. If a write fails the error is returned, the
    /// run stays incomplete and nothing is reported as published.
    async fn record_result(&self, run: &ArtifactId, finish: RunFinish) -> Result<RunRecord>;

    /// Returns a run by artifact reference, if it names a `run`.
    async fn run(&self, run: &ArtifactRef) -> Result<Option<RunRecord>>;

    /// Lists all runs ordered by id.
    async fn runs(&self) -> Result<Vec<RunRecord>>;

    /// The derived status of a run: `Running` only while its lease is held.
    async fn status(&self, run: &ArtifactId) -> Result<RunStatus>;

    /// The stored streams of a completed run, or `None` while it is incomplete.
    async fn streams(&self, run: &ArtifactId) -> Result<Option<RunStreams>>;

    /// Reads the bytes of a capture and verifies them against its digest. Damaged or missing
    /// bytes are `Inconsistent`.
    async fn read_capture(&self, capture: &ArtifactId) -> Result<Vec<u8>>;

    /// Marks runs whose owner is gone as `interrupted` and repairs implied relationships.
    ///
    /// Runs whose lease is held are left alone. It never reruns a child, never publishes
    /// unpublished outputs and never deletes evidence. Returns the runs it completed.
    async fn recover(&self) -> Result<Vec<RunRecord>>;
}

/// What a persisted validation report describes (ADR-0015).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReportSubject {
    /// The whole repository.
    Repository,
    /// One artifact, identified by the state the validation saw.
    Artifact {
        /// The artifact.
        id: ArtifactId,
        /// Its revision at validation time.
        revision: Revision,
        /// The digest of its content at validation time, when it has content.
        digest: Option<ContentDigest>,
    },
}

/// Persists validation reports as artifacts. Opt-in: `ValidationEngine::validate` never calls it.
#[async_trait]
pub trait ReportRecorder: Send + Sync {
    /// Stores the report as an immutable `validation_report` artifact and, for an artifact
    /// subject, a `validates` relationship to it. The report is not modified or revalidated.
    /// Fails with `NotFound` when the subject artifact does not exist and with `Inconsistent`
    /// when the repository is damaged; the caller keeps its report either way.
    async fn save_report(
        &self,
        report: &ValidationReport,
        subject: ReportSubject,
        strict: bool,
    ) -> Result<ArtifactId>;

    /// Loads a saved report and its subject, or `None` when the artifact is not a report.
    async fn load_report(
        &self,
        id: &ArtifactId,
    ) -> Result<Option<(ValidationReport, ReportSubject)>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorders_are_dyn_compatible() {
        fn takes(_: Option<&dyn EvidenceRecorder>, _: Option<&dyn ReportRecorder>) {}
        takes(None, None);
    }
}
