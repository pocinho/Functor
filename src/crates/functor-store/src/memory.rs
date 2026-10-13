//! An in-memory store for tests, examples and as the second implementation in the conformance
//! suite.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use functor_core::{
    ArtifactRepository, Clock, IdGenerator, RelationshipRepository, RepositoryReader, Result,
    SnapshotLoad,
};
use functor_model::{
    Artifact, ArtifactId, ArtifactPatch, ArtifactRef, FormatVersion, NewArtifact, NewRelationship,
    Patched, Relationship, RelationshipId, RepositoryInfo, RepositoryName, RepositorySnapshot,
    Revision,
};

use crate::rules;

#[derive(Debug, Default)]
struct State {
    artifacts: Vec<Artifact>,
    relationships: Vec<Relationship>,
}

/// A repository held entirely in memory.
///
/// It implements the same traits and rules as `FileStore` and passes the same conformance suite,
/// but nothing is persisted. It does not implement `RepositoryManager`: there is nothing to
/// initialize or discover.
pub struct MemoryStore {
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    info: RepositoryInfo,
    state: Mutex<State>,
}

impl std::fmt::Debug for MemoryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryStore")
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

impl MemoryStore {
    /// Creates an empty repository named `Memory`, with a new id and the current time.
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>, ids: Arc<dyn IdGenerator>) -> Self {
        let name = RepositoryName::new("Memory")
            .unwrap_or_else(|_| unreachable!("`Memory` is a valid repository name"));
        let info = RepositoryInfo::new(
            ids.repository_id(),
            name,
            None,
            FormatVersion::CURRENT,
            clock.now(),
        );
        Self::with_info(info, clock, ids)
    }

    /// Creates an empty repository with the given information.
    #[must_use]
    pub fn with_info(
        info: RepositoryInfo,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            clock,
            ids,
            info,
            state: Mutex::default(),
        }
    }

    /// Creates a store pre-filled with the entities of `snapshot` (for example from a fixture).
    #[must_use]
    pub fn from_snapshot(
        snapshot: &RepositorySnapshot,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        let store = Self::with_info(snapshot.info().clone(), clock, ids);
        {
            let mut state = store.state();
            state.artifacts = snapshot.artifacts().to_vec();
            state.relationships = snapshot.relationships().to_vec();
        }
        store
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn snapshot_of(&self, state: &State) -> Result<RepositorySnapshot> {
        Ok(RepositorySnapshot::new(
            self.info.clone(),
            state.artifacts.clone(),
            state.relationships.clone(),
        )?)
    }
}

#[async_trait]
impl RepositoryReader for MemoryStore {
    async fn info(&self) -> Result<RepositoryInfo> {
        Ok(self.info.clone())
    }

    async fn load(&self) -> Result<SnapshotLoad> {
        let state = self.state();
        Ok(SnapshotLoad {
            info: Ok(self.info.clone()),
            artifacts: state.artifacts.clone(),
            relationships: state.relationships.clone(),
            issues: Vec::new(),
        })
    }
}

#[async_trait]
impl ArtifactRepository for MemoryStore {
    async fn create(&self, new: NewArtifact) -> Result<Artifact> {
        let mut state = self.state();
        let snapshot = self.snapshot_of(&state)?;
        let artifact =
            rules::plan_create_artifact(&snapshot, self.ids.artifact_id(), new, self.clock.now())?;
        state.artifacts.push(artifact.clone());
        Ok(artifact)
    }

    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>> {
        Ok(self
            .state()
            .artifacts
            .iter()
            .find(|a| a.id() == id)
            .cloned())
    }

    async fn resolve(&self, reference: &ArtifactRef) -> Result<Option<Artifact>> {
        let state = self.state();
        Ok(state
            .artifacts
            .iter()
            .find(|artifact| match reference {
                ArtifactRef::Id(id) => artifact.id() == id,
                ArtifactRef::Slug(slug) => artifact.slug() == slug,
            })
            .cloned())
    }

    async fn list(&self) -> Result<Vec<Artifact>> {
        let mut artifacts = self.state().artifacts.clone();
        artifacts.sort_by_key(|a| *a.id());
        Ok(artifacts)
    }

    async fn update(
        &self,
        id: &ArtifactId,
        patch: ArtifactPatch,
        expected: Option<Revision>,
    ) -> Result<Patched> {
        let mut state = self.state();
        let snapshot = self.snapshot_of(&state)?;
        let patched =
            rules::plan_update_artifact(&snapshot, id, patch, expected, self.clock.now())?;
        if !patched.changed.is_empty()
            && let Some(slot) = state.artifacts.iter_mut().find(|a| a.id() == id)
        {
            *slot = patched.artifact.clone();
        }
        Ok(patched)
    }
}

#[async_trait]
impl RelationshipRepository for MemoryStore {
    async fn create(&self, new: NewRelationship) -> Result<Relationship> {
        let mut state = self.state();
        let snapshot = self.snapshot_of(&state)?;
        let relationship = rules::plan_create_relationship(
            &snapshot,
            self.ids.relationship_id(),
            new,
            self.clock.now(),
        )?;
        state.relationships.push(relationship.clone());
        Ok(relationship)
    }

    async fn get(&self, id: &RelationshipId) -> Result<Option<Relationship>> {
        Ok(self
            .state()
            .relationships
            .iter()
            .find(|r| r.id() == id)
            .cloned())
    }

    async fn list(&self) -> Result<Vec<Relationship>> {
        let mut relationships = self.state().relationships.clone();
        relationships.sort_by_key(|r| *r.id());
        Ok(relationships)
    }
}

#[cfg(test)]
mod tests {
    use functor_testkit::{Env, Fresh, RepoFixture, run_store_conformance};

    use super::*;

    #[tokio::test]
    async fn memory_store_passes_the_conformance_suite() {
        run_store_conformance(|env: &Env| Fresh::new(MemoryStore::new(env.clock(), env.ids())))
            .await;
    }

    #[tokio::test]
    async fn a_store_can_start_from_a_fixture_snapshot() {
        let fixture =
            RepoFixture::new()
                .artifact("a")
                .artifact("b")
                .relation("b", "depends_on", "a");
        let env = Env::new();
        let store = MemoryStore::from_snapshot(&fixture.build(), env.clock(), env.ids());

        let snapshot = store.snapshot().await.unwrap();
        assert_eq!(snapshot.artifacts(), fixture.artifacts());
        assert_eq!(snapshot.relationships(), fixture.relationships());
        assert_eq!(&store.info().await.unwrap(), fixture.info());
    }

    #[tokio::test]
    async fn the_store_is_usable_as_trait_objects() {
        let env = Env::new();
        let store = Arc::new(MemoryStore::new(env.clock(), env.ids()));
        let reader: Arc<dyn RepositoryReader> = store.clone();
        let artifacts: Arc<dyn ArtifactRepository> = store;
        assert_eq!(reader.snapshot().await.unwrap().artifacts(), []);
        assert_eq!(artifacts.list().await.unwrap(), []);
    }
}
