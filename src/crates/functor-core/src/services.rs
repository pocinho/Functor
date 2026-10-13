//! Service traits for artifacts, relationships, queries, validation and events.

use async_trait::async_trait;
use functor_model::{
    Artifact, ArtifactId, ArtifactPatch, ArtifactRef, Event, NewArtifact, NewEvent,
    NewRelationship, Patched, Query, QueryResult, Relationship, RelationshipId, Revision,
    SearchQuery, Traversal, TraversalResult, ValidationReport,
};

use crate::error::Result;

/// Persistence of artifacts (RFC-0003).
#[async_trait]
pub trait ArtifactRepository: Send + Sync {
    /// Creates an artifact: generates its id, checks slug uniqueness (`Conflict` on a duplicate)
    /// and sets `revision = 1` and `created = updated = now`.
    async fn create(&self, new: NewArtifact) -> Result<Artifact>;

    /// Returns the artifact with this id, if any.
    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>>;

    /// Resolves an id or slug reference, if it names an artifact.
    async fn resolve(&self, reference: &ArtifactRef) -> Result<Option<Artifact>>;

    /// Lists all artifacts ordered by id.
    async fn list(&self) -> Result<Vec<Artifact>>;

    /// Applies a patch.
    ///
    /// Fails with `NotFound` for an unknown id, `Conflict` for a taken slug, and `Conflict`
    /// without writing when `expected` is `Some` and differs from the stored revision. A patch
    /// that changes nothing writes nothing and returns an empty `changed` list.
    async fn update(
        &self,
        id: &ArtifactId,
        patch: ArtifactPatch,
        expected: Option<Revision>,
    ) -> Result<Patched>;
}

/// Persistence of relationships (RFC-0004).
#[async_trait]
pub trait RelationshipRepository: Send + Sync {
    /// Creates a relationship.
    ///
    /// Verifies both endpoints exist (`NotFound`), rejects self-reference for acyclic types
    /// (`InvalidInput`) and an exact duplicate `(source, type, target)` (`Conflict`). It does not
    /// reject cycles across the graph; validation reports those.
    async fn create(&self, new: NewRelationship) -> Result<Relationship>;

    /// Returns the relationship with this id, if any.
    async fn get(&self, id: &RelationshipId) -> Result<Option<Relationship>>;

    /// Lists all relationships ordered by id.
    async fn list(&self) -> Result<Vec<Relationship>>;
}

/// Lookup, search and traversal over a repository (RFC-0006).
#[async_trait]
pub trait QueryEngine: Send + Sync {
    /// Retrieves one artifact by reference.
    async fn lookup(&self, reference: &ArtifactRef) -> Result<Option<Artifact>>;

    /// Searches artifacts, ordered by slug then id.
    async fn search(&self, query: &SearchQuery) -> Result<Vec<Artifact>>;

    /// Traverses relationships. Fails with `NotFound` when the start does not resolve.
    async fn traverse(&self, traversal: &Traversal) -> Result<TraversalResult>;

    /// Executes any query by dispatching to the operations above.
    async fn execute(&self, query: Query) -> Result<QueryResult> {
        match query {
            Query::Lookup(reference) => self.lookup(&reference).await.map(QueryResult::Lookup),
            Query::Search(search) => self.search(&search).await.map(QueryResult::Search),
            Query::Traverse(traversal) => {
                self.traverse(&traversal).await.map(QueryResult::Traverse)
            }
            // `Query` is non-exhaustive; a variant this engine does not know is a bug.
            _ => Err(crate::FunctorError::Internal(
                "unsupported query type".to_owned(),
            )),
        }
    }
}

/// What to validate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationTarget {
    /// The whole repository.
    Repository,
    /// Findings about one artifact and the relationships touching it. Rules still run on the whole
    /// repository, because uniqueness and reachability need it.
    Artifact(ArtifactRef),
}

/// Validation of repository data (RFC-0007).
#[async_trait]
pub trait ValidationEngine: Send + Sync {
    /// Validates the target.
    ///
    /// A run that finds problems succeeds and reports them. Persisting the report and publishing
    /// `validation.completed` is the caller's job.
    async fn validate(&self, target: ValidationTarget) -> Result<ValidationReport>;
}

/// Publishes events (RFC-0017).
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Assigns an id, a sequence number and a timestamp, appends the event and returns it.
    async fn publish(&self, event: NewEvent) -> Result<Event>;
}

/// Where a subscription starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EventPosition {
    /// The oldest retained event.
    Start,
    /// Only events published after the subscription starts.
    End,
    /// Events whose sequence number is greater than this one.
    After(u64),
}

/// A source of events that can be followed.
#[async_trait]
pub trait EventSource: Send + Sync {
    /// Starts a subscription at `from`.
    async fn subscribe(&self, from: EventPosition) -> Result<Box<dyn EventSubscription>>;
}

/// A live stream of events in sequence order.
#[async_trait]
pub trait EventSubscription: Send {
    /// Returns the next event, waiting if none is available. `Ok(None)` means the source closed.
    async fn next(&mut self) -> Result<Option<Event>>;
}
