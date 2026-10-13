//! The event log on disk: appending under the repository write lock, sequence numbering and
//! size-based rotation (`docs/event-model-mvp.md`, ADR-0008).

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use functor_core::{
    Clock, EventPosition, EventPublisher, EventSource, EventSubscription, FunctorError,
    IdGenerator, Result,
};
use functor_model::{Event, NewEvent, RepositoryId};
use functor_repository::{DEFAULT_LOCK_TIMEOUT, Layout, WriteLock, run_blocking};

use crate::follow::{FollowOptions, Follower, Update};
use crate::wire::{encode_event, peek_key, peek_seq};

/// A log file is rotated when it reaches this size (ADR-0008).
pub const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;
/// This many archived logs are kept (ADR-0008).
pub const MAX_ARCHIVES: usize = 10;

/// How much of a file's end or start is read when looking for a sequence number.
const SCAN_CHUNK: u64 = 64 * 1024;

/// The rotation limits. Fixed in the MVP; tests use small values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Rotate `current.log` when it is at least this many bytes.
    pub max_bytes: u64,
    /// Keep this many archives.
    pub max_archives: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: MAX_LOG_BYTES,
            max_archives: MAX_ARCHIVES,
        }
    }
}

/// Tuning options of a [`FileEventLog`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct EventLogOptions {
    /// How long a publisher waits for the repository write lock.
    pub lock_timeout: Duration,
    /// The rotation limits.
    pub limits: Limits,
}

impl Default for EventLogOptions {
    fn default() -> Self {
        Self {
            lock_timeout: DEFAULT_LOCK_TIMEOUT,
            limits: Limits::default(),
        }
    }
}

impl EventLogOptions {
    /// Sets the lock timeout.
    #[must_use]
    pub fn with_lock_timeout(mut self, timeout: Duration) -> Self {
        self.lock_timeout = timeout;
        self
    }

    /// Sets the rotation limits.
    #[must_use]
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
}

/// An event that was appended, with the warnings that did not stop it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    /// The stored event, with its assigned id, sequence number and time.
    pub event: Event,
    /// Problems with rotation or pruning; the event was appended regardless.
    pub warnings: Vec<String>,
}

// ---- files ----

/// The name of the archive holding sequence numbers `first` to `last`.
#[must_use]
pub fn archive_name(first: u64, last: u64) -> String {
    format!("events-{first:012}-{last:012}.log")
}

/// An archived log file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Archive {
    pub path: PathBuf,
    pub first: u64,
    pub last: u64,
}

fn parse_archive_name(name: &str) -> Option<(u64, u64)> {
    let numbers = name.strip_prefix("events-")?.strip_suffix(".log")?;
    let (first, last) = numbers.split_once('-')?;
    if first.len() != 12 || last.len() != 12 {
        return None;
    }
    Some((first.parse().ok()?, last.parse().ok()?))
}

/// The archives in `dir`, oldest first. A missing directory has none; other files are ignored.
pub(crate) fn list_archives(dir: &Path) -> io::Result<Vec<Archive>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut archives = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        if let Some((first, last)) = name.to_str().and_then(parse_archive_name) {
            archives.push(Archive {
                path: entry.path(),
                first,
                last,
            });
        }
    }
    archives.sort_by_key(|a| (a.first, a.last));
    Ok(archives)
}

/// What the end of a log file says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tail {
    /// The file length in bytes.
    pub len: u64,
    /// The highest sequence number found on the last lines, if any line has one.
    pub last_seq: Option<u64>,
    /// Whether the file ends with a newline (an empty file does).
    pub ends_with_newline: bool,
}

/// Reads the end of `path`: its length and the sequence number of the last line that has one.
///
/// A trailing line without a newline (a torn write) counts when it is complete JSON, so that
/// numbering never reuses its sequence number. `Ok(None)` means the file does not exist.
pub(crate) fn scan_tail(path: &Path) -> io::Result<Option<Tail>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let len = file.metadata()?.len();
    if len == 0 {
        return Ok(Some(Tail {
            len,
            last_seq: None,
            ends_with_newline: true,
        }));
    }
    let mut window = SCAN_CHUNK.min(len);
    loop {
        let start = len - window;
        file.seek(SeekFrom::Start(start))?;
        let mut buffer = vec![0_u8; usize::try_from(window).unwrap_or(usize::MAX)];
        file.read_exact(&mut buffer)?;
        let ends_with_newline = buffer.last() == Some(&b'\n');
        for (index, line) in buffer
            .split(|byte| *byte == b'\n')
            .enumerate()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            // The first segment of a window that does not start the file may be cut in half.
            if index == 0 && start > 0 {
                break;
            }
            if let Some(seq) = std::str::from_utf8(line).ok().and_then(peek_seq) {
                return Ok(Some(Tail {
                    len,
                    last_seq: Some(seq),
                    ends_with_newline,
                }));
            }
        }
        if start == 0 {
            return Ok(Some(Tail {
                len,
                last_seq: None,
                ends_with_newline,
            }));
        }
        window = (window * 2).min(len);
    }
}

/// The sequence number of the first line of `file` that has one, looking at its first few KiB.
pub(crate) fn first_seq(file: &mut File) -> io::Result<Option<u64>> {
    Ok(first_key(file)?.map(|(seq, _)| seq))
}

/// The sequence number and event id of the first line of `file` that has a sequence number.
pub(crate) fn first_key(file: &mut File) -> io::Result<Option<(u64, String)>> {
    file.seek(SeekFrom::Start(0))?;
    let mut buffer = Vec::new();
    file.take(SCAN_CHUNK * 4).read_to_end(&mut buffer)?;
    Ok(buffer
        .split(|byte| *byte == b'\n')
        .find_map(|line| std::str::from_utf8(line).ok().and_then(peek_key)))
}

// ---- the log ----

struct Inner {
    layout: Layout,
    lock: WriteLock,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    origin: RepositoryId,
    limits: Limits,
}

/// The event log of one repository: a publisher and an event source over
/// `events/current.log` and `events/archive/`.
///
/// Cloning is cheap and shares the same log. Publishers serialize on the repository write lock,
/// so sequence numbers are gap-free across threads and processes.
#[derive(Clone)]
pub struct FileEventLog {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for FileEventLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileEventLog")
            .field("root", &self.inner.layout.root())
            .finish_non_exhaustive()
    }
}

impl FileEventLog {
    /// Opens the log of the repository rooted at `root`, producing events with origin `origin`.
    #[must_use]
    pub fn new(
        root: &Path,
        origin: RepositoryId,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        options: EventLogOptions,
    ) -> Self {
        let layout = Layout::new(root);
        let lock = WriteLock::new(&layout, options.lock_timeout);
        Self {
            inner: Arc::new(Inner {
                layout,
                lock,
                clock,
                ids,
                origin,
                limits: options.limits,
            }),
        }
    }

    /// The repository layout.
    #[must_use]
    pub fn layout(&self) -> &Layout {
        &self.inner.layout
    }

    /// Appends an event, returning it together with any rotation or pruning warnings.
    ///
    /// # Errors
    /// `Locked` when the write lock is not acquired in time, `Storage` when the log cannot be
    /// read or appended to. State is never affected; callers report a failure as a warning.
    pub async fn publish_detailed(&self, new: NewEvent) -> Result<Published> {
        let inner = Arc::clone(&self.inner);
        run_blocking(move || inner.append(new)).await
    }

    /// The blocking form of [`FileEventLog::publish_detailed`], for synchronous callers.
    ///
    /// # Errors
    /// See [`FileEventLog::publish_detailed`].
    pub fn publish_blocking(&self, new: NewEvent) -> Result<Published> {
        self.inner.append(new)
    }

    /// Starts following the log from `position`, with the default timing.
    ///
    /// # Errors
    /// `Storage` when the log cannot be read.
    pub async fn follow(&self, position: EventPosition) -> Result<Follower> {
        Follower::open(self.inner.layout.root(), position, FollowOptions::default()).await
    }
}

impl Inner {
    fn current(&self) -> PathBuf {
        self.layout.events_log()
    }

    fn append(&self, new: NewEvent) -> Result<Published> {
        self.lock.run(|| self.append_locked(new))
    }

    fn append_locked(&self, new: NewEvent) -> Result<Published> {
        let events_dir = self.layout.events_dir();
        fs::create_dir_all(&events_dir)
            .map_err(|error| FunctorError::storage("create", &events_dir, error))?;
        let current = self.current();
        let archive_dir = self.layout.events_archive_dir();
        let storage = |action: &'static str, path: &Path| {
            let path = path.to_path_buf();
            move |error: io::Error| FunctorError::storage(action, &path, error)
        };

        let mut tail = scan_tail(&current).map_err(storage("read", &current))?;
        let archives = list_archives(&archive_dir).map_err(storage("read", &archive_dir))?;
        let last_seq = tail
            .and_then(|t| t.last_seq)
            .into_iter()
            .chain(archives.last().map(|a| a.last))
            .max()
            .unwrap_or(0);

        let mut warnings = Vec::new();
        if tail.is_some_and(|t| t.len >= self.limits.max_bytes) {
            match self.rotate(tail) {
                Ok(()) => {
                    tail = None;
                    warnings.extend(self.prune());
                }
                Err(warning) => warnings.push(warning),
            }
        }

        let event = Event::new(
            self.ids.event_id(),
            last_seq.saturating_add(1),
            self.clock.now(),
            self.origin,
            new,
        )
        .map_err(|error| FunctorError::Internal(error.to_string()))?;

        // A torn last line is terminated first, so the new event starts on its own line.
        let mut line = String::new();
        if tail.is_some_and(|t| t.len > 0 && !t.ends_with_newline) {
            line.push('\n');
        }
        line.push_str(&encode_event(&event));
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&current)
            .map_err(storage("open", &current))?;
        file.write_all(line.as_bytes())
            .and_then(|()| file.flush())
            .map_err(storage("append to", &current))?;
        Ok(Published { event, warnings })
    }

    /// Moves `current.log` to the archive named after its first and last sequence numbers.
    fn rotate(&self, tail: Option<crate::log::Tail>) -> std::result::Result<(), String> {
        let current = self.current();
        let Some(last) = tail.and_then(|t| t.last_seq) else {
            return Err("event log was not rotated: it holds no readable event".to_owned());
        };
        let mut file = File::open(&current)
            .map_err(|error| format!("event log was not rotated: could not read it: {error}"))?;
        let first = first_seq(&mut file)
            .map_err(|error| format!("event log was not rotated: could not read it: {error}"))?
            .unwrap_or(last);
        drop(file);

        let archive_dir = self.layout.events_archive_dir();
        fs::create_dir_all(&archive_dir)
            .map_err(|error| format!("event log was not rotated: {error}"))?;
        let target = archive_dir.join(archive_name(first, last));
        if target.exists() {
            return Err(format!(
                "event log was not rotated: {} already exists",
                self.layout.relative(&target)
            ));
        }
        fs::rename(&current, &target).map_err(|error| format!("event log was not rotated: {error}"))
    }

    /// Deletes the oldest archives beyond the limit; failures are warnings.
    fn prune(&self) -> Vec<String> {
        let archive_dir = self.layout.events_archive_dir();
        let archives = match list_archives(&archive_dir) {
            Ok(archives) => archives,
            Err(error) => return vec![format!("old event archives were not pruned: {error}")],
        };
        let excess = archives.len().saturating_sub(self.limits.max_archives);
        archives
            .into_iter()
            .take(excess)
            .filter_map(|archive| {
                fs::remove_file(&archive.path).err().map(|error| {
                    format!(
                        "old event archive {} was not removed: {error}",
                        self.layout.relative(&archive.path)
                    )
                })
            })
            .collect()
    }
}

#[async_trait]
impl EventPublisher for FileEventLog {
    async fn publish(&self, event: NewEvent) -> Result<Event> {
        self.publish_detailed(event)
            .await
            .map(|published| published.event)
    }
}

/// A subscription that hides the follower's warnings behind the plain trait.
struct Stream(Follower);

#[async_trait]
impl EventSubscription for Stream {
    async fn next(&mut self) -> Result<Option<Event>> {
        loop {
            match self.0.next_within(None).await? {
                Some(Update::Event(event)) => return Ok(Some(event)),
                Some(Update::Warning(_)) => {}
                None => return Ok(None),
            }
        }
    }
}

#[async_trait]
impl EventSource for FileEventLog {
    async fn subscribe(&self, from: EventPosition) -> Result<Box<dyn EventSubscription>> {
        Ok(Box::new(Stream(self.follow(from).await?)))
    }
}
