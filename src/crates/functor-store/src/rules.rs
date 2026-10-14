//! Semantic rules shared by every store.
//!
//! Each function checks an operation against a consistent snapshot and returns the entity to
//! persist. Stores differ only in where the snapshot comes from and how the result is saved, so
//! the rules cannot drift between `MemoryStore` and `FileStore`.

use functor_core::{FunctorError, Result};
use functor_model::{
    Artifact, ArtifactId, ArtifactPatch, EntityKind, EvidenceKind, NewArtifact, NewRelationship,
    Patched, Relationship, RelationshipId, RepositorySnapshot, Revision, Timestamp,
};

/// Checks slug uniqueness and builds the new artifact (revision 1, `created = updated = now`).
pub(crate) fn plan_create_artifact(
    snapshot: &RepositorySnapshot,
    id: ArtifactId,
    new: NewArtifact,
    now: Timestamp,
) -> Result<Artifact> {
    if snapshot.artifact_by_slug(&new.slug).is_some() {
        return Err(slug_taken(&new.slug));
    }
    Ok(Artifact::create(id, new, now))
}

/// Applies a patch with the documented checks: existence, expected revision, slug uniqueness.
pub(crate) fn plan_update_artifact(
    snapshot: &RepositorySnapshot,
    id: &ArtifactId,
    patch: ArtifactPatch,
    expected: Option<Revision>,
    now: Timestamp,
) -> Result<Patched> {
    let current = snapshot
        .artifact(id)
        .ok_or_else(|| FunctorError::not_found(EntityKind::Artifact, id.to_string()))?;

    if let Some(kind) = EvidenceKind::of(current) {
        return Err(FunctorError::conflict(
            EntityKind::Artifact,
            format!(
                "`{}` is {kind} evidence, which is immutable (ADR-0013)",
                current.slug()
            ),
        ));
    }
    if let Some(expected) = expected
        && expected != current.revision()
    {
        return Err(FunctorError::conflict(
            EntityKind::Artifact,
            format!(
                "expected revision {expected}, but `{}` is at revision {}",
                current.slug(),
                current.revision()
            ),
        ));
    }
    if let Some(slug) = &patch.slug
        && snapshot
            .artifact_by_slug(slug)
            .is_some_and(|owner| owner.id() != id)
    {
        return Err(slug_taken(slug));
    }
    Ok(current.apply_patch(patch, now)?)
}

/// Checks endpoints, self-reference and duplicates, and builds the new relationship.
pub(crate) fn plan_create_relationship(
    snapshot: &RepositorySnapshot,
    id: RelationshipId,
    new: NewRelationship,
    now: Timestamp,
) -> Result<Relationship> {
    for endpoint in [new.source, new.target] {
        if snapshot.artifact(&endpoint).is_none() {
            return Err(FunctorError::not_found(
                EntityKind::Artifact,
                endpoint.to_string(),
            ));
        }
    }
    let relationship = Relationship::create(id, new, now)?;
    let duplicate = snapshot.outgoing(relationship.source()).any(|existing| {
        existing.target() == relationship.target()
            && existing.relationship_type() == relationship.relationship_type()
    });
    if duplicate {
        return Err(FunctorError::conflict(
            EntityKind::Relationship,
            format!(
                "`{}` already relates {} to {}",
                relationship.relationship_type(),
                relationship.source(),
                relationship.target()
            ),
        ));
    }
    Ok(relationship)
}

fn slug_taken(slug: &functor_model::Slug) -> FunctorError {
    FunctorError::conflict(
        EntityKind::Artifact,
        format!("slug `{slug}` is already taken"),
    )
}
