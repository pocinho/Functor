//! Functor entity persistence: file-backed and in-memory stores.
//!
//! See `docs/design/storage-design.md`.
//!
//! - [`FileStore`] keeps artifacts and relationships as TOML files in a repository directory,
//!   with atomic writes under the repository write lock and in-place edits that preserve what
//!   people wrote in the files.
//! - [`MemoryStore`] holds the same state in memory for tests and examples.
//!
//! Both apply the same rules (`rules`) and pass the same conformance suite from
//! `functor-testkit`. Repository lifecycle (init, open, discover) lives in `functor-repository`.

mod artifact_file;
mod caching;
mod file_store;
mod loader;
mod memory;
mod relationship_file;
mod rules;

pub use artifact_file::{
    ARTIFACT_KEYS, MAX_INLINE_CONTENT, ParsedArtifact, edit_artifact_file, parse_artifact_file,
    render_artifact_file,
};
pub use caching::CachingStore;
pub use file_store::{FileStore, StoreOptions};
pub use memory::MemoryStore;
pub use relationship_file::{
    ParsedRelationship, RELATIONSHIP_KEYS, parse_relationship_file, render_relationship_file,
};
