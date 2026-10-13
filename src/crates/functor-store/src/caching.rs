//! A store that reads the repository once and serves later reads from memory.

use std::sync::{Arc, Mutex, PoisonError};

use async_trait::async_trait;
use functor_core::{
    ArtifactRepository, RelationshipRepository, RepositoryReader, Result, SnapshotLoad,
};
use functor_model::{
    Artifact, ArtifactId, ArtifactPatch, ArtifactRef, NewArtifact, NewRelationship, Patched,
    Relationship, RelationshipId, RepositoryInfo, RepositorySnapshot, Revision,
};

/// Wraps a store so that its strict reads share one snapshot until the wrapper writes.
///
/// A file-backed store loads the whole repository for every read. A command often reads several
/// times (resolve a source, resolve a target, list for suggestions), so a command-line process
/// wraps its store in a `CachingStore`: the first read loads, the rest are served from memory.
///
/// - Reads (`snapshot`, `get`, `resolve`, `list`) use the cached snapshot. A failed load is not
///   cached.
/// - Writes (`create`, `update`) go to the inner store, which checks its rules against fresh
///   state under its own lock, and then drop the cache.
/// - `load` (the lenient read validation uses) and `info` always go to the inner store.
///
/// Changes made by anyone else after the first read are not seen; the cache lives for one
/// command, not for a long-running process.
#[derive(Debug)]
pub struct CachingStore<S> {
    inner: S,
    cached: Mutex<Option<Arc<RepositorySnapshot>>>,
}

impl<S> CachingStore<S> {
    /// Wraps `inner` with an empty cache.
    #[must_use]
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            cached: Mutex::new(None),
        }
    }

    /// The wrapped store.
    #[must_use]
    pub fn inner(&self) -> &S {
        &self.inner
    }

    fn cached(&self) -> Option<Arc<RepositorySnapshot>> {
        self.cached
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn remember(&self, snapshot: Arc<RepositorySnapshot>) {
        *self.cached.lock().unwrap_or_else(PoisonError::into_inner) = Some(snapshot);
    }

    fn forget(&self) {
        *self.cached.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

impl<S: RepositoryReader> CachingStore<S> {
    async fn shared(&self) -> Result<Arc<RepositorySnapshot>> {
        if let Some(snapshot) = self.cached() {
            return Ok(snapshot);
        }
        let snapshot = Arc::new(self.inner.snapshot().await?);
        self.remember(Arc::clone(&snapshot));
        Ok(snapshot)
    }
}

#[async_trait]
impl<S: RepositoryReader> RepositoryReader for CachingStore<S> {
    async fn info(&self) -> Result<RepositoryInfo> {
        self.inner.info().await
    }

    async fn load(&self) -> Result<SnapshotLoad> {
        self.inner.load().await
    }

    async fn snapshot(&self) -> Result<RepositorySnapshot> {
        Ok(RepositorySnapshot::clone(&*self.shared().await?))
    }
}

#[async_trait]
impl<S> ArtifactRepository for CachingStore<S>
where
    S: RepositoryReader + ArtifactRepository,
{
    async fn create(&self, new: NewArtifact) -> Result<Artifact> {
        let created = ArtifactRepository::create(&self.inner, new).await;
        self.forget();
        created
    }

    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>> {
        Ok(self.shared().await?.artifact(id).cloned())
    }

    async fn resolve(&self, reference: &ArtifactRef) -> Result<Option<Artifact>> {
        Ok(self.shared().await?.resolve(reference).cloned())
    }

    async fn list(&self) -> Result<Vec<Artifact>> {
        Ok(self.shared().await?.artifacts().to_vec())
    }

    async fn update(
        &self,
        id: &ArtifactId,
        patch: ArtifactPatch,
        expected: Option<Revision>,
    ) -> Result<Patched> {
        let patched = self.inner.update(id, patch, expected).await;
        self.forget();
        patched
    }
}

#[async_trait]
impl<S> RelationshipRepository for CachingStore<S>
where
    S: RepositoryReader + RelationshipRepository,
{
    async fn create(&self, new: NewRelationship) -> Result<Relationship> {
        let created = RelationshipRepository::create(&self.inner, new).await;
        self.forget();
        created
    }

    async fn get(&self, id: &RelationshipId) -> Result<Option<Relationship>> {
        Ok(self
            .shared()
            .await?
            .relationships()
            .iter()
            .find(|relationship| relationship.id() == id)
            .cloned())
    }

    async fn list(&self) -> Result<Vec<Relationship>> {
        Ok(self.shared().await?.relationships().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use functor_model::{ArtifactType, RelationshipType, Slug, Title};
    use functor_testkit::Env;

    use super::*;
    use crate::MemoryStore;

    /// Everything a store must do holds through the cache too: reads see the wrapper's own writes.
    #[tokio::test]
    async fn the_caching_store_passes_the_conformance_suite() {
        functor_testkit::run_store_conformance(|env: &Env| {
            functor_testkit::Fresh::new(CachingStore::new(MemoryStore::new(env.clock(), env.ids())))
        })
        .await;
    }

    /// Counts how often the inner store is asked for a snapshot.
    struct Counting {
        inner: MemoryStore,
        snapshots: AtomicUsize,
    }

    #[async_trait]
    impl RepositoryReader for Counting {
        async fn info(&self) -> Result<RepositoryInfo> {
            self.inner.info().await
        }

        async fn load(&self) -> Result<SnapshotLoad> {
            self.inner.load().await
        }

        async fn snapshot(&self) -> Result<RepositorySnapshot> {
            self.snapshots.fetch_add(1, Ordering::SeqCst);
            self.inner.snapshot().await
        }
    }

    #[async_trait]
    impl ArtifactRepository for Counting {
        async fn create(&self, new: NewArtifact) -> Result<Artifact> {
            ArtifactRepository::create(&self.inner, new).await
        }
        async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>> {
            ArtifactRepository::get(&self.inner, id).await
        }
        async fn resolve(&self, reference: &ArtifactRef) -> Result<Option<Artifact>> {
            self.inner.resolve(reference).await
        }
        async fn list(&self) -> Result<Vec<Artifact>> {
            ArtifactRepository::list(&self.inner).await
        }
        async fn update(
            &self,
            id: &ArtifactId,
            patch: ArtifactPatch,
            expected: Option<Revision>,
        ) -> Result<Patched> {
            self.inner.update(id, patch, expected).await
        }
    }

    #[async_trait]
    impl RelationshipRepository for Counting {
        async fn create(&self, new: NewRelationship) -> Result<Relationship> {
            RelationshipRepository::create(&self.inner, new).await
        }
        async fn get(&self, id: &RelationshipId) -> Result<Option<Relationship>> {
            RelationshipRepository::get(&self.inner, id).await
        }
        async fn list(&self) -> Result<Vec<Relationship>> {
            RelationshipRepository::list(&self.inner).await
        }
    }

    fn counting() -> CachingStore<Counting> {
        let env = Env::new();
        CachingStore::new(Counting {
            inner: MemoryStore::new(env.clock(), env.ids()),
            snapshots: AtomicUsize::new(0),
        })
    }

    fn new_artifact(slug: &str) -> NewArtifact {
        NewArtifact::new(
            Slug::parse(slug).unwrap(),
            ArtifactType::document(),
            Title::new(slug).unwrap(),
        )
    }

    fn loads(store: &CachingStore<Counting>) -> usize {
        store.inner().snapshots.load(Ordering::SeqCst)
    }

    #[tokio::test]
    async fn many_reads_load_once() {
        let store = counting();
        let a = ArtifactRepository::create(&store, new_artifact("a"))
            .await
            .unwrap();
        let b = ArtifactRepository::create(&store, new_artifact("b"))
            .await
            .unwrap();
        RelationshipRepository::create(
            &store,
            NewRelationship::new(RelationshipType::References, *a.id(), *b.id()),
        )
        .await
        .unwrap();
        assert_eq!(loads(&store), 0, "writes do not read through the cache");

        let reference = ArtifactRef::parse("a").unwrap();
        assert_eq!(store.resolve(&reference).await.unwrap().as_ref(), Some(&a));
        assert_eq!(
            ArtifactRepository::get(&store, b.id())
                .await
                .unwrap()
                .as_ref(),
            Some(&b)
        );
        assert_eq!(ArtifactRepository::list(&store).await.unwrap().len(), 2);
        assert_eq!(RelationshipRepository::list(&store).await.unwrap().len(), 1);
        assert_eq!(store.snapshot().await.unwrap().artifacts().len(), 2);
        assert_eq!(loads(&store), 1);
    }

    #[tokio::test]
    async fn a_write_drops_the_cache_so_later_reads_see_it() {
        let store = counting();
        ArtifactRepository::create(&store, new_artifact("a"))
            .await
            .unwrap();
        assert_eq!(ArtifactRepository::list(&store).await.unwrap().len(), 1);
        assert_eq!(loads(&store), 1);

        let b = ArtifactRepository::create(&store, new_artifact("b"))
            .await
            .unwrap();
        assert_eq!(ArtifactRepository::list(&store).await.unwrap().len(), 2);
        assert_eq!(loads(&store), 2, "the write invalidated the snapshot");

        store
            .update(
                b.id(),
                ArtifactPatch::new().with_title(Title::new("B").unwrap()),
                None,
            )
            .await
            .unwrap();
        let again = ArtifactRepository::get(&store, b.id())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(again.title().as_str(), "B");
        assert_eq!(loads(&store), 3);
    }

    #[tokio::test]
    async fn a_failed_write_also_drops_the_cache() {
        let store = counting();
        ArtifactRepository::create(&store, new_artifact("a"))
            .await
            .unwrap();
        let _ = ArtifactRepository::list(&store).await.unwrap();
        // A duplicate slug fails; the next read must not rely on a possibly stale snapshot.
        assert!(
            ArtifactRepository::create(&store, new_artifact("a"))
                .await
                .is_err()
        );
        let _ = ArtifactRepository::list(&store).await.unwrap();
        assert_eq!(loads(&store), 2);
    }

    #[tokio::test]
    async fn the_lenient_load_and_info_are_never_cached() {
        let store = counting();
        ArtifactRepository::create(&store, new_artifact("a"))
            .await
            .unwrap();
        assert_eq!(store.load().await.unwrap().issues.len(), 0);
        store.info().await.unwrap();
        assert_eq!(loads(&store), 0);
    }
}
