//! Functor semantic model: identifiers, artifacts, relationships and value types.
//!
//! See `docs/design/domain-model.md`. This crate has no I/O, no runtime and no serialization framework.
//! Invariants are enforced by validating constructors, so an invalid value cannot be built.

mod artifact;
mod error;
mod event;
mod grammar;
mod id;
mod query;
mod relationship;
mod repository;
mod timestamp;
mod validation;

pub use artifact::{
    Artifact, ArtifactField, ArtifactPatch, ArtifactType, Content, ContentFormat, ContentPath,
    Metadata, MetadataKey, MetadataValue, NewArtifact, Patched, Revision, Tag, Tags, Title,
};
pub use error::ModelError;
pub use event::{Event, EventData, EventKind, EventSubject, NewEvent};
pub use id::{
    ArtifactId, ArtifactRef, EntityKind, EventId, MonotonicUlids, RelationshipId, RepositoryId,
    Slug,
};
pub use query::{
    Direction, Query, QueryResult, SearchQuery, Traversal, TraversalNode, TraversalResult,
};
pub use relationship::{NewRelationship, Relationship, RelationshipType, RelationshipTypeName};
pub use repository::{
    FormatVersion, RepoPath, RepositoryInfo, RepositoryName, RepositorySnapshot, SnapshotError,
};
pub use timestamp::Timestamp;
pub use validation::{Counts, Finding, Outcome, RuleCode, Severity, Subject, ValidationReport};
