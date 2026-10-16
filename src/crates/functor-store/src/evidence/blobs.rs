//! Where evidence bytes and per-run ownership live (ADR-0013).
//!
//! The recorder keeps its records as artifacts and relationships in an ordinary store. Everything
//! that is not an entity file goes through [`EvidenceBlobs`]: content-addressed files (captures,
//! streams, outputs, results, reports), reading repository content, per-run leases and the lock
//! that serializes recording one request id.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use functor_core::{FunctorError, Result};
use functor_model::{ArtifactId, ContentDigest, ContentPath, RequestId, bounds};
use functor_repository::{HeldLock, Layout, atomic_write, run_blocking};

/// Proof of ownership; dropping it releases the lock or lease.
pub type Guard = Box<dyn Any + Send + Sync>;

/// Storage for everything evidence needs besides entity files.
#[async_trait]
pub trait EvidenceBlobs: Send + Sync {
    /// Stores `bytes` under `content/evidence/<area>/<hex of digest>` and returns the path.
    ///
    /// Content-addressed, so storing the same bytes again is a no-op, and a damaged file at that
    /// address is repaired with the correct bytes.
    async fn put(&self, area: &str, digest: &ContentDigest, bytes: Vec<u8>) -> Result<ContentPath>;

    /// Reads a repository content file, or `None` when it does not exist. Fails with
    /// `InvalidInput` above the item size limit and `Inconsistent` for a path that leaves the
    /// repository.
    async fn get(&self, path: &ContentPath) -> Result<Option<Vec<u8>>>;

    /// Takes ownership of a run. `Locked` when another owner holds it.
    async fn try_lease(&self, run: &ArtifactId) -> Result<Guard>;

    /// Whether another owner currently holds the run.
    async fn is_leased(&self, run: &ArtifactId) -> Result<bool>;

    /// Serializes recording of one request id, so two recordings of it cannot both create a run.
    async fn lock_request(&self, request: &RequestId) -> Result<Guard>;
}

/// The content path of an evidence file.
fn evidence_path(area: &str, digest: &ContentDigest) -> Result<ContentPath> {
    Ok(ContentPath::new(&format!(
        "content/evidence/{area}/{}",
        digest.hex()
    ))?)
}

// ---------------------------------------------------------------------------------------------
// files
// ---------------------------------------------------------------------------------------------

/// Evidence files and lock files inside a repository directory.
#[derive(Debug, Clone)]
pub struct FileBlobs {
    layout: Layout,
    lock_timeout: Duration,
}

impl FileBlobs {
    /// Evidence storage for the repository with this layout.
    #[must_use]
    pub fn new(layout: Layout, lock_timeout: Duration) -> Self {
        Self {
            layout,
            lock_timeout,
        }
    }

    /// The repository layout.
    #[must_use]
    pub fn layout(&self) -> &Layout {
        &self.layout
    }
}

fn read_limited(file: File, path: &PathBuf) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(bounds::MAX_ITEM_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| FunctorError::storage("read", path, error))?;
    if bytes.len() as u64 > bounds::MAX_ITEM_BYTES {
        return Err(FunctorError::invalid_input(
            "content",
            format!(
                "`{}` is larger than {} bytes",
                path.display(),
                bounds::MAX_ITEM_BYTES
            ),
        ));
    }
    Ok(bytes)
}

#[async_trait]
impl EvidenceBlobs for FileBlobs {
    async fn put(&self, area: &str, digest: &ContentDigest, bytes: Vec<u8>) -> Result<ContentPath> {
        let content_path = evidence_path(area, digest)?;
        let dir = self.layout.evidence_dir(area);
        let file = dir.join(digest.hex());
        run_blocking(move || {
            fs::create_dir_all(&dir)
                .map_err(|error| FunctorError::storage("create directory", &dir, error))?;
            match fs::read(&file) {
                Ok(existing) if existing == bytes => return Ok(()),
                Ok(_) | Err(_) => {}
            }
            atomic_write(&file, &bytes)
        })
        .await?;
        Ok(content_path)
    }

    async fn get(&self, path: &ContentPath) -> Result<Option<Vec<u8>>> {
        let layout = self.layout.clone();
        let path = path.clone();
        run_blocking(move || {
            let full = layout.content_file(&path);
            let file = match File::open(&full) {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(FunctorError::storage("open", &full, error)),
            };
            let metadata = file
                .metadata()
                .map_err(|error| FunctorError::storage("read", &full, error))?;
            if !metadata.is_file() {
                return Ok(None);
            }
            // A link inside the repository must not lead outside it.
            let inside = match (fs::canonicalize(&full), fs::canonicalize(layout.root())) {
                (Ok(real), Ok(root)) => real.starts_with(root),
                _ => false,
            };
            if !inside {
                return Err(FunctorError::Inconsistent {
                    detail: format!("content path `{path}` resolves outside the repository"),
                });
            }
            read_limited(file, &full).map(Some)
        })
        .await
    }

    async fn try_lease(&self, run: &ArtifactId) -> Result<Guard> {
        let path = self.layout.run_lease_file(run);
        run_blocking(move || match HeldLock::try_acquire(&path)? {
            Some(held) => Ok(Box::new(held) as Guard),
            None => Err(FunctorError::Locked { path }),
        })
        .await
    }

    async fn is_leased(&self, run: &ArtifactId) -> Result<bool> {
        let path = self.layout.run_lease_file(run);
        run_blocking(move || HeldLock::is_held(&path)).await
    }

    async fn lock_request(&self, request: &RequestId) -> Result<Guard> {
        let path = self.layout.request_lock_file(request.as_str());
        let timeout = self.lock_timeout;
        run_blocking(move || Ok(Box::new(HeldLock::acquire(&path, timeout)?) as Guard)).await
    }
}

// ---------------------------------------------------------------------------------------------
// memory
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct MemoryState {
    files: BTreeMap<String, Vec<u8>>,
    leases: BTreeSet<String>,
    requests: BTreeSet<String>,
}

/// Evidence files and locks held in memory, for use with `MemoryStore`.
#[derive(Debug, Clone, Default)]
pub struct MemoryBlobs {
    state: Arc<Mutex<MemoryState>>,
}

struct MemoryGuard {
    state: Arc<Mutex<MemoryState>>,
    lease: bool,
    key: String,
}

impl Drop for MemoryGuard {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if self.lease {
            state.leases.remove(&self.key);
        } else {
            state.requests.remove(&self.key);
        }
    }
}

impl MemoryBlobs {
    /// Empty evidence storage.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn state(&self) -> MutexGuard<'_, MemoryState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Puts a file at `path`, as a user would put content next to an artifact.
    pub fn put_content(&self, path: &ContentPath, bytes: &[u8]) {
        self.state()
            .files
            .insert(path.as_str().to_owned(), bytes.to_vec());
    }

    /// Appends a byte to the file at `path`, damaging it. Returns whether it existed.
    #[must_use]
    pub fn damage(&self, path: &ContentPath) -> bool {
        self.state()
            .files
            .get_mut(path.as_str())
            .map(|bytes| bytes.push(b'!'))
            .is_some()
    }

    fn lock(&self, lease: bool, key: String) -> Result<Guard> {
        let mut state = self.state();
        let held = if lease {
            !state.leases.insert(key.clone())
        } else {
            !state.requests.insert(key.clone())
        };
        if held {
            return Err(FunctorError::Locked {
                path: PathBuf::from(key),
            });
        }
        Ok(Box::new(MemoryGuard {
            state: Arc::clone(&self.state),
            lease,
            key,
        }))
    }
}

#[async_trait]
impl EvidenceBlobs for MemoryBlobs {
    async fn put(&self, area: &str, digest: &ContentDigest, bytes: Vec<u8>) -> Result<ContentPath> {
        let path = evidence_path(area, digest)?;
        self.state().files.insert(path.as_str().to_owned(), bytes);
        Ok(path)
    }

    async fn get(&self, path: &ContentPath) -> Result<Option<Vec<u8>>> {
        let found = self.state().files.get(path.as_str()).cloned();
        if let Some(bytes) = &found
            && bytes.len() as u64 > bounds::MAX_ITEM_BYTES
        {
            return Err(FunctorError::invalid_input(
                "content",
                "larger than the item limit",
            ));
        }
        Ok(found)
    }

    async fn try_lease(&self, run: &ArtifactId) -> Result<Guard> {
        self.lock(true, format!("run:{run}"))
    }

    async fn is_leased(&self, run: &ArtifactId) -> Result<bool> {
        Ok(self.state().leases.contains(&format!("run:{run}")))
    }

    async fn lock_request(&self, request: &RequestId) -> Result<Guard> {
        self.lock(false, format!("request:{request}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::codec::sha256;

    fn digest(bytes: &[u8]) -> ContentDigest {
        sha256(bytes)
    }

    async fn blobs_behave<B: EvidenceBlobs>(blobs: &B) {
        let d = digest(b"hello");
        let path = blobs.put("captures", &d, b"hello".to_vec()).await.unwrap();
        assert_eq!(
            path.as_str(),
            format!("content/evidence/captures/{}", d.hex())
        );
        assert_eq!(blobs.get(&path).await.unwrap().unwrap(), b"hello");
        let again = blobs.put("captures", &d, b"hello".to_vec()).await.unwrap();
        assert_eq!(again, path, "storing the same bytes again is a no-op");
        let missing = ContentPath::new("content/evidence/captures/none").unwrap();
        assert_eq!(blobs.get(&missing).await.unwrap(), None);

        let run = ArtifactId::from_bits(1);
        assert!(!blobs.is_leased(&run).await.unwrap());
        let lease = blobs.try_lease(&run).await.unwrap();
        assert!(blobs.is_leased(&run).await.unwrap());
        assert_eq!(
            blobs.try_lease(&run).await.err().map(|e| e.kind()),
            Some(functor_core::ErrorKind::Locked)
        );
        drop(lease);
        assert!(!blobs.is_leased(&run).await.unwrap());
        blobs.try_lease(&run).await.unwrap();

        let request = RequestId::new("r1").unwrap();
        let guard = blobs.lock_request(&request).await.unwrap();
        drop(guard);
        blobs.lock_request(&request).await.unwrap();
    }

    #[tokio::test]
    async fn memory_blobs_behave() {
        blobs_behave(&MemoryBlobs::new()).await;
    }

    #[tokio::test]
    async fn file_blobs_behave_and_repair_damaged_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".functor")).unwrap();
        let blobs = FileBlobs::new(Layout::new(dir.path()), Duration::from_millis(200));
        blobs_behave(&blobs).await;

        let d = digest(b"data");
        let path = blobs.put("streams", &d, b"data".to_vec()).await.unwrap();
        fs::write(blobs.layout().content_file(&path), b"damaged").unwrap();
        blobs.put("streams", &d, b"data".to_vec()).await.unwrap();
        assert_eq!(blobs.get(&path).await.unwrap().unwrap(), b"data");
    }

    #[tokio::test]
    async fn a_request_lock_waits_then_times_out() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".functor")).unwrap();
        let blobs = FileBlobs::new(Layout::new(dir.path()), Duration::from_millis(50));
        let request = RequestId::new("r1").unwrap();
        let _held = blobs.lock_request(&request).await.unwrap();
        let error = blobs.lock_request(&request).await.err().unwrap();
        assert_eq!(error.kind(), functor_core::ErrorKind::Locked);
    }

    #[tokio::test]
    async fn oversized_files_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("content")).unwrap();
        let blobs = FileBlobs::new(Layout::new(dir.path()), Duration::from_millis(50));
        let path = ContentPath::new("content/big").unwrap();
        let file = blobs.layout().content_file(&path);
        let size = usize::try_from(bounds::MAX_ITEM_BYTES).unwrap();
        fs::write(&file, vec![0_u8; size]).unwrap();
        assert!(
            blobs.get(&path).await.unwrap().is_some(),
            "exactly at the limit"
        );
        fs::write(&file, vec![0_u8; size + 1]).unwrap();
        assert_eq!(
            blobs.get(&path).await.err().map(|e| e.kind()),
            Some(functor_core::ErrorKind::InvalidInput)
        );
    }
}
