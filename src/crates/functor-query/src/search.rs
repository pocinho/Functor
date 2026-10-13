//! Search: filter artifacts by type, tag, text and metadata.

use functor_model::{Artifact, MetadataKey, MetadataValue, RepositorySnapshot, SearchQuery};

/// Returns the artifacts matching every criterion of `query`, ordered by slug then id.
///
/// A query without criteria matches every artifact. `text` is a case-insensitive substring of the
/// slug, title or description; case is folded with Unicode lowercasing.
#[must_use]
pub fn search(snapshot: &RepositorySnapshot, query: &SearchQuery) -> Vec<Artifact> {
    let needle = query.text.as_deref().map(str::to_lowercase);
    let mut found: Vec<Artifact> = snapshot
        .artifacts()
        .iter()
        .filter(|artifact| matches(artifact, query, needle.as_deref()))
        .cloned()
        .collect();
    found.sort_by(|a, b| (a.slug(), a.id()).cmp(&(b.slug(), b.id())));
    found
}

fn matches(artifact: &Artifact, query: &SearchQuery, needle: Option<&str>) -> bool {
    query
        .artifact_type
        .as_ref()
        .is_none_or(|wanted| artifact.artifact_type() == wanted)
        && query
            .tag
            .as_ref()
            .is_none_or(|wanted| artifact.tags().contains(wanted))
        && needle.is_none_or(|needle| text_matches(artifact, needle))
        && query
            .metadata
            .iter()
            .all(|(key, wanted)| metadata_matches(artifact, key, wanted))
}

fn text_matches(artifact: &Artifact, needle: &str) -> bool {
    let contains = |haystack: &str| haystack.to_lowercase().contains(needle);
    contains(artifact.slug().as_str())
        || contains(artifact.title().as_str())
        || artifact.description().is_some_and(contains)
}

/// The key must exist and its value must equal the wanted one; a list value also matches when it
/// contains the wanted string.
fn metadata_matches(artifact: &Artifact, key: &MetadataKey, wanted: &MetadataValue) -> bool {
    match (artifact.metadata().get(key), wanted) {
        (Some(MetadataValue::StringList(items)), MetadataValue::String(wanted)) => {
            items.contains(wanted)
        }
        (Some(actual), wanted) => actual == wanted,
        (None, _) => false,
    }
}

#[cfg(test)]
mod tests {
    use functor_model::{
        ArtifactId, ArtifactType, FormatVersion, Metadata, NewArtifact, RepositoryId,
        RepositoryInfo, RepositoryName, Slug, Tag, Tags, Timestamp, Title,
    };

    use super::*;

    struct Spec {
        slug: &'static str,
        artifact_type: &'static str,
        title: &'static str,
        description: Option<&'static str>,
        tags: &'static [&'static str],
        metadata: Vec<(&'static str, MetadataValue)>,
    }

    fn spec(slug: &'static str) -> Spec {
        Spec {
            slug,
            artifact_type: "document",
            title: slug,
            description: None,
            tags: &[],
            metadata: Vec::new(),
        }
    }

    fn snapshot(specs: Vec<Spec>) -> RepositorySnapshot {
        let now = Timestamp::parse("2026-10-12T18:00:00Z").unwrap();
        let artifacts = specs
            .into_iter()
            .enumerate()
            .map(|(index, spec)| {
                let mut new = NewArtifact::new(
                    Slug::parse(spec.slug).unwrap(),
                    ArtifactType::new(spec.artifact_type).unwrap(),
                    Title::new(spec.title).unwrap(),
                )
                .with_tags(Tags::new(
                    spec.tags
                        .iter()
                        .map(|tag| Tag::new(tag).unwrap())
                        .collect::<Vec<_>>(),
                ))
                .with_metadata(
                    spec.metadata
                        .into_iter()
                        .map(|(key, value)| (MetadataKey::new(key).unwrap(), value))
                        .collect::<Metadata>(),
                );
                if let Some(description) = spec.description {
                    new = new.with_description(description);
                }
                // Ids deliberately run against slug order to prove the ordering rule.
                Artifact::create(ArtifactId::from_bits(100 - index as u128), new, now)
            })
            .collect();
        let info = RepositoryInfo::new(
            RepositoryId::from_bits(1),
            RepositoryName::new("Test").unwrap(),
            None,
            FormatVersion::CURRENT,
            now,
        );
        RepositorySnapshot::new(info, artifacts, Vec::new()).unwrap()
    }

    fn slugs(found: &[Artifact]) -> Vec<String> {
        found.iter().map(|a| a.slug().to_string()).collect()
    }

    fn corpus() -> RepositorySnapshot {
        snapshot(vec![
            Spec {
                artifact_type: "rfc",
                title: "Functor Vision",
                description: Some("Platform goals and non-goals"),
                tags: &["core", "architecture"],
                metadata: vec![
                    ("status", "draft".into()),
                    ("authors", vec!["paulo".to_owned(), "ana".to_owned()].into()),
                    ("priority", 3_i64.into()),
                    ("public", true.into()),
                ],
                ..spec("rfc-0000")
            },
            Spec {
                artifact_type: "rfc",
                title: "Core Semantic Model",
                tags: &["core"],
                metadata: vec![("status", "accepted".into())],
                ..spec("rfc-0001")
            },
            Spec {
                title: "Meeting Notes",
                description: Some("Notes on the VISION discussion"),
                tags: &["notes"],
                ..spec("notes")
            },
            Spec {
                title: "Ünïcode Straße",
                ..spec("unicode")
            },
        ])
    }

    #[test]
    fn no_criteria_matches_everything_ordered_by_slug_then_id() {
        let found = search(&corpus(), &SearchQuery::new());
        assert_eq!(slugs(&found), ["notes", "rfc-0000", "rfc-0001", "unicode"]);
    }

    #[test]
    fn an_empty_repository_has_no_results() {
        let empty = snapshot(Vec::new());
        assert_eq!(search(&empty, &SearchQuery::new()), []);
    }

    #[test]
    fn filters_by_type() {
        let query = SearchQuery::new().with_type(ArtifactType::new("rfc").unwrap());
        assert_eq!(slugs(&search(&corpus(), &query)), ["rfc-0000", "rfc-0001"]);
        let query = SearchQuery::new().with_type(ArtifactType::new("adr").unwrap());
        assert_eq!(search(&corpus(), &query), []);
    }

    #[test]
    fn filters_by_tag() {
        let query = SearchQuery::new().with_tag(Tag::new("core").unwrap());
        assert_eq!(slugs(&search(&corpus(), &query)), ["rfc-0000", "rfc-0001"]);
        let query = SearchQuery::new().with_tag(Tag::new("architecture").unwrap());
        assert_eq!(slugs(&search(&corpus(), &query)), ["rfc-0000"]);
        let query = SearchQuery::new().with_tag(Tag::new("missing").unwrap());
        assert_eq!(search(&corpus(), &query), []);
    }

    #[test]
    fn text_matches_slug_title_and_description_ignoring_case() {
        let find = |text: &str| slugs(&search(&corpus(), &SearchQuery::new().with_text(text)));
        // Slug.
        assert_eq!(find("rfc-000"), ["rfc-0000", "rfc-0001"]);
        // Title, any case.
        assert_eq!(find("SEMANTIC"), ["rfc-0001"]);
        // Description, any case: the title of rfc-0000 and the description of notes both match.
        assert_eq!(find("vision"), ["notes", "rfc-0000"]);
        assert_eq!(find("non-goals"), ["rfc-0000"]);
        assert_eq!(find("no such text"), Vec::<&str>::new());
        // An empty needle matches everything.
        assert_eq!(find("").len(), 4);
    }

    #[test]
    fn text_matching_folds_unicode_case() {
        let find = |text: &str| slugs(&search(&corpus(), &SearchQuery::new().with_text(text)));
        assert_eq!(find("ÜNÏCODE"), ["unicode"]);
        assert_eq!(find("ünïcode"), ["unicode"]);
        assert_eq!(find("straße"), ["unicode"]);
    }

    #[test]
    fn metadata_requires_the_key_and_an_equal_value() {
        let find = |key: &str, value: MetadataValue| {
            slugs(&search(
                &corpus(),
                &SearchQuery::new().with_metadata(MetadataKey::new(key).unwrap(), value),
            ))
        };
        assert_eq!(find("status", "draft".into()), ["rfc-0000"]);
        assert_eq!(find("status", "accepted".into()), ["rfc-0001"]);
        assert_eq!(find("status", "Draft".into()), Vec::<&str>::new());
        assert_eq!(find("missing", "draft".into()), Vec::<&str>::new());
        assert_eq!(find("priority", 3_i64.into()), ["rfc-0000"]);
        assert_eq!(find("public", true.into()), ["rfc-0000"]);
    }

    #[test]
    fn a_string_matches_inside_a_list_value() {
        let find = |value: MetadataValue| {
            slugs(&search(
                &corpus(),
                &SearchQuery::new().with_metadata(MetadataKey::new("authors").unwrap(), value),
            ))
        };
        assert_eq!(find("ana".into()), ["rfc-0000"]);
        assert_eq!(find("paulo".into()), ["rfc-0000"]);
        assert_eq!(find("nobody".into()), Vec::<&str>::new());
        assert_eq!(
            find(vec!["paulo".to_owned(), "ana".to_owned()].into()),
            ["rfc-0000"]
        );
    }

    #[test]
    fn a_string_does_not_match_a_number() {
        let query =
            SearchQuery::new().with_metadata(MetadataKey::new("priority").unwrap(), "3".into());
        assert_eq!(search(&corpus(), &query), []);
    }

    #[test]
    fn criteria_combine_with_and() {
        let query = SearchQuery::new()
            .with_type(ArtifactType::new("rfc").unwrap())
            .with_tag(Tag::new("core").unwrap())
            .with_text("model")
            .with_metadata(MetadataKey::new("status").unwrap(), "accepted".into());
        assert_eq!(slugs(&search(&corpus(), &query)), ["rfc-0001"]);

        let query = SearchQuery::new()
            .with_type(ArtifactType::new("rfc").unwrap())
            .with_metadata(MetadataKey::new("status").unwrap(), "draft".into())
            .with_metadata(MetadataKey::new("priority").unwrap(), 3_i64.into());
        assert_eq!(slugs(&search(&corpus(), &query)), ["rfc-0000"]);

        let query = SearchQuery::new()
            .with_type(ArtifactType::new("rfc").unwrap())
            .with_metadata(MetadataKey::new("status").unwrap(), "draft".into())
            .with_metadata(MetadataKey::new("priority").unwrap(), 4_i64.into());
        assert_eq!(search(&corpus(), &query), []);

        let query = SearchQuery::new()
            .with_tag(Tag::new("notes").unwrap())
            .with_type(ArtifactType::new("rfc").unwrap());
        assert_eq!(search(&corpus(), &query), []);
    }
}
