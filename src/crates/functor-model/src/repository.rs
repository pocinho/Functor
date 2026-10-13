//! Repository metadata and the immutable in-memory snapshot used by queries and validation.
//!
//! See `docs/domain-model.md` and RFC-0015.

use std::collections::BTreeMap;
use std::fmt;

use crate::artifact::Artifact;
use crate::error::{ModelError, Result};
use crate::grammar::check_text_line;
use crate::id::{ArtifactId, ArtifactRef, RelationshipId, RepositoryId, Slug};
use crate::relationship::Relationship;
use crate::timestamp::Timestamp;

/// A repository name: trimmed, non-empty, at most 100 characters, without control characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepositoryName(String);

impl RepositoryName {
    /// The maximum number of characters.
    pub const MAX_CHARS: usize = 100;

    /// Validates the text and stores it trimmed.
    ///
    /// # Errors
    /// Fails when the name is empty, too long or contains control characters.
    pub fn new(input: &str) -> Result<Self> {
        check_text_line("repository name", input, Self::MAX_CHARS).map(Self)
    }

    /// The name text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RepositoryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The on-disk format version of a repository. The MVP value is [`FormatVersion::CURRENT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FormatVersion(u32);

impl FormatVersion {
    /// The version written and supported by this release.
    pub const CURRENT: Self = Self(1);

    /// Wraps a version number.
    #[must_use]
    pub fn new(number: u32) -> Self {
        Self(number)
    }

    /// The version number.
    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

impl fmt::Display for FormatVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Repository identity and metadata (RFC-0015).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryInfo {
    id: RepositoryId,
    name: RepositoryName,
    description: Option<String>,
    format_version: FormatVersion,
    created: Timestamp,
}

impl RepositoryInfo {
    /// Assembles repository information.
    #[must_use]
    pub fn new(
        id: RepositoryId,
        name: RepositoryName,
        description: Option<String>,
        format_version: FormatVersion,
        created: Timestamp,
    ) -> Self {
        Self {
            id,
            name,
            description,
            format_version,
            created,
        }
    }

    /// The identity.
    #[must_use]
    pub fn id(&self) -> &RepositoryId {
        &self.id
    }

    /// The name.
    #[must_use]
    pub fn name(&self) -> &RepositoryName {
        &self.name
    }

    /// The description, if any.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The format version.
    #[must_use]
    pub fn format_version(&self) -> FormatVersion {
        self.format_version
    }

    /// When the repository was created.
    #[must_use]
    pub fn created(&self) -> Timestamp {
        self.created
    }
}

/// A path relative to the repository root, using `/` separators.
///
/// Unlike `ContentPath`, this can name any file found in a repository, including ones that
/// violate the layout, so it only requires a non-empty, NUL-free, relative-looking string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RepoPath(String);

impl RepoPath {
    /// Wraps a path, converting `\` to `/`.
    ///
    /// # Errors
    /// Fails when the path is empty or contains a NUL character.
    pub fn new(input: &str) -> Result<Self> {
        if input.is_empty() {
            return Err(ModelError::new("repository path", "must not be empty"));
        }
        if input.contains('\0') {
            return Err(ModelError::new("repository path", "must not contain NUL"));
        }
        Ok(Self(input.replace('\\', "/")))
    }

    /// The path text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RepoPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a set of entities cannot form a well-formed snapshot.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SnapshotError {
    /// Two artifacts share an id.
    #[error("duplicate artifact id {0}")]
    DuplicateArtifactId(ArtifactId),
    /// Two artifacts share a slug.
    #[error("duplicate slug {0}")]
    DuplicateSlug(Slug),
    /// Two relationships share an id.
    #[error("duplicate relationship id {0}")]
    DuplicateRelationshipId(RelationshipId),
    /// A relationship's source artifact does not exist.
    #[error("relationship {relationship} has a missing source {artifact}")]
    DanglingSource {
        /// The offending relationship.
        relationship: RelationshipId,
        /// The missing artifact.
        artifact: ArtifactId,
    },
    /// A relationship's target artifact does not exist.
    #[error("relationship {relationship} has a missing target {artifact}")]
    DanglingTarget {
        /// The offending relationship.
        relationship: RelationshipId,
        /// The missing artifact.
        artifact: ArtifactId,
    },
}

/// An immutable, consistent in-memory view of a repository.
///
/// A snapshot is well-formed: ids and slugs are unique and every relationship endpoint exists.
/// Entities are sorted by id, and indexes are built once. Queries and validation run over
/// snapshots.
#[derive(Debug, Clone)]
pub struct RepositorySnapshot {
    info: RepositoryInfo,
    artifacts: Vec<Artifact>,
    relationships: Vec<Relationship>,
    by_id: BTreeMap<ArtifactId, usize>,
    by_slug: BTreeMap<Slug, usize>,
    outgoing: BTreeMap<ArtifactId, Vec<usize>>,
    incoming: BTreeMap<ArtifactId, Vec<usize>>,
}

impl RepositorySnapshot {
    /// Builds a snapshot, sorting entities by id and checking well-formedness.
    ///
    /// # Errors
    /// Fails on duplicate ids or slugs, or a relationship whose source or target is missing.
    /// When several problems exist, the first found in id order is reported.
    pub fn new(
        info: RepositoryInfo,
        mut artifacts: Vec<Artifact>,
        mut relationships: Vec<Relationship>,
    ) -> std::result::Result<Self, SnapshotError> {
        artifacts.sort_by_key(|a| *a.id());
        relationships.sort_by_key(|r| *r.id());

        let mut by_id = BTreeMap::new();
        let mut by_slug = BTreeMap::new();
        for (index, artifact) in artifacts.iter().enumerate() {
            if by_id.insert(*artifact.id(), index).is_some() {
                return Err(SnapshotError::DuplicateArtifactId(*artifact.id()));
            }
            if by_slug.insert(artifact.slug().clone(), index).is_some() {
                return Err(SnapshotError::DuplicateSlug(artifact.slug().clone()));
            }
        }

        let mut seen = std::collections::BTreeSet::new();
        let mut outgoing: BTreeMap<ArtifactId, Vec<usize>> = BTreeMap::new();
        let mut incoming: BTreeMap<ArtifactId, Vec<usize>> = BTreeMap::new();
        for (index, relationship) in relationships.iter().enumerate() {
            if !seen.insert(*relationship.id()) {
                return Err(SnapshotError::DuplicateRelationshipId(*relationship.id()));
            }
            if !by_id.contains_key(relationship.source()) {
                return Err(SnapshotError::DanglingSource {
                    relationship: *relationship.id(),
                    artifact: *relationship.source(),
                });
            }
            if !by_id.contains_key(relationship.target()) {
                return Err(SnapshotError::DanglingTarget {
                    relationship: *relationship.id(),
                    artifact: *relationship.target(),
                });
            }
            outgoing
                .entry(*relationship.source())
                .or_default()
                .push(index);
            incoming
                .entry(*relationship.target())
                .or_default()
                .push(index);
        }

        Ok(Self {
            info,
            artifacts,
            relationships,
            by_id,
            by_slug,
            outgoing,
            incoming,
        })
    }

    /// The repository information.
    #[must_use]
    pub fn info(&self) -> &RepositoryInfo {
        &self.info
    }

    /// All artifacts, sorted by id.
    #[must_use]
    pub fn artifacts(&self) -> &[Artifact] {
        &self.artifacts
    }

    /// All relationships, sorted by id.
    #[must_use]
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    /// Looks up an artifact by id.
    #[must_use]
    pub fn artifact(&self, id: &ArtifactId) -> Option<&Artifact> {
        self.by_id.get(id).map(|&i| &self.artifacts[i])
    }

    /// Looks up an artifact by slug.
    #[must_use]
    pub fn artifact_by_slug(&self, slug: &Slug) -> Option<&Artifact> {
        self.by_slug.get(slug).map(|&i| &self.artifacts[i])
    }

    /// Resolves an id or slug reference.
    #[must_use]
    pub fn resolve(&self, reference: &ArtifactRef) -> Option<&Artifact> {
        match reference {
            ArtifactRef::Id(id) => self.artifact(id),
            ArtifactRef::Slug(slug) => self.artifact_by_slug(slug),
        }
    }

    /// Relationships whose source is `id`, sorted by relationship id.
    pub fn outgoing(&self, id: &ArtifactId) -> impl Iterator<Item = &Relationship> {
        self.outgoing
            .get(id)
            .into_iter()
            .flatten()
            .map(|&i| &self.relationships[i])
    }

    /// Relationships whose target is `id`, sorted by relationship id.
    pub fn incoming(&self, id: &ArtifactId) -> impl Iterator<Item = &Relationship> {
        self.incoming
            .get(id)
            .into_iter()
            .flatten()
            .map(|&i| &self.relationships[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{ArtifactType, NewArtifact, Title};
    use crate::relationship::{NewRelationship, RelationshipType};

    fn now() -> Timestamp {
        Timestamp::parse("2026-10-12T18:00:00Z").unwrap()
    }

    fn info() -> RepositoryInfo {
        RepositoryInfo::new(
            RepositoryId::from_bits(1),
            RepositoryName::new("Demo").unwrap(),
            None,
            FormatVersion::CURRENT,
            now(),
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
            now(),
        )
    }

    fn rel(bits: u128, kind: RelationshipType, source: u128, target: u128) -> Relationship {
        Relationship::create(
            RelationshipId::from_bits(bits),
            NewRelationship::new(
                kind,
                ArtifactId::from_bits(source),
                ArtifactId::from_bits(target),
            ),
            now(),
        )
        .unwrap()
    }

    #[test]
    fn repository_name_rules() {
        assert_eq!(RepositoryName::new("  Demo ").unwrap().as_str(), "Demo");
        assert!(RepositoryName::new("").is_err());
        assert!(RepositoryName::new(&"x".repeat(101)).is_err());
        assert!(RepositoryName::new("a\tb").is_err());
    }

    #[test]
    fn repo_path_normalizes_separators() {
        assert_eq!(
            RepoPath::new("artifacts\\x.toml").unwrap().as_str(),
            "artifacts/x.toml"
        );
        assert!(RepoPath::new("").is_err());
        assert!(RepoPath::new("a\0b").is_err());
    }

    #[test]
    fn snapshot_sorts_and_indexes() {
        let snapshot = RepositorySnapshot::new(
            info(),
            vec![artifact(3, "c"), artifact(1, "a"), artifact(2, "b")],
            vec![
                rel(20, RelationshipType::DependsOn, 3, 1),
                rel(10, RelationshipType::DependsOn, 2, 1),
            ],
        )
        .unwrap();
        let slugs: Vec<&str> = snapshot
            .artifacts()
            .iter()
            .map(|a| a.slug().as_str())
            .collect();
        assert_eq!(slugs, ["a", "b", "c"]);
        let one = ArtifactId::from_bits(1);
        let incoming: Vec<RelationshipId> = snapshot.incoming(&one).map(|r| *r.id()).collect();
        assert_eq!(
            incoming,
            [RelationshipId::from_bits(10), RelationshipId::from_bits(20)]
        );
        assert_eq!(snapshot.outgoing(&one).count(), 0);
        assert_eq!(snapshot.outgoing(&ArtifactId::from_bits(3)).count(), 1);
        assert_eq!(snapshot.outgoing(&ArtifactId::from_bits(99)).count(), 0);
    }

    #[test]
    fn snapshot_resolves_ids_and_slugs() {
        let snapshot = RepositorySnapshot::new(info(), vec![artifact(1, "a")], Vec::new()).unwrap();
        let by_slug = snapshot.resolve(&ArtifactRef::parse("a").unwrap()).unwrap();
        let by_id = snapshot
            .resolve(&ArtifactRef::Id(ArtifactId::from_bits(1)))
            .unwrap();
        assert_eq!(by_slug, by_id);
        assert!(
            snapshot
                .resolve(&ArtifactRef::parse("zzz").unwrap())
                .is_none()
        );
    }

    #[test]
    fn snapshot_rejects_duplicates() {
        let dup_id =
            RepositorySnapshot::new(info(), vec![artifact(1, "a"), artifact(1, "b")], vec![]);
        assert!(matches!(dup_id, Err(SnapshotError::DuplicateArtifactId(_))));
        let dup_slug =
            RepositorySnapshot::new(info(), vec![artifact(1, "a"), artifact(2, "a")], vec![]);
        assert!(matches!(dup_slug, Err(SnapshotError::DuplicateSlug(_))));
        let dup_rel = RepositorySnapshot::new(
            info(),
            vec![artifact(1, "a"), artifact(2, "b")],
            vec![
                rel(5, RelationshipType::References, 1, 2),
                rel(5, RelationshipType::References, 2, 1),
            ],
        );
        assert!(matches!(
            dup_rel,
            Err(SnapshotError::DuplicateRelationshipId(_))
        ));
    }

    #[test]
    fn snapshot_rejects_dangling_endpoints() {
        let dangling_target = RepositorySnapshot::new(
            info(),
            vec![artifact(1, "a")],
            vec![rel(5, RelationshipType::References, 1, 2)],
        );
        assert!(matches!(
            dangling_target,
            Err(SnapshotError::DanglingTarget { .. })
        ));
        let dangling_source = RepositorySnapshot::new(
            info(),
            vec![artifact(2, "b")],
            vec![rel(5, RelationshipType::References, 1, 2)],
        );
        assert!(matches!(
            dangling_source,
            Err(SnapshotError::DanglingSource { .. })
        ));
    }
}
