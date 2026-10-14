//! Durable execution and validation evidence (ADR-0013, ADR-0015).
//!
//! [`EvidenceStore`] implements `EvidenceRecorder` and `ReportRecorder` over any store that holds
//! artifacts and relationships, so the same logic runs against files and memory:
//!
//! - [`FileEvidence`]: `FileStore` entities, content-addressed files under `content/evidence/`
//!   and per-run lock files under `.functor/runs/`.
//! - [`InMemoryEvidence`]: `MemoryStore` entities with bytes and locks in memory.

mod blobs;
pub(crate) mod codec;
mod recorder;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use functor_core::{Clock, IdGenerator};

use crate::{FileStore, MemoryStore};

pub use blobs::{EvidenceBlobs, FileBlobs, Guard, MemoryBlobs};
pub use recorder::{EvidenceBacking, EvidenceStore};

/// Evidence stored in a repository directory.
pub type FileEvidence = EvidenceStore<FileStore, FileBlobs>;

/// Evidence held in memory, for tests and examples.
pub type InMemoryEvidence = EvidenceStore<MemoryStore, MemoryBlobs>;

impl FileEvidence {
    /// Evidence for the repository behind `store`, waiting at most the store's default lock
    /// timeout when serializing one request id.
    #[must_use]
    pub fn open(store: FileStore, clock: Arc<dyn Clock>, ids: Arc<dyn IdGenerator>) -> Self {
        let blobs = FileBlobs::new(
            store.layout().clone(),
            functor_repository::DEFAULT_LOCK_TIMEOUT,
        );
        Self::new(store, blobs, clock, ids)
    }
}

impl InMemoryEvidence {
    /// Empty in-memory evidence over a new `MemoryStore`.
    #[must_use]
    pub fn in_memory(clock: Arc<dyn Clock>, ids: Arc<dyn IdGenerator>) -> Self {
        Self::new(
            MemoryStore::new(Arc::clone(&clock), Arc::clone(&ids)),
            MemoryBlobs::new(),
            clock,
            ids,
        )
    }
}
