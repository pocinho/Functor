//! The composition root: where concrete implementations are chosen and wired.
//!
//! See `docs/crate-dependencies.md`. This is the only place that reads the environment or builds
//! services; command handlers receive traits.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use functor_core::{
    Clock, FunctorError, IdGenerator, RepositoryHandle, Result, SystemClock, UlidGenerator,
};
use functor_events::{EventLogOptions, FileEventLog};
use functor_model::RepositoryId;
use functor_repository::{DEFAULT_LOCK_TIMEOUT, FileRepositoryManager, discover_root};
use functor_store::{FileStore, StoreOptions};

use crate::cli::OutputMode;
use crate::events::Recorder;

/// Options that apply to every command.
#[derive(Debug, Clone)]
pub struct Globals {
    /// `--repo` / `FUNCTOR_REPO`.
    pub repo: Option<PathBuf>,
    /// `--output`.
    pub output: OutputMode,
    /// `--quiet`.
    pub quiet: bool,
}

/// The injected time and id sources.
#[derive(Clone)]
pub struct Services {
    /// The clock.
    pub clock: Arc<dyn Clock>,
    /// The id generator.
    pub ids: Arc<dyn IdGenerator>,
}

impl Services {
    /// The production services, or the deterministic ones requested by the test-only
    /// environment variables when built with the `test-support` feature.
    ///
    /// # Errors
    /// Fails if a test-only variable is set to an invalid value.
    pub fn from_environment() -> Result<Self> {
        #[cfg(feature = "test-support")]
        if let Some(services) = crate::test_support::services()? {
            return Ok(services);
        }
        Ok(Self {
            clock: Arc::new(SystemClock),
            ids: Arc::new(UlidGenerator::new()),
        })
    }

    /// The repository manager wired to these services.
    #[must_use]
    pub fn manager(&self) -> FileRepositoryManager {
        FileRepositoryManager::new(Arc::clone(&self.clock), Arc::clone(&self.ids))
    }
}

/// An opened repository and the store over it.
pub struct Context {
    /// Where the repository is and what it is.
    pub handle: RepositoryHandle,
    /// The entity store.
    pub store: FileStore,
    /// Appends the events of mutating commands.
    pub recorder: Recorder,
}

fn lock_timeout() -> Result<Duration> {
    match std::env::var("FUNCTOR_LOCK_TIMEOUT") {
        Err(_) => Ok(DEFAULT_LOCK_TIMEOUT),
        Ok(text) => text
            .trim()
            .parse::<u64>()
            .map(Duration::from_secs)
            .map_err(|_| {
                FunctorError::invalid_input(
                    "FUNCTOR_LOCK_TIMEOUT",
                    format!("`{text}` is not a whole number of seconds"),
                )
            }),
    }
}

/// The event log of the repository rooted at `root`, producing events with origin `origin`.
///
/// # Errors
/// `InvalidInput` for a bad `FUNCTOR_LOCK_TIMEOUT`.
pub fn event_recorder(root: &Path, origin: RepositoryId, services: &Services) -> Result<Recorder> {
    let options = EventLogOptions::default().with_lock_timeout(lock_timeout()?);
    Ok(Recorder::new(FileEventLog::new(
        root,
        origin,
        Arc::clone(&services.clock),
        Arc::clone(&services.ids),
        options,
    )))
}

/// Finds the repository root (walking up from `--repo` or the current directory) and opens its
/// store **without** reading `repository.toml`, so that `validate` can report a damaged
/// repository instead of failing before it starts.
///
/// # Errors
/// `NotARepository` when no ancestor contains `.functor/repository.toml`, or `InvalidInput` for
/// a bad `FUNCTOR_LOCK_TIMEOUT`.
pub fn open_unchecked(globals: &Globals, services: &Services) -> Result<FileStore> {
    let start = globals.repo.clone().unwrap_or_else(|| PathBuf::from("."));
    let root = discover_root(&start)?;
    let options = StoreOptions::default().with_lock_timeout(lock_timeout()?);
    Ok(FileStore::at_root(
        &root,
        Arc::clone(&services.clock),
        Arc::clone(&services.ids),
        options,
    ))
}

/// Finds the repository (walking up from `--repo` or the current directory) and opens its store.
///
/// # Errors
/// `NotARepository`, `UnsupportedFormat`, `Inconsistent` or `InvalidInput` for a bad
/// `FUNCTOR_LOCK_TIMEOUT`.
pub async fn open(globals: &Globals, services: &Services) -> Result<Context> {
    use functor_core::RepositoryManager;

    let start = globals.repo.clone().unwrap_or_else(|| PathBuf::from("."));
    let handle = services.manager().discover(&start).await?;
    let options = StoreOptions::default().with_lock_timeout(lock_timeout()?);
    let store = FileStore::with_options(
        &handle,
        Arc::clone(&services.clock),
        Arc::clone(&services.ids),
        options,
    );
    let recorder = event_recorder(&handle.root, *handle.info.id(), services)?;
    Ok(Context {
        handle,
        store,
        recorder,
    })
}
