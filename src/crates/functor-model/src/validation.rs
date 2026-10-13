//! Validation value types (RFC-0007). Rules and the engine live in `functor-validation`.
//!
//! See `docs/validation-rules.md`.

use std::fmt;

use crate::id::{ArtifactId, RelationshipId};
use crate::repository::RepoPath;
use crate::timestamp::Timestamp;

/// A stable rule code such as `V0007`. Codes are never reused for a different check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleCode(&'static str);

impl RuleCode {
    /// Declares a rule code.
    #[must_use]
    pub const fn new(code: &'static str) -> Self {
        Self(code)
    }

    /// The code text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for RuleCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// How serious a finding is. Ordered from least to most serious.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Severity {
    /// Informational; never changes the outcome.
    Info,
    /// Valid but suspect or non-portable.
    Warning,
    /// Invalid or inconsistent data.
    Error,
}

impl Severity {
    /// The lowercase name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The overall result of a validation run (RFC-0007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Outcome {
    /// No warnings or errors, and every rule ran.
    Passed,
    /// No errors, at least one warning.
    Warning,
    /// At least one error.
    Failed,
    /// No errors, but at least one rule could not complete.
    Unknown,
}

impl Outcome {
    /// Derives the outcome. Precedence: `Failed` > `Unknown` > `Warning` > `Passed`.
    #[must_use]
    pub fn derive(findings: &[Finding], incomplete: bool) -> Self {
        if findings.iter().any(|f| f.severity == Severity::Error) {
            Self::Failed
        } else if incomplete {
            Self::Unknown
        } else if findings.iter().any(|f| f.severity == Severity::Warning) {
            Self::Warning
        } else {
            Self::Passed
        }
    }

    /// The lowercase name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Warning => "warning",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a finding is about. Ordered by kind, then by id or path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Subject {
    /// The repository as a whole.
    Repository,
    /// An artifact.
    Artifact(ArtifactId),
    /// A relationship.
    Relationship(RelationshipId),
    /// A file, possibly not a valid entity.
    File(RepoPath),
}

/// One validation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The rule that produced the finding.
    pub code: RuleCode,
    /// The severity.
    pub severity: Severity,
    /// What the finding is about.
    pub subject: Subject,
    /// One sentence, no trailing period.
    pub message: String,
    /// Other subjects involved, for example the other duplicate or the nodes of a cycle.
    pub related: Vec<Subject>,
}

impl Finding {
    /// Creates a finding with no related subjects.
    #[must_use]
    pub fn new(
        code: RuleCode,
        severity: Severity,
        subject: Subject,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity,
            subject,
            message: message.into(),
            related: Vec::new(),
        }
    }

    /// Adds related subjects.
    #[must_use]
    pub fn with_related(mut self, related: impl IntoIterator<Item = Subject>) -> Self {
        self.related = related.into_iter().collect();
        self
    }
}

/// Counts shown in a validation report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// Artifacts checked.
    pub artifacts: usize,
    /// Relationships checked.
    pub relationships: usize,
    /// Findings with severity `Error`.
    pub errors: usize,
    /// Findings with severity `Warning`.
    pub warnings: usize,
    /// Findings with severity `Info`.
    pub infos: usize,
}

/// The result of validating a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    outcome: Outcome,
    findings: Vec<Finding>,
    counts: Counts,
    completed: Timestamp,
}

impl ValidationReport {
    /// Assembles a report: sorts findings deterministically (code, subject, message), derives the
    /// outcome and computes the counts.
    ///
    /// `incomplete` states that at least one rule could not run to completion.
    #[must_use]
    pub fn new(
        mut findings: Vec<Finding>,
        artifacts: usize,
        relationships: usize,
        incomplete: bool,
        completed: Timestamp,
    ) -> Self {
        findings.sort_by(|a, b| {
            (a.code, &a.subject, &a.message).cmp(&(b.code, &b.subject, &b.message))
        });
        let count = |severity| findings.iter().filter(|f| f.severity == severity).count();
        let counts = Counts {
            artifacts,
            relationships,
            errors: count(Severity::Error),
            warnings: count(Severity::Warning),
            infos: count(Severity::Info),
        };
        Self {
            outcome: Outcome::derive(&findings, incomplete),
            findings,
            counts,
            completed,
        }
    }

    /// The overall outcome.
    #[must_use]
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// The findings, sorted by code, subject, then message.
    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// The counts.
    #[must_use]
    pub fn counts(&self) -> Counts {
        self.counts
    }

    /// When validation completed.
    #[must_use]
    pub fn completed(&self) -> Timestamp {
        self.completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1: RuleCode = RuleCode::new("V0001");
    const V7: RuleCode = RuleCode::new("V0007");

    fn finding(code: RuleCode, severity: Severity, subject: Subject, message: &str) -> Finding {
        Finding::new(code, severity, subject, message)
    }

    fn at() -> Timestamp {
        Timestamp::parse("2026-10-12T18:00:00Z").unwrap()
    }

    #[test]
    fn outcome_derivation_table() {
        let info = finding(V1, Severity::Info, Subject::Repository, "i");
        let warn = finding(V1, Severity::Warning, Subject::Repository, "w");
        let err = finding(V1, Severity::Error, Subject::Repository, "e");
        let all = [warn.clone(), err.clone(), info.clone()];
        let cases: [(&[Finding], bool, Outcome); 9] = [
            (&[], false, Outcome::Passed),
            (&[], true, Outcome::Unknown),
            (std::slice::from_ref(&info), false, Outcome::Passed),
            (std::slice::from_ref(&info), true, Outcome::Unknown),
            (std::slice::from_ref(&warn), false, Outcome::Warning),
            (std::slice::from_ref(&warn), true, Outcome::Unknown),
            (std::slice::from_ref(&err), false, Outcome::Failed),
            (std::slice::from_ref(&err), true, Outcome::Failed),
            (&all, false, Outcome::Failed),
        ];
        for (findings, incomplete, expected) in cases {
            assert_eq!(Outcome::derive(findings, incomplete), expected);
        }
    }

    #[test]
    fn severity_is_ordered() {
        assert!(Severity::Info < Severity::Warning && Severity::Warning < Severity::Error);
    }

    #[test]
    fn report_sorts_and_counts() {
        let a = ArtifactId::from_bits(1);
        let b = ArtifactId::from_bits(2);
        let report = ValidationReport::new(
            vec![
                finding(V7, Severity::Error, Subject::Artifact(b), "z"),
                finding(V1, Severity::Warning, Subject::Artifact(b), "m"),
                finding(V7, Severity::Error, Subject::Artifact(a), "z"),
                finding(V7, Severity::Error, Subject::Artifact(a), "a"),
                finding(V1, Severity::Info, Subject::Repository, "r"),
            ],
            2,
            1,
            false,
            at(),
        );
        let order: Vec<(&str, &str)> = report
            .findings()
            .iter()
            .map(|f| (f.code.as_str(), f.message.as_str()))
            .collect();
        assert_eq!(
            order,
            [
                ("V0001", "r"),
                ("V0001", "m"),
                ("V0007", "a"),
                ("V0007", "z"),
                ("V0007", "z")
            ]
        );
        assert_eq!(report.outcome(), Outcome::Failed);
        let counts = report.counts();
        assert_eq!(
            (
                counts.artifacts,
                counts.relationships,
                counts.errors,
                counts.warnings,
                counts.infos
            ),
            (2, 1, 3, 1, 1)
        );
        assert_eq!(report.completed(), at());
    }

    #[test]
    fn report_order_is_independent_of_input_order() {
        let a = finding(V1, Severity::Error, Subject::Repository, "a");
        let b = finding(
            V7,
            Severity::Warning,
            Subject::Artifact(ArtifactId::from_bits(1)),
            "b",
        );
        let one = ValidationReport::new(vec![a.clone(), b.clone()], 0, 0, false, at());
        let two = ValidationReport::new(vec![b, a], 0, 0, false, at());
        assert_eq!(one, two);
    }

    #[test]
    fn clean_report_passes() {
        let report = ValidationReport::new(Vec::new(), 3, 2, false, at());
        assert_eq!(report.outcome(), Outcome::Passed);
        assert_eq!(report.outcome().to_string(), "passed");
    }
}
