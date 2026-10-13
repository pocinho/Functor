//! Atomic file writes and the repository write lock.
//!
//! See `docs/storage-design.md`, "Write Protocol" and "Locking".

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use functor_core::{FunctorError, Result};

use crate::layout::{Layout, TEMP_MARKER};

/// How long a writer waits for the lock before giving up, by default.
pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(5);

const RENAME_ATTEMPTS: u32 = 5;
const RENAME_BACKOFF: Duration = Duration::from_millis(20);
const LOCK_POLL: Duration = Duration::from_millis(10);

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_path_for(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .map_or_else(|| "file".into(), |n| n.to_string_lossy().into_owned());
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    target.with_file_name(format!(
        "{name}{TEMP_MARKER}{}-{unique}",
        std::process::id()
    ))
}

/// Writes `bytes` to `target` atomically: a temporary file in the same directory is written and
/// synced, then renamed over the target. Readers see the old or the new file, never a partial one.
///
/// A leftover temporary file (after a crash) carries the `.tmp-` marker and is ignored by loaders.
///
/// # Errors
/// Returns `Storage` when any step fails; the temporary file is removed on a best-effort basis.
pub fn atomic_write(target: &Path, bytes: &[u8]) -> Result<()> {
    atomic_write_with(target, bytes, || Ok(()))
}

/// [`atomic_write`] with a hook that runs after the temporary file is durable and before the
/// rename. Tests use it to simulate a crash at that point.
pub(crate) fn atomic_write_with(
    target: &Path,
    bytes: &[u8],
    before_rename: impl FnOnce() -> io::Result<()>,
) -> Result<()> {
    let temp = temp_path_for(target);
    let result = write_and_rename(target, &temp, bytes, before_rename);
    if let Err(error) = result {
        let _ = fs::remove_file(&temp);
        return Err(FunctorError::storage("write", target, error));
    }
    Ok(())
}

fn write_and_rename(
    target: &Path,
    temp: &Path,
    bytes: &[u8],
    before_rename: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    let mut file = File::create_new(temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    before_rename()?;
    rename_with_retry(temp, target)?;
    sync_parent_directory(target);
    Ok(())
}

// Another process (an indexer, antivirus) may briefly hold the target open on Windows.
fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    let mut attempt = 1;
    loop {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(error) if attempt >= RENAME_ATTEMPTS => return Err(error),
            Err(_) => {
                attempt += 1;
                thread::sleep(RENAME_BACKOFF);
            }
        }
    }
}

// Best effort: narrows the power-loss window where the platform allows opening a directory.
fn sync_parent_directory(target: &Path) {
    #[cfg(unix)]
    if let Some(parent) = target.parent()
        && let Ok(dir) = File::open(parent)
    {
        let _ = dir.sync_all();
    }
    #[cfg(not(unix))]
    let _ = target;
}

/// Runs blocking filesystem work on tokio's blocking pool, so it never stalls the async executor
/// (ADR-0010 and `docs/api-design-guidelines.md`).
///
/// # Errors
/// Returns whatever `work` returns, or `Internal` if the blocking task panicked.
pub async fn run_blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| FunctorError::Internal(format!("blocking task failed: {error}")))?
}

/// The advisory exclusive write lock of one repository (`.functor/lock`).
///
/// Writers serialize on it; readers never lock. The lock is released when the closure passed to
/// [`WriteLock::run`] returns, or if the process dies.
#[derive(Debug, Clone)]
pub struct WriteLock {
    path: PathBuf,
    timeout: Duration,
}

impl WriteLock {
    /// Creates the lock for a repository, waiting at most `timeout` to acquire it.
    #[must_use]
    pub fn new(layout: &Layout, timeout: Duration) -> Self {
        Self {
            path: layout.lock_file(),
            timeout,
        }
    }

    /// The lock file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Runs `f` while holding the lock.
    ///
    /// # Errors
    /// Returns `Locked` if the lock is not acquired within the timeout, `Storage` if the lock
    /// file cannot be opened, or whatever `f` returns.
    pub fn run<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|error| FunctorError::storage("open lock", &self.path, error))?;

        let deadline = Instant::now() + self.timeout;
        loop {
            match file.try_lock() {
                Ok(()) => break,
                Err(TryLockError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(FunctorError::Locked {
                            path: self.path.clone(),
                        });
                    }
                    thread::sleep(LOCK_POLL);
                }
                Err(TryLockError::Error(error)) => {
                    return Err(FunctorError::storage("lock", &self.path, error));
                }
            }
        }

        let result = f();
        let _ = file.unlock();
        result
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn atomic_write_creates_and_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.toml");
        atomic_write(&target, b"one").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"one");
        atomic_write(&target, b"two").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"two");
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, ["a.toml"], "no temporary files remain");
    }

    #[test]
    fn failure_before_rename_leaves_the_old_content_and_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a.toml");
        atomic_write(&target, b"old").unwrap();

        let result = atomic_write_with(&target, b"new", || Err(io::Error::other("crash")));
        assert!(result.is_err());
        assert_eq!(fs::read(&target).unwrap(), b"old");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failure_before_rename_does_not_create_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("new.toml");
        assert!(atomic_write_with(&target, b"x", || Err(io::Error::other("crash"))).is_err());
        assert!(!target.exists());
    }

    #[test]
    fn writing_into_a_missing_directory_is_a_storage_error() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("missing").join("a.toml");
        let error = atomic_write(&target, b"x").unwrap_err();
        assert_eq!(error.kind(), functor_core::ErrorKind::Storage);
    }

    fn lock_in(dir: &Path, timeout: Duration) -> WriteLock {
        fs::create_dir_all(dir.join(crate::layout::FUNCTOR_DIR)).unwrap();
        WriteLock::new(&Layout::new(dir), timeout)
    }

    #[test]
    fn lock_runs_the_closure_and_returns_its_value() {
        let dir = tempfile::tempdir().unwrap();
        let lock = lock_in(dir.path(), Duration::from_secs(1));
        assert_eq!(lock.run(|| Ok(7)).unwrap(), 7);
        assert_eq!(
            lock.run(|| Ok(8)).unwrap(),
            8,
            "the lock is released between runs"
        );
    }

    #[test]
    fn closure_errors_pass_through_and_release_the_lock() {
        let dir = tempfile::tempdir().unwrap();
        let lock = lock_in(dir.path(), Duration::from_secs(1));
        let error = lock
            .run::<()>(|| Err(FunctorError::Internal("boom".into())))
            .unwrap_err();
        assert_eq!(error.kind(), functor_core::ErrorKind::Internal);
        assert!(lock.run(|| Ok(())).is_ok());
    }

    #[test]
    fn contention_times_out_with_locked() {
        let dir = tempfile::tempdir().unwrap();
        let holder = lock_in(dir.path(), Duration::from_secs(5));
        let waiter = lock_in(dir.path(), Duration::from_millis(100));

        let (acquired_tx, acquired_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let thread = thread::spawn(move || {
            holder
                .run(|| {
                    acquired_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap();
        });
        acquired_rx.recv().unwrap();

        let error = waiter.run(|| Ok(())).unwrap_err();
        assert_eq!(error.kind(), functor_core::ErrorKind::Locked);

        release_tx.send(()).unwrap();
        thread.join().unwrap();
        assert!(
            waiter.run(|| Ok(())).is_ok(),
            "the lock is free after the holder finishes"
        );
    }

    #[test]
    fn a_waiting_writer_proceeds_when_the_lock_is_released() {
        let dir = tempfile::tempdir().unwrap();
        let holder = lock_in(dir.path(), Duration::from_secs(5));
        let waiter = lock_in(dir.path(), Duration::from_secs(5));

        let (acquired_tx, acquired_rx) = mpsc::channel();
        let thread = thread::spawn(move || {
            holder
                .run(|| {
                    acquired_tx.send(()).unwrap();
                    thread::sleep(Duration::from_millis(150));
                    Ok(())
                })
                .unwrap();
        });
        acquired_rx.recv().unwrap();
        assert!(waiter.run(|| Ok(())).is_ok());
        thread.join().unwrap();
    }

    #[test]
    fn missing_lock_directory_is_a_storage_error() {
        let dir = tempfile::tempdir().unwrap();
        let lock = WriteLock::new(&Layout::new(dir.path()), Duration::from_millis(10));
        let error = lock.run(|| Ok(())).unwrap_err();
        assert_eq!(error.kind(), functor_core::ErrorKind::Storage);
    }
}
