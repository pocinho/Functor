//! The file-backed store.
//!
//! See `docs/storage-design.md`. Every mutation follows the write protocol: take the repository
//! write lock, re-read what the operation depends on, check the rules, then write one entity
//! file atomically.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use functor_core::{
    ArtifactRepository, Clock, FunctorError, IdGenerator, RelationshipRepository, RepositoryHandle,
    RepositoryReader, Result, SnapshotLoad,
};
use functor_model::{
    Artifact, ArtifactId, ArtifactPatch, ArtifactRef, EntityKind, NewArtifact, NewRelationship,
    Patched, Relationship, RelationshipId, RepositoryInfo, RepositorySnapshot, Revision,
};
use functor_repository::{
    DEFAULT_LOCK_TIMEOUT, Layout, WriteLock, atomic_write, read_repository_file, run_blocking,
};

use crate::artifact_file::{edit_artifact_file, render_artifact_file};
use crate::loader::load_repository;
use crate::relationship_file::render_relationship_file;
use crate::rules;

/// Tuning options of a [`FileStore`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct StoreOptions {
    /// How long a writer waits for the repository write lock.
    pub lock_timeout: Duration,
}

impl Default for StoreOptions {
    fn default() -> Self {
        Self {
            lock_timeout: DEFAULT_LOCK_TIMEOUT,
        }
    }
}

impl StoreOptions {
    /// Sets the lock timeout.
    #[must_use]
    pub fn with_lock_timeout(mut self, timeout: Duration) -> Self {
        self.lock_timeout = timeout;
        self
    }
}

struct Inner {
    layout: Layout,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    lock: WriteLock,
}

/// Artifacts and relationships stored as TOML files in a repository directory.
///
/// Cloning is cheap and shares the same repository. The store is safe to use from several
/// threads and processes: writers serialize on the repository lock, readers see complete files.
#[derive(Clone)]
pub struct FileStore {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for FileStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileStore")
            .field("root", &self.inner.layout.root())
            .finish_non_exhaustive()
    }
}

impl FileStore {
    /// Opens the store of an already opened repository with default options.
    #[must_use]
    pub fn new(
        handle: &RepositoryHandle,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self::with_options(handle, clock, ids, StoreOptions::default())
    }

    /// Opens the store of an already opened repository.
    #[must_use]
    pub fn with_options(
        handle: &RepositoryHandle,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        options: StoreOptions,
    ) -> Self {
        Self::at_root(&handle.root, clock, ids, options)
    }

    /// Opens the store of the repository rooted at `root` without reading `repository.toml`.
    ///
    /// Reads then report what is wrong with the repository (see `RepositoryReader::load`)
    /// instead of failing at open time; `validate` relies on this.
    #[must_use]
    pub fn at_root(
        root: &Path,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        options: StoreOptions,
    ) -> Self {
        let layout = Layout::new(root);
        let lock = WriteLock::new(&layout, options.lock_timeout);
        Self {
            inner: Arc::new(Inner {
                layout,
                clock,
                ids,
                lock,
            }),
        }
    }

    /// The repository layout.
    #[must_use]
    pub fn layout(&self) -> &Layout {
        &self.inner.layout
    }
}

impl Inner {
    fn info(&self) -> Result<RepositoryInfo> {
        Ok(read_repository_file(&self.layout)?.info)
    }

    fn load(&self) -> Result<SnapshotLoad> {
        load_repository(&self.layout)
    }

    fn snapshot(&self) -> Result<RepositorySnapshot> {
        self.load()?.into_snapshot()
    }

    fn write_new(&self, path: &Path, text: &str, entity: EntityKind) -> Result<()> {
        if path.exists() {
            return Err(FunctorError::conflict(
                entity,
                format!("{} already exists", self.layout.relative(path)),
            ));
        }
        atomic_write(path, text.as_bytes())
    }

    fn create_artifact(&self, new: NewArtifact) -> Result<Artifact> {
        self.lock.run(|| {
            let snapshot = self.snapshot()?;
            let artifact = rules::plan_create_artifact(
                &snapshot,
                self.ids.artifact_id(),
                new,
                self.clock.now(),
            )?;
            let path = self.layout.artifact_file(artifact.id());
            self.write_new(
                &path,
                &render_artifact_file(&artifact),
                EntityKind::Artifact,
            )?;
            Ok(artifact)
        })
    }

    fn update_artifact(
        &self,
        id: &ArtifactId,
        patch: ArtifactPatch,
        expected: Option<Revision>,
    ) -> Result<Patched> {
        self.lock.run(|| {
            let snapshot = self.snapshot()?;
            let patched =
                rules::plan_update_artifact(&snapshot, id, patch, expected, self.clock.now())?;
            if patched.changed.is_empty() {
                return Ok(patched);
            }
            let file = self.layout.artifact_file(id);
            let original = fs::read_to_string(&file)
                .map_err(|error| FunctorError::storage("read", &file, error))?;
            let edited = edit_artifact_file(&original, &patched.artifact, &patched.changed)
                .map_err(|problem| FunctorError::Inconsistent {
                    detail: format!("{}: {}", self.layout.relative(&file), problem.detail),
                })?;
            atomic_write(&file, edited.as_bytes())?;
            Ok(patched)
        })
    }

    fn create_relationship(&self, new: NewRelationship) -> Result<Relationship> {
        self.lock.run(|| {
            let snapshot = self.snapshot()?;
            let relationship = rules::plan_create_relationship(
                &snapshot,
                self.ids.relationship_id(),
                new,
                self.clock.now(),
            )?;
            let path = self.layout.relationship_file(relationship.id());
            self.write_new(
                &path,
                &render_relationship_file(&relationship),
                EntityKind::Relationship,
            )?;
            Ok(relationship)
        })
    }
}

#[async_trait]
impl RepositoryReader for FileStore {
    async fn info(&self) -> Result<RepositoryInfo> {
        let inner = Arc::clone(&self.inner);
        run_blocking(move || inner.info()).await
    }

    async fn load(&self) -> Result<SnapshotLoad> {
        let inner = Arc::clone(&self.inner);
        run_blocking(move || inner.load()).await
    }

    async fn snapshot(&self) -> Result<RepositorySnapshot> {
        let inner = Arc::clone(&self.inner);
        run_blocking(move || inner.snapshot()).await
    }
}

#[async_trait]
impl ArtifactRepository for FileStore {
    async fn create(&self, new: NewArtifact) -> Result<Artifact> {
        let inner = Arc::clone(&self.inner);
        run_blocking(move || inner.create_artifact(new)).await
    }

    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>> {
        let inner = Arc::clone(&self.inner);
        let id = *id;
        run_blocking(move || Ok(inner.snapshot()?.artifact(&id).cloned())).await
    }

    async fn resolve(&self, reference: &ArtifactRef) -> Result<Option<Artifact>> {
        let inner = Arc::clone(&self.inner);
        let reference = reference.clone();
        run_blocking(move || Ok(inner.snapshot()?.resolve(&reference).cloned())).await
    }

    async fn list(&self) -> Result<Vec<Artifact>> {
        let inner = Arc::clone(&self.inner);
        run_blocking(move || Ok(inner.snapshot()?.artifacts().to_vec())).await
    }

    async fn update(
        &self,
        id: &ArtifactId,
        patch: ArtifactPatch,
        expected: Option<Revision>,
    ) -> Result<Patched> {
        let inner = Arc::clone(&self.inner);
        let id = *id;
        run_blocking(move || inner.update_artifact(&id, patch, expected)).await
    }
}

#[async_trait]
impl RelationshipRepository for FileStore {
    async fn create(&self, new: NewRelationship) -> Result<Relationship> {
        let inner = Arc::clone(&self.inner);
        run_blocking(move || inner.create_relationship(new)).await
    }

    async fn get(&self, id: &RelationshipId) -> Result<Option<Relationship>> {
        let inner = Arc::clone(&self.inner);
        let id = *id;
        run_blocking(move || {
            Ok(inner
                .snapshot()?
                .relationships()
                .iter()
                .find(|relationship| relationship.id() == &id)
                .cloned())
        })
        .await
    }

    async fn list(&self) -> Result<Vec<Relationship>> {
        let inner = Arc::clone(&self.inner);
        run_blocking(move || Ok(inner.snapshot()?.relationships().to_vec())).await
    }
}
