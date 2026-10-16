//! Running the rules over a load and scoping the report.

use std::collections::BTreeSet;

use functor_core::{FunctorError, Result, SnapshotLoad, ValidationTarget};
use functor_model::{
    Artifact, ArtifactRef, EntityKind, Finding, RepoPath, RuleCode, Subject, Timestamp,
    ValidationReport,
};

use crate::catalog::{V0013, V0014, V0019};
use crate::rules::RULES;

/// The repository metadata file; problems in it concern the whole repository.
const REPOSITORY_FILE: &str = ".functor/repository.toml";

/// Runs every rule over `load` and assembles the report.
///
/// For [`ValidationTarget::Artifact`] the rules still run over the whole load, because
/// uniqueness and reachability need it; only the reported findings are restricted to those about
/// the artifact, its relationships and the files defining them. Repository-level findings
/// (`V0013`, `V0014`, `V0019` and anything about `repository.toml`) are always reported.
///
/// The outcome is derived from the reported findings; a rule that could not complete makes it
/// `Unknown` unless an error exists.
///
/// # Errors
/// `NotFound` when the scoped artifact is not among the artifacts that loaded cleanly.
pub fn evaluate(
    load: &SnapshotLoad,
    target: &ValidationTarget,
    completed: Timestamp,
) -> Result<ValidationReport> {
    let scope = match target {
        ValidationTarget::Repository => None,
        ValidationTarget::Artifact(reference) => Some(Scope::of(load, reference)?),
        // `ValidationTarget` is non-exhaustive; a variant this engine does not know is a bug.
        _ => {
            return Err(FunctorError::Internal(
                "unsupported validation target".to_owned(),
            ));
        }
    };

    let mut findings = Vec::new();
    let mut incomplete = false;
    for rule in RULES {
        let output = (rule.check)(load);
        debug_assert!(
            output
                .findings
                .iter()
                .all(|finding| finding.code == rule.code),
            "rule {} produced a finding with another code",
            rule.code
        );
        incomplete |= output.incomplete;
        findings.extend(output.findings);
    }
    if let Some(scope) = &scope {
        findings.retain(|finding| scope.reports(finding));
    }
    Ok(ValidationReport::new(
        findings,
        load.artifacts.len(),
        load.relationships.len(),
        incomplete,
        completed,
    ))
}

/// Which findings concern one artifact.
struct Scope {
    subjects: BTreeSet<Subject>,
    files: BTreeSet<RepoPath>,
}

fn find<'a>(load: &'a SnapshotLoad, reference: &ArtifactRef) -> Option<&'a Artifact> {
    load.artifacts.iter().find(|artifact| match reference {
        ArtifactRef::Id(id) => artifact.id() == id,
        ArtifactRef::Slug(slug) => artifact.slug() == slug,
    })
}

impl Scope {
    fn of(load: &SnapshotLoad, reference: &ArtifactRef) -> Result<Self> {
        let artifact = find(load, reference)
            .ok_or_else(|| FunctorError::not_found(EntityKind::Artifact, reference.to_string()))?;
        let id = *artifact.id();

        let mut subjects = BTreeSet::from([Subject::Artifact(id)]);
        let mut files = BTreeSet::new();
        files.extend(RepoPath::new(&format!("artifacts/{id}.toml")));
        for relationship in load
            .relationships
            .iter()
            .filter(|r| *r.source() == id || *r.target() == id)
        {
            subjects.insert(Subject::Relationship(*relationship.id()));
            files.extend(RepoPath::new(&format!(
                "relationships/{}.toml",
                relationship.id()
            )));
        }
        // Files the loader tied to these entities, whatever they are called.
        files.extend(
            load.issues
                .iter()
                .filter(|issue| issue.entity.as_ref().is_some_and(|e| subjects.contains(e)))
                .map(|issue| issue.path.clone()),
        );
        Ok(Self { subjects, files })
    }

    fn mentions(&self, subject: &Subject) -> bool {
        match subject {
            Subject::File(path) => self.files.contains(path),
            other => self.subjects.contains(other),
        }
    }

    fn reports(&self, finding: &Finding) -> bool {
        is_repository_level(finding)
            || self.mentions(&finding.subject)
            || finding.related.iter().any(|related| self.mentions(related))
    }
}

fn is_repository_level(finding: &Finding) -> bool {
    const ALWAYS: [RuleCode; 3] = [V0013, V0014, V0019];
    ALWAYS.contains(&finding.code)
        || matches!(&finding.subject, Subject::File(path) if path.as_str() == REPOSITORY_FILE)
        || finding.subject == Subject::Repository
}

#[cfg(test)]
mod tests {
    use functor_core::{ErrorKind, LoadIssue, LoadIssueKind};
    use functor_model::{
        ArtifactId, ArtifactType, FormatVersion, NewArtifact, NewRelationship, Outcome,
        Relationship, RelationshipId, RelationshipType, RepositoryId, RepositoryInfo,
        RepositoryName, Slug, Title,
    };

    use super::*;

    fn at() -> Timestamp {
        Timestamp::parse("2026-10-12T18:00:00Z").unwrap()
    }

    fn artifact(bits: u128, slug: &str) -> Artifact {
        Artifact::create(
            ArtifactId::from_bits(bits),
            NewArtifact::new(
                Slug::parse(slug).unwrap(),
                ArtifactType::document(),
                Title::new(slug).unwrap(),
            ),
            at(),
        )
    }

    fn relationship(bits: u128, source: u128, kind: &str, target: u128) -> Relationship {
        Relationship::create(
            RelationshipId::from_bits(bits),
            NewRelationship::new(
                RelationshipType::parse(kind).unwrap(),
                ArtifactId::from_bits(source),
                ArtifactId::from_bits(target),
            ),
            at(),
        )
        .unwrap()
    }

    fn path(text: &str) -> RepoPath {
        RepoPath::new(text).unwrap()
    }

    /// Artifacts `a` (1), `b` (2), `c` (3), `lonely` (4); `b depends_on a`, `c depends_on b`.
    fn load() -> SnapshotLoad {
        SnapshotLoad {
            info: Ok(RepositoryInfo::new(
                RepositoryId::from_bits(1),
                RepositoryName::new("Fixture").unwrap(),
                None,
                FormatVersion::CURRENT,
                at(),
            )),
            artifacts: vec![
                artifact(1, "a"),
                artifact(2, "b"),
                artifact(3, "c"),
                artifact(4, "lonely"),
            ],
            relationships: vec![
                relationship(10, 2, "depends_on", 1),
                relationship(11, 3, "depends_on", 2),
            ],
            issues: Vec::new(),
        }
    }

    fn run(load: &SnapshotLoad, target: &ValidationTarget) -> ValidationReport {
        evaluate(load, target, at()).unwrap()
    }

    fn codes(report: &ValidationReport) -> Vec<&'static str> {
        report.findings().iter().map(|f| f.code.as_str()).collect()
    }

    fn scoped(text: &str) -> ValidationTarget {
        ValidationTarget::Artifact(ArtifactRef::parse(text).unwrap())
    }

    #[test]
    fn a_clean_repository_passes_with_counts() {
        let report = run(&load(), &ValidationTarget::Repository);
        assert_eq!(report.outcome(), Outcome::Passed);
        assert_eq!(report.findings(), []);
        assert_eq!(report.counts().artifacts, 4);
        assert_eq!(report.counts().relationships, 2);
        assert_eq!(report.completed(), at());
    }

    #[test]
    fn the_outcome_follows_the_worst_finding() {
        let mut warning = load();
        warning
            .relationships
            .push(relationship(12, 1, "related_to", 4));
        assert_eq!(
            run(&warning, &ValidationTarget::Repository).outcome(),
            Outcome::Warning
        );

        let mut failed = load();
        failed
            .relationships
            .push(relationship(12, 1, "depends_on", 99));
        let report = run(&failed, &ValidationTarget::Repository);
        assert_eq!(report.outcome(), Outcome::Failed);
        assert_eq!(report.counts().errors, 1);

        let mut info = load();
        info.issues.push(LoadIssue::new(
            LoadIssueKind::UnknownField,
            path("artifacts/x.toml"),
            "unknown field(s): colour",
        ));
        let report = run(&info, &ValidationTarget::Repository);
        assert_eq!(
            report.outcome(),
            Outcome::Passed,
            "info never changes the outcome"
        );
        assert_eq!(report.counts().infos, 1);
    }

    #[test]
    fn an_unreadable_path_makes_the_outcome_unknown_unless_there_is_an_error() {
        let mut unknown = load();
        unknown.issues.push(LoadIssue::new(
            LoadIssueKind::Unreadable,
            path("artifacts"),
            "access is denied",
        ));
        let report = run(&unknown, &ValidationTarget::Repository);
        assert_eq!(report.outcome(), Outcome::Unknown);
        assert_eq!(codes(&report), ["V0019"]);

        unknown
            .relationships
            .push(relationship(12, 1, "depends_on", 99));
        assert_eq!(
            run(&unknown, &ValidationTarget::Repository).outcome(),
            Outcome::Failed
        );
    }

    #[test]
    fn findings_are_ordered_by_code_then_subject() {
        let mut load = load();
        load.relationships = vec![
            relationship(11, 1, "depends_on", 99),
            relationship(10, 1, "depends_on", 98),
            relationship(12, 98, "related_to", 1),
        ];
        let report = run(&load, &ValidationTarget::Repository);
        assert_eq!(codes(&report), ["V0006", "V0007", "V0007", "V0012"]);
        let subjects: Vec<&Subject> = report.findings().iter().map(|f| &f.subject).collect();
        assert_eq!(
            subjects[1..3],
            [
                &Subject::Relationship(RelationshipId::from_bits(10)),
                &Subject::Relationship(RelationshipId::from_bits(11))
            ]
        );
    }

    #[test]
    fn evaluation_is_deterministic() {
        let mut load = load();
        load.relationships
            .push(relationship(12, 1, "depends_on", 99));
        load.relationships
            .push(relationship(13, 1, "related_to", 2));
        let first = run(&load, &ValidationTarget::Repository);
        let second = run(&load, &ValidationTarget::Repository);
        assert_eq!(first, second);
    }

    // ---- scoped validation ----

    fn broken() -> SnapshotLoad {
        let mut load = load();
        // A dangling relationship touching `a`, a custom one touching only `c` and `lonely`.
        load.relationships
            .push(relationship(12, 1, "depends_on", 99));
        load.relationships
            .push(relationship(13, 3, "related_to", 4));
        load
    }

    #[test]
    fn scoping_reports_only_findings_about_the_artifact_and_its_relationships() {
        let load = broken();
        let whole = run(&load, &ValidationTarget::Repository);
        assert_eq!(codes(&whole), ["V0007", "V0012"]);

        assert_eq!(codes(&run(&load, &scoped("a"))), ["V0007"]);
        assert_eq!(codes(&run(&load, &scoped("c"))), ["V0012"]);
        assert_eq!(codes(&run(&load, &scoped("lonely"))), ["V0012"]);
        assert_eq!(codes(&run(&load, &scoped("b"))), Vec::<&str>::new());
        assert_eq!(run(&load, &scoped("b")).outcome(), Outcome::Passed);
    }

    #[test]
    fn scoped_runs_resolve_slugs_ids_and_canonical_references_alike() {
        let load = broken();
        let id = ArtifactId::from_bits(1);
        let by_slug = run(&load, &scoped("a"));
        assert_eq!(by_slug, run(&load, &scoped(&id.to_string())));
        assert_eq!(by_slug, run(&load, &scoped(&id.canonical())));
    }

    #[test]
    fn scoped_counts_still_describe_the_whole_repository() {
        let report = run(&broken(), &scoped("b"));
        assert_eq!(report.counts().artifacts, 4);
        assert_eq!(report.counts().relationships, 4);
    }

    #[test]
    fn an_unknown_scoped_artifact_is_not_found() {
        let error = evaluate(&load(), &scoped("nope"), at()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::NotFound);
    }

    #[test]
    fn a_cycle_is_reported_for_every_participant_and_not_for_others() {
        let mut load = load();
        load.relationships
            .push(relationship(12, 1, "depends_on", 3));
        // a -> ? : b depends_on a, c depends_on b, a depends_on c.
        for member in ["a", "b", "c"] {
            assert_eq!(codes(&run(&load, &scoped(member))), ["V0011"], "{member}");
        }
        assert_eq!(codes(&run(&load, &scoped("lonely"))), Vec::<&str>::new());
    }

    #[test]
    fn duplicates_are_reported_for_every_artifact_touched() {
        let mut load = load();
        load.relationships
            .push(relationship(12, 2, "depends_on", 1));
        for member in ["a", "b"] {
            assert_eq!(codes(&run(&load, &scoped(member))), ["V0015"], "{member}");
        }
        assert_eq!(codes(&run(&load, &scoped("c"))), Vec::<&str>::new());
    }

    #[test]
    fn file_findings_tied_to_an_entity_follow_it_even_when_the_file_was_renamed() {
        let mut load = load();
        load.issues.push(
            LoadIssue::new(
                LoadIssueKind::FilenameMismatch,
                path("artifacts/renamed.toml"),
                "file name does not match its id",
            )
            .with_entity(Subject::Artifact(ArtifactId::from_bits(2))),
        );
        assert_eq!(codes(&run(&load, &scoped("b"))), ["V0017"]);
        assert_eq!(codes(&run(&load, &scoped("a"))), Vec::<&str>::new());
        assert_eq!(codes(&run(&load, &ValidationTarget::Repository)), ["V0017"]);
    }

    #[test]
    fn a_duplicate_slug_is_reported_for_the_artifact_that_kept_the_slug() {
        let first = format!("artifacts/{}.toml", ArtifactId::from_bits(1));
        let mut load = load();
        load.issues.push(
            LoadIssue::new(
                LoadIssueKind::DuplicateSlug,
                path("artifacts/zzz.toml"),
                "slug `a` is already used",
            )
            .with_entity(Subject::Artifact(ArtifactId::from_bits(500)))
            .with_related([Subject::File(path(&first))]),
        );
        assert_eq!(codes(&run(&load, &scoped("a"))), ["V0004"]);
        assert_eq!(codes(&run(&load, &scoped("b"))), Vec::<&str>::new());
    }

    #[test]
    fn a_file_named_after_the_artifact_is_in_scope() {
        let mut load = load();
        load.issues.push(LoadIssue::new(
            LoadIssueKind::MissingContentFile,
            path(&format!("artifacts/{}.toml", ArtifactId::from_bits(4))),
            "content file `docs/x.md` does not exist",
        ));
        assert_eq!(codes(&run(&load, &scoped("lonely"))), ["V0010"]);
        assert_eq!(codes(&run(&load, &scoped("a"))), Vec::<&str>::new());
    }

    #[test]
    fn repository_level_findings_are_always_reported() {
        let mut load = load();
        load.info = Err(LoadIssue::new(
            LoadIssueKind::UnsupportedFormat,
            path(".functor/repository.toml"),
            "format version 9 is not supported",
        ));
        load.issues.push(LoadIssue::new(
            LoadIssueKind::MissingStructure,
            path("relationships"),
            "required directory is missing",
        ));
        load.issues.push(LoadIssue::new(
            LoadIssueKind::MalformedFile,
            path(".functor/repository.toml"),
            "bad",
        ));
        load.issues.push(LoadIssue::new(
            LoadIssueKind::Unreadable,
            path("artifacts"),
            "denied",
        ));
        let report = run(&load, &scoped("lonely"));
        assert_eq!(codes(&report), ["V0001", "V0013", "V0014", "V0019"]);
        assert_eq!(report.outcome(), Outcome::Failed);
    }

    #[test]
    fn unrelated_file_problems_stay_out_of_a_scoped_report() {
        let mut load = load();
        load.issues.push(LoadIssue::new(
            LoadIssueKind::UnexpectedFile,
            path("artifacts/notes.txt"),
            "not an entity file",
        ));
        assert_eq!(codes(&run(&load, &ValidationTarget::Repository)), ["V0016"]);
        assert_eq!(codes(&run(&load, &scoped("a"))), Vec::<&str>::new());
    }
}
