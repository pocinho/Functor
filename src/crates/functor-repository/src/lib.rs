//! Functor repository lifecycle: layout, repository metadata, init/open/discover and the write lock.
//!
//! See `docs/design/repository-layout.md`, `docs/design/repository-lifecycle.md` and ADR-0009. This is the one
//! foundation service: `functor-store` and `functor-events` build on its layout and write lock.

mod fsutil;
mod layout;
mod manager;
mod repository_file;
pub mod toml_support;

pub use fsutil::{DEFAULT_LOCK_TIMEOUT, WriteLock, atomic_write, run_blocking};
pub use layout::{FUNCTOR_DIR, GITKEEP, Layout, TEMP_MARKER};
pub use manager::{
    FileRepositoryManager, discover_repository, discover_root, open_repository,
    read_repository_file,
};
pub use repository_file::{
    KNOWN_KEYS as REPOSITORY_FILE_KEYS, ParsedRepositoryFile, RepositoryFileError,
    parse_repository_file, render_repository_file,
};
