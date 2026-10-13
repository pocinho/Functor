//! The rules. Each is a pure function from a lenient load to findings; none depends on another.

use std::collections::{BTreeMap, BTreeSet};

use functor_core::{LoadIssue, LoadIssueKind, SnapshotLoad};
use functor_model::{
    Artifact, ArtifactId, Finding, Relationship, RelationshipType, RuleCode, Severity, Subject,
};

use crate::catalog::{
    V0001, V0002, V0003, V0004, V0005, V0006, V0007, V0008, V0009, V0010, V0011, V0012, V0013,
    V0014, V0015, V0016, V0017, V0018, V0019, rule_info,
};
use crate::graph::components;

/// What one rule found.
#[derive(Debug, Default)]
pub(crate) struct Output {
    pub findings: Vec<Finding>,
    /// The rule could not run to completion.
    pub incomplete: bool,
}

/// A rule: its code and the function that checks a load.
pub(crate) struct Rule {
    pub code: RuleCode,
    pub check: fn(&SnapshotLoad) -> Output,
}

/// Every rule, ordered by code.
pub(crate) const RULES: &[Rule] = &[
    Rule {
        code: V0001,
        check: v0001,
    },
    Rule {
        code: V0002,
        check: v0002,
    },
    Rule {
        code: V0003,
        check: v0003,
    },
    Rule {
        code: V0004,
        check: v0004,
    },
    Rule {
        code: V0005,
        check: v0005,
    },
    Rule {
        code: V0006,
        check: v0006,
    },
    Rule {
        code: V0007,
        check: v0007,
    },
    Rule {
        code: V0008,
        check: v0008,
    },
    Rule {
        code: V0009,
        check: v0009,
    },
    Rule {
        code: V0010,
        check: v0010,
    },
    Rule {
        code: V0011,
        check: v0011,
    },
    Rule {
        code: V0012,
        check: v0012,
    },
    Rule {
        code: V0013,
        check: v0013,
    },
    Rule {
        code: V0014,
        check: v0014,
    },
    Rule {
        code: V0015,
        check: v0015,
    },
    Rule {
        code: V0016,
        check: v0016,
    },
    Rule {
        code: V0017,
        check: v0017,
    },
    Rule {
        code: V0018,
        check: v0018,
    },
    Rule {
        code: V0019,
        check: v0019,
    },
];

fn severity(code: RuleCode) -> Severity {
    rule_info(code).map_or(Severity::Error, |info| info.severity)
}

fn finding(code: RuleCode, subject: Subject, message: impl Into<String>) -> Finding {
    Finding::new(code, severity(code), subject, message)
}

/// One sentence without a trailing period. Multi-line parser messages are reduced to their first
/// and last line, which carry the position and the reason.
fn summarize(detail: &str) -> String {
    let lines: Vec<&str> = detail
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let text = match lines.as_slice() {
        [] => "no detail".to_owned(),
        [only] => (*only).to_owned(),
        [first, .., last] => format!("{first}: {last}"),
    };
    text.trim_end_matches('.').to_owned()
}

// ---- rules about files, reported by the loader ----

/// Every problem the loader found, including the one that stopped it reading `repository.toml`.
fn issues(load: &SnapshotLoad) -> impl Iterator<Item = &LoadIssue> {
    load.info
        .as_ref()
        .err()
        .into_iter()
        .chain(load.issues.iter())
}

fn file_findings(code: RuleCode, kinds: &[LoadIssueKind], load: &SnapshotLoad) -> Output {
    let findings = issues(load)
        .filter(|issue| kinds.contains(&issue.kind))
        .map(|issue| {
            finding(
                code,
                Subject::File(issue.path.clone()),
                summarize(&issue.detail),
            )
            .with_related(issue.related.clone())
        })
        .collect();
    Output {
        findings,
        incomplete: false,
    }
}

fn v0001(load: &SnapshotLoad) -> Output {
    file_findings(V0001, &[LoadIssueKind::MalformedFile], load)
}

fn v0002(load: &SnapshotLoad) -> Output {
    file_findings(V0002, &[LoadIssueKind::MissingIdentifier], load)
}

fn v0003(load: &SnapshotLoad) -> Output {
    file_findings(V0003, &[LoadIssueKind::DuplicateArtifactId], load)
}

fn v0004(load: &SnapshotLoad) -> Output {
    file_findings(V0004, &[LoadIssueKind::DuplicateSlug], load)
}

fn v0005(load: &SnapshotLoad) -> Output {
    file_findings(V0005, &[LoadIssueKind::InvalidIdentifier], load)
}

fn v0008(load: &SnapshotLoad) -> Output {
    file_findings(V0008, &[LoadIssueKind::DuplicateRelationshipId], load)
}

fn v0009(load: &SnapshotLoad) -> Output {
    file_findings(V0009, &[LoadIssueKind::SelfReference], load)
}

fn v0010(load: &SnapshotLoad) -> Output {
    file_findings(
        V0010,
        &[
            LoadIssueKind::UnsafeContentPath,
            LoadIssueKind::MissingContentFile,
        ],
        load,
    )
}

fn v0013(load: &SnapshotLoad) -> Output {
    file_findings(V0013, &[LoadIssueKind::UnsupportedFormat], load)
}

fn v0014(load: &SnapshotLoad) -> Output {
    file_findings(V0014, &[LoadIssueKind::MissingStructure], load)
}

fn v0016(load: &SnapshotLoad) -> Output {
    file_findings(V0016, &[LoadIssueKind::UnexpectedFile], load)
}

fn v0017(load: &SnapshotLoad) -> Output {
    file_findings(V0017, &[LoadIssueKind::FilenameMismatch], load)
}

fn v0018(load: &SnapshotLoad) -> Output {
    file_findings(V0018, &[LoadIssueKind::UnknownField], load)
}

/// Something could not be read, so rules that needed it could not run to completion.
fn v0019(load: &SnapshotLoad) -> Output {
    let mut output = file_findings(V0019, &[LoadIssueKind::Unreadable], load);
    output.incomplete = !output.findings.is_empty();
    output
}

// ---- rules about the graph, over the entities that loaded cleanly ----

fn dangling(
    code: RuleCode,
    load: &SnapshotLoad,
    end: fn(&Relationship) -> &ArtifactId,
    role: &str,
) -> Output {
    let known: BTreeSet<&ArtifactId> = load.artifacts.iter().map(Artifact::id).collect();
    let findings = load
        .relationships
        .iter()
        .filter(|r| !known.contains(end(r)))
        .map(|r| {
            finding(
                code,
                Subject::Relationship(*r.id()),
                format!("{role} {} does not exist", end(r)),
            )
        })
        .collect();
    Output {
        findings,
        incomplete: false,
    }
}

fn v0006(load: &SnapshotLoad) -> Output {
    dangling(V0006, load, |r| r.source(), "source")
}

fn v0007(load: &SnapshotLoad) -> Output {
    dangling(V0007, load, |r| r.target(), "target")
}

/// One finding per cycle: a strongly connected component of more than one artifact in the graph
/// of one acyclic relationship type. Self-loops are `V0009`.
fn v0011(load: &SnapshotLoad) -> Output {
    let mut ids: Vec<ArtifactId> = load.artifacts.iter().map(|a| *a.id()).collect();
    ids.sort();
    let position: BTreeMap<ArtifactId, usize> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();
    let slugs: BTreeMap<ArtifactId, &str> = load
        .artifacts
        .iter()
        .map(|a| (*a.id(), a.slug().as_str()))
        .collect();

    let acyclic_types: BTreeSet<&RelationshipType> = load
        .relationships
        .iter()
        .map(Relationship::relationship_type)
        .filter(|t| t.is_acyclic())
        .collect();

    let mut findings = Vec::new();
    for relationship_type in acyclic_types {
        let mut adjacency = vec![Vec::new(); ids.len()];
        for relationship in load
            .relationships
            .iter()
            .filter(|r| r.relationship_type() == relationship_type)
        {
            if let (Some(&from), Some(&to)) = (
                position.get(relationship.source()),
                position.get(relationship.target()),
            ) {
                adjacency[from].push(to);
            }
        }
        for mut component in components(ids.len(), &adjacency) {
            if component.len() < 2 {
                continue;
            }
            component.sort_unstable();
            let members: Vec<ArtifactId> = component.iter().map(|&index| ids[index]).collect();
            let names: Vec<&str> = members
                .iter()
                .map(|id| slugs.get(id).copied().unwrap_or_default())
                .collect();
            findings.push(
                finding(
                    V0011,
                    Subject::Artifact(members[0]),
                    format!(
                        "cycle in `{relationship_type}` relationships among {}",
                        names.join(", ")
                    ),
                )
                .with_related(members.iter().map(|id| Subject::Artifact(*id))),
            );
        }
    }
    Output {
        findings,
        incomplete: false,
    }
}

fn v0012(load: &SnapshotLoad) -> Output {
    let findings = load
        .relationships
        .iter()
        .filter(|r| !r.relationship_type().is_built_in())
        .map(|r| {
            finding(
                V0012,
                Subject::Relationship(*r.id()),
                format!("custom relationship type `{}`", r.relationship_type()),
            )
        })
        .collect();
    Output {
        findings,
        incomplete: false,
    }
}

/// One finding per group of relationships sharing `(source, type, target)`, about the one with
/// the lowest id and relating the others.
fn v0015(load: &SnapshotLoad) -> Output {
    let mut groups: BTreeMap<(ArtifactId, &RelationshipType, ArtifactId), Vec<&Relationship>> =
        BTreeMap::new();
    for relationship in &load.relationships {
        groups
            .entry((
                *relationship.source(),
                relationship.relationship_type(),
                *relationship.target(),
            ))
            .or_default()
            .push(relationship);
    }
    let mut findings = Vec::new();
    for ((source, relationship_type, target), mut group) in groups {
        if group.len() < 2 {
            continue;
        }
        group.sort_by_key(|r| *r.id());
        findings.push(
            finding(
                V0015,
                Subject::Relationship(*group[0].id()),
                format!(
                    "`{relationship_type}` from {source} to {target} is defined {} times",
                    group.len()
                ),
            )
            .with_related(group[1..].iter().map(|r| Subject::Relationship(*r.id()))),
        );
    }
    Output {
        findings,
        incomplete: false,
    }
}

#[cfg(test)]
mod tests {
    use functor_model::{
        Artifact, ArtifactType, FormatVersion, NewArtifact, NewRelationship, RelationshipId,
        RepoPath, RepositoryId, RepositoryInfo, RepositoryName, Slug, Timestamp, Title,
    };

    use super::*;

    fn at() -> Timestamp {
        Timestamp::parse("2026-10-12T18:00:00Z").unwrap()
    }

    fn info() -> RepositoryInfo {
        RepositoryInfo::new(
            RepositoryId::from_bits(1),
            RepositoryName::new("Fixture").unwrap(),
            None,
            FormatVersion::CURRENT,
            at(),
        )
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

    fn issue(kind: LoadIssueKind, file: &str) -> LoadIssue {
        LoadIssue::new(kind, path(file), "the detail")
    }

    /// `a` (1) and `b` (2) with `b depends_on a`: no findings.
    fn clean() -> SnapshotLoad {
        SnapshotLoad {
            info: Ok(info()),
            artifacts: vec![artifact(1, "a"), artifact(2, "b")],
            relationships: vec![relationship(10, 2, "depends_on", 1)],
            issues: Vec::new(),
        }
    }

    /// Every code that fires on `load`, in catalog order, once per finding.
    fn fired(load: &SnapshotLoad) -> Vec<&'static str> {
        RULES
            .iter()
            .flat_map(|rule| {
                (rule.check)(load)
                    .findings
                    .into_iter()
                    .map(|f| f.code.as_str())
            })
            .collect()
    }

    #[test]
    fn a_clean_repository_triggers_no_rule() {
        assert_eq!(fired(&clean()), Vec::<&str>::new());
    }

    #[test]
    fn the_rule_table_lists_every_catalog_code_in_order() {
        let codes: Vec<&str> = RULES.iter().map(|r| r.code.as_str()).collect();
        let catalog: Vec<&str> = crate::catalog::CATALOG
            .iter()
            .map(|r| r.code.as_str())
            .collect();
        assert_eq!(codes, catalog);
    }

    #[test]
    fn every_rule_only_emits_its_own_code_and_severity() {
        let mut load = clean();
        load.issues = [
            LoadIssueKind::MalformedFile,
            LoadIssueKind::MissingIdentifier,
            LoadIssueKind::DuplicateArtifactId,
            LoadIssueKind::DuplicateSlug,
            LoadIssueKind::InvalidIdentifier,
            LoadIssueKind::SelfReference,
            LoadIssueKind::DuplicateRelationshipId,
            LoadIssueKind::UnsafeContentPath,
            LoadIssueKind::MissingContentFile,
            LoadIssueKind::MissingStructure,
            LoadIssueKind::UnexpectedFile,
            LoadIssueKind::FilenameMismatch,
            LoadIssueKind::UnknownField,
            LoadIssueKind::Unreadable,
            LoadIssueKind::UnsupportedFormat,
        ]
        .into_iter()
        .map(|kind| issue(kind, "artifacts/x.toml"))
        .collect();
        load.relationships.push(relationship(11, 2, "supports", 1));
        for rule in RULES {
            let expected = rule_info(rule.code).unwrap().severity;
            for finding in (rule.check)(&load).findings {
                assert_eq!(finding.code, rule.code);
                assert_eq!(finding.severity, expected, "{}", rule.code);
            }
        }
    }

    // ---- one minimal fixture per rule code, triggering exactly that code ----

    fn with_issue(kind: LoadIssueKind, file: &str) -> SnapshotLoad {
        let mut load = clean();
        load.issues.push(issue(kind, file));
        load
    }

    #[test]
    fn v0001_malformed_file() {
        let load = with_issue(LoadIssueKind::MalformedFile, "artifacts/x.toml");
        assert_eq!(fired(&load), ["V0001"]);
    }

    #[test]
    fn v0002_missing_identifier() {
        let load = with_issue(LoadIssueKind::MissingIdentifier, "artifacts/x.toml");
        assert_eq!(fired(&load), ["V0002"]);
    }

    #[test]
    fn v0003_duplicate_artifact_id() {
        let load = with_issue(LoadIssueKind::DuplicateArtifactId, "artifacts/x.toml");
        assert_eq!(fired(&load), ["V0003"]);
    }

    #[test]
    fn v0004_duplicate_slug() {
        let load = with_issue(LoadIssueKind::DuplicateSlug, "artifacts/x.toml");
        assert_eq!(fired(&load), ["V0004"]);
    }

    #[test]
    fn v0005_invalid_identifier() {
        let load = with_issue(LoadIssueKind::InvalidIdentifier, "relationships/x.toml");
        assert_eq!(fired(&load), ["V0005"]);
    }

    #[test]
    fn v0006_dangling_source() {
        let mut load = clean();
        load.relationships = vec![relationship(10, 99, "depends_on", 1)];
        assert_eq!(fired(&load), ["V0006"]);
        let found = (V0006_RULE.check)(&load).findings;
        assert_eq!(
            found[0].subject,
            Subject::Relationship(RelationshipId::from_bits(10))
        );
        assert_eq!(
            found[0].message,
            format!("source {} does not exist", ArtifactId::from_bits(99))
        );
    }

    const V0006_RULE: Rule = Rule {
        code: V0006,
        check: v0006,
    };

    #[test]
    fn v0007_dangling_target() {
        let mut load = clean();
        load.relationships = vec![relationship(10, 2, "depends_on", 99)];
        assert_eq!(fired(&load), ["V0007"]);
    }

    #[test]
    fn both_endpoints_missing_raise_both_rules() {
        let mut load = clean();
        load.relationships = vec![relationship(10, 98, "references", 99)];
        assert_eq!(fired(&load), ["V0006", "V0007"]);
    }

    #[test]
    fn v0008_duplicate_relationship_id() {
        let load = with_issue(
            LoadIssueKind::DuplicateRelationshipId,
            "relationships/x.toml",
        );
        assert_eq!(fired(&load), ["V0008"]);
    }

    #[test]
    fn v0009_self_reference() {
        let load = with_issue(LoadIssueKind::SelfReference, "relationships/x.toml");
        assert_eq!(fired(&load), ["V0009"]);
    }

    #[test]
    fn v0010_unsafe_and_missing_content() {
        let load = with_issue(LoadIssueKind::UnsafeContentPath, "artifacts/x.toml");
        assert_eq!(fired(&load), ["V0010"]);
        let load = with_issue(LoadIssueKind::MissingContentFile, "artifacts/x.toml");
        assert_eq!(fired(&load), ["V0010"]);
    }

    #[test]
    fn v0011_two_node_cycle() {
        let mut load = clean();
        load.relationships = vec![
            relationship(10, 2, "depends_on", 1),
            relationship(11, 1, "depends_on", 2),
        ];
        assert_eq!(fired(&load), ["V0011"]);
        let found = (RULES[10].check)(&load).findings;
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].subject,
            Subject::Artifact(ArtifactId::from_bits(1))
        );
        assert_eq!(
            found[0].related,
            [
                Subject::Artifact(ArtifactId::from_bits(1)),
                Subject::Artifact(ArtifactId::from_bits(2))
            ]
        );
        assert_eq!(
            found[0].message,
            "cycle in `depends_on` relationships among a, b"
        );
    }

    fn ring(size: u128, kind: &str) -> SnapshotLoad {
        SnapshotLoad {
            info: Ok(info()),
            artifacts: (1..=size).map(|n| artifact(n, &format!("n{n}"))).collect(),
            relationships: (1..=size)
                .map(|n| relationship(100 + n, n, kind, n % size + 1))
                .collect(),
            issues: Vec::new(),
        }
    }

    #[test]
    fn v0011_long_cycle_is_one_finding_listing_everyone_sorted() {
        let load = ring(6, "extends");
        let found = (RULES[10].check)(&load).findings;
        assert_eq!(found.len(), 1);
        let expected: Vec<Subject> = (1..=6)
            .map(|n| Subject::Artifact(ArtifactId::from_bits(n)))
            .collect();
        assert_eq!(found[0].related, expected);
    }

    #[test]
    fn v0011_two_disjoint_cycles_are_two_findings() {
        let mut load = SnapshotLoad {
            info: Ok(info()),
            artifacts: (1..=5).map(|n| artifact(n, &format!("n{n}"))).collect(),
            relationships: Vec::new(),
            issues: Vec::new(),
        };
        load.relationships = vec![
            relationship(10, 1, "depends_on", 2),
            relationship(11, 2, "depends_on", 1),
            relationship(12, 3, "depends_on", 4),
            relationship(13, 4, "depends_on", 5),
            relationship(14, 5, "depends_on", 3),
        ];
        let found = (RULES[10].check)(&load).findings;
        assert_eq!(found.len(), 2);
        let sizes: Vec<usize> = found.iter().map(|f| f.related.len()).collect();
        assert_eq!(sizes.iter().sum::<usize>(), 5);
        assert!(sizes.contains(&2) && sizes.contains(&3));
    }

    #[test]
    fn v0011_mixed_types_do_not_form_a_cross_type_cycle() {
        let mut load = clean();
        // `b depends_on a` and `a extends b` is not a cycle: each type is acyclic on its own.
        load.relationships = vec![
            relationship(10, 2, "depends_on", 1),
            relationship(11, 1, "extends", 2),
        ];
        assert_eq!(fired(&load), Vec::<&str>::new());
    }

    #[test]
    fn v0011_ignores_types_that_may_be_cyclic() {
        let mut load = clean();
        load.relationships = vec![
            relationship(10, 2, "references", 1),
            relationship(11, 1, "references", 2),
        ];
        assert_eq!(fired(&load), Vec::<&str>::new());
        let mut custom = clean();
        custom.relationships = vec![
            relationship(10, 2, "supports", 1),
            relationship(11, 1, "supports", 2),
        ];
        assert_eq!(fired(&custom), ["V0012", "V0012"]);
    }

    #[test]
    fn v0011_does_not_follow_dangling_edges() {
        let mut load = clean();
        load.relationships = vec![
            relationship(10, 1, "depends_on", 99),
            relationship(11, 99, "depends_on", 1),
        ];
        assert_eq!(fired(&load), ["V0006", "V0007"]);
    }

    #[test]
    fn v0012_custom_relationship_type() {
        let mut load = clean();
        load.relationships = vec![relationship(10, 2, "supports", 1)];
        assert_eq!(fired(&load), ["V0012"]);
        let found = (RULES[11].check)(&load).findings;
        assert_eq!(found[0].message, "custom relationship type `supports`");
    }

    #[test]
    fn v0013_unsupported_format_version() {
        let mut load = clean();
        load.info = Err(issue(
            LoadIssueKind::UnsupportedFormat,
            ".functor/repository.toml",
        ));
        assert_eq!(fired(&load), ["V0013"]);
    }

    #[test]
    fn v0014_missing_structure() {
        let load = with_issue(LoadIssueKind::MissingStructure, "relationships");
        assert_eq!(fired(&load), ["V0014"]);
    }

    #[test]
    fn v0015_duplicate_relationship_is_reported_once_per_group() {
        let mut load = clean();
        load.relationships = vec![
            relationship(12, 2, "depends_on", 1),
            relationship(10, 2, "depends_on", 1),
            relationship(11, 2, "depends_on", 1),
        ];
        assert_eq!(fired(&load), ["V0015"]);
        let found = (RULES[14].check)(&load).findings;
        assert_eq!(
            found[0].subject,
            Subject::Relationship(RelationshipId::from_bits(10)),
            "the lowest id is the subject"
        );
        assert_eq!(
            found[0].related,
            [
                Subject::Relationship(RelationshipId::from_bits(11)),
                Subject::Relationship(RelationshipId::from_bits(12))
            ]
        );
    }

    #[test]
    fn different_directions_or_types_are_not_duplicates() {
        let mut load = clean();
        load.relationships = vec![
            relationship(10, 2, "depends_on", 1),
            relationship(11, 1, "depends_on", 2),
            relationship(12, 2, "references", 1),
        ];
        // The first two form a cycle (V0011) but are not duplicates.
        assert_eq!(fired(&load), ["V0011"]);
    }

    #[test]
    fn v0016_unexpected_file() {
        let load = with_issue(LoadIssueKind::UnexpectedFile, "artifacts/notes.txt");
        assert_eq!(fired(&load), ["V0016"]);
    }

    #[test]
    fn v0017_filename_mismatch() {
        let load = with_issue(LoadIssueKind::FilenameMismatch, "artifacts/renamed.toml");
        assert_eq!(fired(&load), ["V0017"]);
    }

    #[test]
    fn v0018_unknown_field() {
        let load = with_issue(LoadIssueKind::UnknownField, "artifacts/x.toml");
        assert_eq!(fired(&load), ["V0018"]);
    }

    #[test]
    fn v0019_unreadable_marks_the_rule_set_incomplete() {
        let load = with_issue(LoadIssueKind::Unreadable, "artifacts");
        assert_eq!(fired(&load), ["V0019"]);
        let output = v0019(&load);
        assert!(output.incomplete);
        assert!(!v0019(&clean()).incomplete);
    }

    // ---- messages and context ----

    #[test]
    fn file_findings_are_about_the_file_and_carry_related_files() {
        let mut load = clean();
        let other = Subject::File(path("artifacts/first.toml"));
        load.issues.push(
            LoadIssue::new(
                LoadIssueKind::DuplicateSlug,
                path("artifacts/second.toml"),
                "slug `a` is already used in first.toml",
            )
            .with_related([other.clone()]),
        );
        let found = v0004(&load).findings;
        assert_eq!(
            found[0].subject,
            Subject::File(path("artifacts/second.toml"))
        );
        assert_eq!(found[0].related, [other]);
        assert_eq!(found[0].message, "slug `a` is already used in first.toml");
    }

    #[test]
    fn messages_are_one_line_without_a_trailing_period() {
        assert_eq!(
            summarize("file is not valid UTF-8."),
            "file is not valid UTF-8"
        );
        assert_eq!(summarize(""), "no detail");
        assert_eq!(
            summarize(
                "TOML parse error at line 1, column 8\n  |\n1 | this is = = bad\n  |        ^\ninvalid string\n"
            ),
            "TOML parse error at line 1, column 8: invalid string"
        );
    }

    #[test]
    fn the_repository_file_problem_is_found_even_though_the_load_has_no_info() {
        let mut load = clean();
        load.info = Err(issue(
            LoadIssueKind::MalformedFile,
            ".functor/repository.toml",
        ));
        assert_eq!(fired(&load), ["V0001"]);
    }
}
