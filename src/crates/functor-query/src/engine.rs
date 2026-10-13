//! The asynchronous engine: one snapshot per call, then a pure function.

use async_trait::async_trait;
use functor_core::{QueryEngine, RepositoryReader, Result};
use functor_model::{
    Artifact, ArtifactRef, Direction, Relationship, RelationshipType, SearchQuery, Traversal,
    TraversalResult,
};

use crate::{relationships_of, search, traverse};

/// A [`QueryEngine`] that answers queries from a fresh strict snapshot of any
/// [`RepositoryReader`].
///
/// Every call reads one consistent snapshot and runs the pure functions of this crate over it;
/// nothing is cached between calls. An inconsistent repository makes the call fail with
/// `Inconsistent`.
#[derive(Debug)]
pub struct SnapshotQueryEngine<'a, R: ?Sized> {
    reader: &'a R,
}

impl<'a, R: RepositoryReader + ?Sized> SnapshotQueryEngine<'a, R> {
    /// Creates an engine over `reader`.
    #[must_use]
    pub fn new(reader: &'a R) -> Self {
        Self { reader }
    }

    /// Every relationship touching the artifact, ordered by relationship id, together with the
    /// artifact itself. See [`relationships_of`].
    ///
    /// # Errors
    /// `NotFound` for an unknown artifact, or the reader's error for an inconsistent repository.
    pub async fn relationships(
        &self,
        reference: &ArtifactRef,
        types: &[RelationshipType],
        direction: Direction,
    ) -> Result<(Artifact, Vec<Relationship>)> {
        let snapshot = self.reader.snapshot().await?;
        relationships_of(&snapshot, reference, types, direction)
    }
}

#[async_trait]
impl<R: RepositoryReader + ?Sized> QueryEngine for SnapshotQueryEngine<'_, R> {
    async fn lookup(&self, reference: &ArtifactRef) -> Result<Option<Artifact>> {
        let snapshot = self.reader.snapshot().await?;
        Ok(snapshot.resolve(reference).cloned())
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<Artifact>> {
        let snapshot = self.reader.snapshot().await?;
        Ok(search(&snapshot, query))
    }

    async fn traverse(&self, traversal: &Traversal) -> Result<TraversalResult> {
        let snapshot = self.reader.snapshot().await?;
        traverse(&snapshot, traversal)
    }
}
