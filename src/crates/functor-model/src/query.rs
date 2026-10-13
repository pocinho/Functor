//! Query value types (RFC-0006). Execution lives in `functor-query`.
//!
//! See `docs/query-model-mvp.md`.

use crate::artifact::{Artifact, ArtifactType, MetadataKey, MetadataValue, Tag};
use crate::id::ArtifactRef;
use crate::relationship::{Relationship, RelationshipType};

/// Which way a traversal follows relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Direction {
    /// From source to target.
    Outbound,
    /// From target to source.
    Inbound,
    /// Both ways.
    Both,
}

/// Criteria for searching artifacts. All present criteria combine with AND.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct SearchQuery {
    /// Match this artifact type.
    pub artifact_type: Option<ArtifactType>,
    /// Match artifacts carrying this tag.
    pub tag: Option<Tag>,
    /// Case-insensitive substring of slug, title or description.
    pub text: Option<String>,
    /// Metadata entries that must all match.
    pub metadata: Vec<(MetadataKey, MetadataValue)>,
}

impl SearchQuery {
    /// A query with no criteria, matching every artifact.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Restricts to a type.
    #[must_use]
    pub fn with_type(mut self, artifact_type: ArtifactType) -> Self {
        self.artifact_type = Some(artifact_type);
        self
    }

    /// Restricts to a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: Tag) -> Self {
        self.tag = Some(tag);
        self
    }

    /// Restricts to a text substring.
    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Adds a metadata criterion.
    #[must_use]
    pub fn with_metadata(mut self, key: MetadataKey, value: MetadataValue) -> Self {
        self.metadata.push((key, value));
        self
    }
}

/// A relationship traversal from a starting artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Traversal {
    /// Where to start.
    pub start: ArtifactRef,
    /// Relationship types to follow; empty means all.
    pub relationship_types: Vec<RelationshipType>,
    /// Which way to follow edges.
    pub direction: Direction,
    /// Maximum depth; `None` means unlimited.
    pub max_depth: Option<u32>,
}

impl Traversal {
    /// An unlimited, all-types, outbound traversal from `start`.
    #[must_use]
    pub fn new(start: ArtifactRef) -> Self {
        Self {
            start,
            relationship_types: Vec::new(),
            direction: Direction::Outbound,
            max_depth: None,
        }
    }

    /// Restricts to the given relationship types.
    #[must_use]
    pub fn with_types(mut self, types: impl IntoIterator<Item = RelationshipType>) -> Self {
        self.relationship_types = types.into_iter().collect();
        self
    }

    /// Sets the direction.
    #[must_use]
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// Limits the depth.
    #[must_use]
    pub fn with_max_depth(mut self, max_depth: u32) -> Self {
        self.max_depth = Some(max_depth);
        self
    }
}

/// Any query the engine can execute.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Query {
    /// Retrieve one artifact by reference.
    Lookup(ArtifactRef),
    /// Search artifacts.
    Search(SearchQuery),
    /// Traverse relationships.
    Traverse(Traversal),
}

/// An artifact reached by a traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalNode {
    /// The reached artifact.
    pub artifact: Artifact,
    /// The minimum depth at which it was reached (the start is depth 0 and not reported).
    pub depth: u32,
    /// The first edge that reached it.
    pub via: Relationship,
}

/// The outcome of a traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraversalResult {
    /// The starting artifact.
    pub start: Artifact,
    /// Reached artifacts, excluding the start, ordered by depth, slug, then id.
    pub nodes: Vec<TraversalNode>,
    /// Every distinct edge examined within the depth limit, sorted by relationship id.
    pub edges: Vec<Relationship>,
}

/// The outcome of [`Query`] execution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueryResult {
    /// The result of a lookup.
    Lookup(Option<Artifact>),
    /// The result of a search, ordered by slug then id.
    Search(Vec<Artifact>),
    /// The result of a traversal.
    Traverse(TraversalResult),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders_set_fields() {
        let query = SearchQuery::new()
            .with_type(ArtifactType::document())
            .with_tag(Tag::new("core").unwrap())
            .with_text("vision")
            .with_metadata(MetadataKey::new("status").unwrap(), "draft".into());
        assert!(query.artifact_type.is_some() && query.tag.is_some());
        assert_eq!(query.text.as_deref(), Some("vision"));
        assert_eq!(query.metadata.len(), 1);
        assert_eq!(SearchQuery::new(), SearchQuery::default());
    }

    #[test]
    fn traversal_defaults_and_builders() {
        let start = ArtifactRef::parse("rfc-0001").unwrap();
        let t = Traversal::new(start.clone());
        assert_eq!(t.direction, Direction::Outbound);
        assert_eq!(t.max_depth, None);
        assert_eq!(t.relationship_types, []);
        let t = t
            .with_types([RelationshipType::DependsOn])
            .with_direction(Direction::Inbound)
            .with_max_depth(2);
        assert_eq!(t.relationship_types, [RelationshipType::DependsOn]);
        assert_eq!(t.direction, Direction::Inbound);
        assert_eq!(t.max_depth, Some(2));
        assert_eq!(t.start, start);
    }
}
