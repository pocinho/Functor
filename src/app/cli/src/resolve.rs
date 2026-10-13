//! Turning what a user typed into an artifact.
//!
//! See `docs/identity-and-addressing.md`, "Resolution on the Command Line".

use functor_core::{ArtifactRepository, FunctorError, RelationshipRepository, Result};
use functor_model::{Artifact, ArtifactRef, EntityKind, Relationship, RelationshipId};

const MAX_SUGGESTIONS: usize = 3;
const MAX_DISTANCE: usize = 2;

/// Resolves an id, canonical reference or slug to an artifact.
///
/// A malformed id or slug is an input error; an unknown one is `NotFound` with the closest slugs
/// as suggestions.
///
/// # Errors
/// `InvalidInput`, `NotFound`, or whatever the store reports.
pub async fn artifact(store: &dyn ArtifactRepository, text: &str) -> Result<Artifact> {
    let reference = ArtifactRef::parse(text)?;
    if let Some(found) = store.resolve(&reference).await? {
        return Ok(found);
    }
    let suggestions = match &reference {
        ArtifactRef::Slug(slug) => {
            let artifacts = store.list().await?;
            suggest(slug.as_str(), artifacts.iter().map(|a| a.slug().as_str()))
        }
        ArtifactRef::Id(_) => Vec::new(),
    };
    Err(FunctorError::NotFound {
        entity: EntityKind::Artifact,
        reference: text.to_owned(),
        suggestions,
    })
}

/// Resolves a relationship id or canonical reference to a relationship.
///
/// Relationships have no slug, so anything that is not an id or canonical reference is an input
/// error.
///
/// # Errors
/// `InvalidInput`, `NotFound`, or whatever the store reports.
pub async fn relationship(store: &dyn RelationshipRepository, text: &str) -> Result<Relationship> {
    let id = if text.starts_with("functor:") {
        RelationshipId::from_canonical(text)?
    } else if text.starts_with(RelationshipId::PREFIX) {
        text.parse::<RelationshipId>()?
    } else {
        return Err(FunctorError::invalid_input(
            "relationship",
            format!("`{text}` is not a relationship id (`rel_...`) or canonical reference"),
        ));
    };
    store.get(&id).await?.ok_or_else(|| FunctorError::NotFound {
        entity: EntityKind::Relationship,
        reference: text.to_owned(),
        suggestions: Vec::new(),
    })
}

/// The slugs closest to `wanted` (edit distance of at most 2), nearest first.
fn suggest<'a>(wanted: &str, slugs: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut close: Vec<(usize, &str)> = slugs
        .filter_map(|slug| {
            let distance = edit_distance(wanted, slug);
            (distance <= MAX_DISTANCE).then_some((distance, slug))
        })
        .collect();
    close.sort_unstable();
    close
        .into_iter()
        .take(MAX_SUGGESTIONS)
        .map(|(_, slug)| slug.to_owned())
        .collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != *cb);
            current.push(substitution.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use functor_core::ErrorKind;
    use functor_model::{
        ArtifactType, NewArtifact, NewRelationship, RelationshipType, Slug, Title,
    };
    use functor_store::MemoryStore;
    use functor_testkit::Env;

    use super::*;

    async fn store_with(slugs: &[&str]) -> MemoryStore {
        let env = Env::new();
        let store = MemoryStore::new(env.clock(), env.ids());
        for slug in slugs {
            ArtifactRepository::create(
                &store,
                NewArtifact::new(
                    Slug::parse(slug).unwrap(),
                    ArtifactType::document(),
                    Title::new(slug).unwrap(),
                ),
            )
            .await
            .unwrap();
        }
        store
    }

    #[test]
    fn edit_distance_basics() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("abc", "abd"), 1);
        assert_eq!(edit_distance("abc", "ab"), 1);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
    }

    #[test]
    fn suggestions_are_close_nearest_first_and_capped() {
        let slugs = [
            "rfc-0000", "rfc-0001", "rfc-0010", "rfc-0100", "other", "rfc-9999",
        ];
        let found = suggest("rfc-0002", slugs.iter().copied());
        assert_eq!(found, ["rfc-0000", "rfc-0001", "rfc-0010"]);
        assert_eq!(
            suggest("zzzzzzzz", slugs.iter().copied()),
            Vec::<String>::new()
        );
    }

    #[tokio::test]
    async fn slugs_ids_and_canonical_references_resolve() {
        let store = store_with(&["alpha", "beta"]).await;
        let alpha = artifact(&store, "alpha").await.unwrap();
        assert_eq!(
            artifact(&store, &alpha.id().to_string()).await.unwrap(),
            alpha
        );
        assert_eq!(
            artifact(&store, &alpha.id().canonical()).await.unwrap(),
            alpha
        );
    }

    #[tokio::test]
    async fn unknown_slugs_are_not_found_with_suggestions() {
        let store = store_with(&["alpha", "beta"]).await;
        let error = artifact(&store, "alpah").await.unwrap_err();
        match error {
            FunctorError::NotFound {
                entity,
                reference,
                suggestions,
            } => {
                assert_eq!(entity, EntityKind::Artifact);
                assert_eq!(reference, "alpah");
                assert_eq!(suggestions, ["alpha"]);
            }
            other => panic!("expected not found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unknown_ids_are_not_found_without_suggestions() {
        let store = store_with(&["alpha"]).await;
        let id = functor_model::ArtifactId::from_bits(999).to_string();
        match artifact(&store, &id).await.unwrap_err() {
            FunctorError::NotFound { suggestions, .. } => {
                assert_eq!(suggestions, Vec::<String>::new());
            }
            other => panic!("expected not found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn malformed_references_are_input_errors_not_lookups() {
        let store = store_with(&["alpha"]).await;
        for bad in ["art_xyz", "functor:artifact:xyz", "Not A Slug", "", "-x"] {
            let error = artifact(&store, bad).await.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidInput, "{bad:?}");
        }
    }

    async fn related() -> (MemoryStore, Relationship) {
        let store = store_with(&["alpha", "beta"]).await;
        let alpha = artifact(&store, "alpha").await.unwrap();
        let beta = artifact(&store, "beta").await.unwrap();
        let created = RelationshipRepository::create(
            &store,
            NewRelationship::new(RelationshipType::References, *alpha.id(), *beta.id()),
        )
        .await
        .unwrap();
        (store, created)
    }

    #[tokio::test]
    async fn relationships_resolve_by_id_and_canonical_reference() {
        let (store, created) = related().await;
        assert_eq!(
            relationship(&store, &created.id().to_string())
                .await
                .unwrap(),
            created
        );
        assert_eq!(
            relationship(&store, &created.id().canonical())
                .await
                .unwrap(),
            created
        );
    }

    #[tokio::test]
    async fn unknown_relationships_are_not_found() {
        let (store, _) = related().await;
        let id = RelationshipId::from_bits(999);
        for text in [id.to_string(), id.canonical()] {
            match relationship(&store, &text).await.unwrap_err() {
                FunctorError::NotFound {
                    entity, reference, ..
                } => {
                    assert_eq!(entity, EntityKind::Relationship);
                    assert_eq!(reference, text);
                }
                other => panic!("expected not found, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn relationship_references_without_ids_are_input_errors() {
        let (store, created) = related().await;
        let artifact_ref = functor_model::ArtifactId::from_bits(1).canonical();
        for bad in [
            "alpha",
            "rel_xyz",
            "functor:relationship:xyz",
            "",
            artifact_ref.as_str(),
        ] {
            let error = relationship(&store, bad).await.unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidInput, "{bad:?}");
        }
        // Using an artifact id where a relationship is expected is also an input error.
        let error = relationship(&store, &created.source().to_string())
            .await
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
    }
}
