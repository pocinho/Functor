//! Following the event log: delivery by sequence number across rotation and truncation
//! (`docs/event-model-mvp.md`, "Watching").

use std::collections::{BTreeSet, VecDeque};
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use functor_core::{EventPosition, FunctorError, Result};
use functor_model::Event;
use functor_repository::{Layout, run_blocking};
use tokio::sync::Notify;

use crate::log::{first_key, first_seq, list_archives, scan_tail};
use crate::wire::{LineProblem, decode_event, peek_seq};

const SCAN_CHUNK: u64 = 64 * 1024;

/// Something a follower produced: an event, or a problem that did not stop it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    /// The next event, in sequence order.
    Event(Event),
    /// A warning, for example that the requested events were pruned or a line was unreadable.
    Warning(String),
}

/// How a follower waits for new events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FollowOptions {
    /// Use filesystem notifications when they are available.
    pub notifications: bool,
    /// How often to poll when notifications are not in use (500 ms).
    pub poll_interval: Duration,
    /// How often to poll even when notifications are active, in case they miss a change (5 s).
    pub safety_interval: Duration,
}

impl Default for FollowOptions {
    fn default() -> Self {
        Self {
            notifications: true,
            poll_interval: Duration::from_millis(500),
            safety_interval: Duration::from_secs(5),
        }
    }
}

/// Wakes the follower when something under `events/` changes.
struct Notifier {
    _watcher: ::notify::RecommendedWatcher,
    wake: Arc<Notify>,
}

impl Notifier {
    /// Starts watching `events/`, or `None` when it cannot be watched (it does not exist yet, or
    /// the platform refuses); the follower then polls.
    fn start(events_dir: &Path) -> Option<Self> {
        use ::notify::{RecursiveMode, Watcher};

        if !events_dir.is_dir() {
            return None;
        }
        let wake = Arc::new(Notify::new());
        let handle = Arc::clone(&wake);
        let mut watcher =
            ::notify::recommended_watcher(move |_result: ::notify::Result<::notify::Event>| {
                handle.notify_one();
            })
            .ok()?;
        watcher.watch(events_dir, RecursiveMode::Recursive).ok()?;
        Some(Self {
            _watcher: watcher,
            wake,
        })
    }
}

/// Where a follower is in the log.
struct Cursor {
    layout: Layout,
    /// The highest sequence number delivered or skipped.
    last_seq: u64,
    /// The offset in `current.log` up to which complete lines were read.
    offset: u64,
    /// The sequence number of the first line of the `current.log` being read; a different value
    /// means the file was replaced (rotation).
    file_first: Option<(u64, String)>,
    /// Whether to check once for events that were pruned before the start position.
    check_pruned: bool,
    /// Set when the log was truncated, deleted or replaced: if its numbering then starts at or
    /// below what was already delivered, the log was reset and delivery restarts from its start.
    restart_check: bool,
    warned_versions: BTreeSet<u64>,
}

impl Cursor {
    /// Handles a log whose first sequence number is `first` after a replacement.
    fn apply_restart(&mut self, first: u64) {
        if std::mem::take(&mut self.restart_check) && first <= self.last_seq {
            self.last_seq = first.saturating_sub(1);
        }
    }
}

fn storage(action: &'static str, path: &Path) -> impl FnOnce(io::Error) -> FunctorError {
    let path = path.to_path_buf();
    move |error| FunctorError::storage(action, &path, error)
}

/// The length of `path` up to and including its last newline.
fn complete_len(path: &Path) -> io::Result<u64> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error),
    };
    let len = file.metadata()?.len();
    if len == 0 {
        return Ok(0);
    }
    let mut window = SCAN_CHUNK.min(len);
    loop {
        let start = len - window;
        file.seek(SeekFrom::Start(start))?;
        let mut buffer = vec![0_u8; usize::try_from(window).unwrap_or(usize::MAX)];
        file.read_exact(&mut buffer)?;
        if let Some(position) = buffer.iter().rposition(|byte| *byte == b'\n') {
            return Ok(start + position as u64 + 1);
        }
        if start == 0 {
            return Ok(0);
        }
        window = (window * 2).min(len);
    }
}

impl Cursor {
    fn start(layout: Layout, position: EventPosition) -> Result<Self> {
        let current = layout.events_log();
        let mut cursor = Self {
            layout,
            last_seq: 0,
            offset: 0,
            file_first: None,
            check_pruned: false,
            restart_check: false,
            warned_versions: BTreeSet::new(),
        };
        match position {
            EventPosition::End => {
                let tail = scan_tail(&current).map_err(storage("read", &current))?;
                let archives = list_archives(&cursor.layout.events_archive_dir())
                    .map_err(storage("read", &cursor.layout.events_archive_dir()))?;
                cursor.last_seq = tail
                    .and_then(|t| t.last_seq)
                    .into_iter()
                    .chain(archives.last().map(|a| a.last))
                    .max()
                    .unwrap_or(0);
                cursor.offset = complete_len(&current).map_err(storage("read", &current))?;
                cursor.file_first = match File::open(&current) {
                    Ok(mut file) => first_key(&mut file).map_err(storage("read", &current))?,
                    Err(_) => None,
                };
            }
            EventPosition::After(seq) => {
                cursor.last_seq = seq;
                cursor.check_pruned = true;
            }
            // `Start` and any position this version does not know begin at the oldest event.
            _ => cursor.check_pruned = true,
        }
        Ok(cursor)
    }

    /// Delivers the complete lines in `bytes`, skipping what was already seen.
    fn deliver(&mut self, bytes: &[u8], source: &str, out: &mut Vec<Update>) {
        for line in bytes.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            let Ok(text) = std::str::from_utf8(line) else {
                out.push(Update::Warning(format!(
                    "skipped an unreadable line in {source}: not valid UTF-8"
                )));
                continue;
            };
            match decode_event(text) {
                Ok(event) => {
                    if event.seq > self.last_seq {
                        self.last_seq = event.seq;
                        out.push(Update::Event(event));
                    }
                }
                Err(LineProblem::Malformed(detail)) => out.push(Update::Warning(format!(
                    "skipped an unreadable line in {source}: {detail}"
                ))),
                Err(problem) => {
                    if let LineProblem::UnknownVersion(version) = problem
                        && self.warned_versions.insert(version)
                    {
                        out.push(Update::Warning(format!(
                            "ignoring events with unsupported envelope version {version}"
                        )));
                    }
                    // Ignored events still consume their sequence number.
                    if let Some(seq) = peek_seq(text) {
                        self.last_seq = self.last_seq.max(seq);
                    }
                }
            }
        }
    }

    /// Reads the archives that hold events not delivered yet.
    fn catch_up(&mut self, out: &mut Vec<Update>) -> Result<()> {
        let archive_dir = self.layout.events_archive_dir();
        let archives = list_archives(&archive_dir).map_err(storage("read", &archive_dir))?;
        if self.check_pruned {
            self.check_pruned = false;
            let current = self.layout.events_log();
            let oldest = match archives.first() {
                Some(archive) => Some(archive.first),
                None => match File::open(&current) {
                    Ok(mut file) => first_seq(&mut file).map_err(storage("read", &current))?,
                    Err(_) => None,
                },
            };
            if let Some(oldest) = oldest
                && oldest > self.last_seq.saturating_add(1)
            {
                out.push(Update::Warning(format!(
                    "events before seq {oldest} were pruned"
                )));
            }
        }
        let from = self.last_seq;
        for archive in archives.iter().filter(|a| a.last > from) {
            let bytes = fs::read(&archive.path).map_err(storage("read", &archive.path))?;
            let name = self.layout.relative(&archive.path).to_string();
            self.deliver(&bytes, &name, out);
        }
        Ok(())
    }

    /// Reads new complete lines of `current.log`. Returns `true` when the file was replaced since
    /// the last read, so the archives must be consulted again.
    fn read_current(&mut self, out: &mut Vec<Update>) -> Result<bool> {
        let path = self.layout.events_log();
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if self.file_first.is_some() || self.offset > 0 {
                    self.restart_check = true;
                }
                self.offset = 0;
                self.file_first = None;
                return Ok(false);
            }
            Err(error) => return Err(storage("read", &path)(error)),
        };
        let len = file.metadata().map_err(storage("read", &path))?.len();
        if len < self.offset {
            // Truncated or replaced by a shorter file.
            self.offset = 0;
            self.file_first = None;
            self.restart_check = true;
            return Ok(true);
        }
        let first = first_key(&mut file).map_err(storage("read", &path))?;
        match (self.file_first.take(), first) {
            (Some(known), Some(now)) if known != now => {
                self.offset = 0;
                self.restart_check = true;
                self.apply_restart(now.0);
                self.file_first = Some(now);
                return Ok(true);
            }
            (None, Some(now)) => {
                self.apply_restart(now.0);
                // Events between what was delivered and this file's first line may sit in an
                // archive that was created after this poll listed the archives (the file only
                // exists once the archive does), so read the archives again before this file.
                let gap = now.0 > self.last_seq.saturating_add(1);
                self.file_first = Some(now);
                if gap {
                    self.offset = 0;
                    return Ok(true);
                }
            }
            (known, _) => self.file_first = known,
        }
        if len == self.offset {
            return Ok(false);
        }
        file.seek(SeekFrom::Start(self.offset))
            .map_err(storage("read", &path))?;
        let mut buffer = Vec::new();
        file.take(len - self.offset)
            .read_to_end(&mut buffer)
            .map_err(storage("read", &path))?;
        // A partial last line is held back until its newline arrives.
        let Some(end) = buffer.iter().rposition(|byte| *byte == b'\n') else {
            return Ok(false);
        };
        let complete = &buffer[..=end];
        self.deliver(complete, "events/current.log", out);
        self.offset += complete.len() as u64;
        Ok(false)
    }

    /// One pass over the archives and the current log.
    fn poll(&mut self) -> Result<Vec<Update>> {
        let mut out = Vec::new();
        // A rotation between the two reads is noticed by `read_current`; go around again so the
        // events that moved into the new archive are not skipped.
        for _ in 0..3 {
            self.catch_up(&mut out)?;
            if !self.read_current(&mut out)? {
                break;
            }
        }
        Ok(out)
    }
}

/// A live view of the event log of one repository.
///
/// `next_within` is cancel-safe only while it waits; dropping it while it reads the files leaves
/// the follower unusable (it then reports an internal error).
pub struct Follower {
    cursor: Option<Cursor>,
    pending: VecDeque<Update>,
    notifier: Option<Notifier>,
    options: FollowOptions,
}

impl std::fmt::Debug for Follower {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Follower")
            .field("notifications", &self.notifier.is_some())
            .finish_non_exhaustive()
    }
}

impl Follower {
    /// Starts following the log of the repository rooted at `root`.
    ///
    /// `End` delivers only events published after this call; `Start` and `After` deliver
    /// existing events first, reading archives when they hold them.
    ///
    /// # Errors
    /// `Storage` when the log cannot be read.
    pub async fn open(
        root: &Path,
        position: EventPosition,
        options: FollowOptions,
    ) -> Result<Self> {
        let layout = Layout::new(root);
        let notifier = if options.notifications {
            Notifier::start(&layout.events_dir())
        } else {
            None
        };
        let cursor = run_blocking(move || Cursor::start(layout, position)).await?;
        Ok(Self {
            cursor: Some(cursor),
            pending: VecDeque::new(),
            notifier,
            options,
        })
    }

    /// Whether filesystem notifications are in use (otherwise the follower polls).
    #[must_use]
    pub fn uses_notifications(&self) -> bool {
        self.notifier.is_some()
    }

    /// The next update, waiting for new events.
    ///
    /// With `idle`, gives up after that long without anything new and returns `Ok(None)`.
    ///
    /// # Errors
    /// `Storage` when the log cannot be read; the follower can be used again afterwards.
    pub async fn next_within(&mut self, idle: Option<Duration>) -> Result<Option<Update>> {
        let deadline = idle.map(|idle| Instant::now() + idle);
        loop {
            if let Some(update) = self.pending.pop_front() {
                return Ok(Some(update));
            }
            let mut cursor = self.cursor.take().ok_or_else(|| {
                FunctorError::Internal("the follower was cancelled while reading".to_owned())
            })?;
            let (cursor, updates) = run_blocking(move || {
                let updates = cursor.poll();
                Ok((cursor, updates))
            })
            .await?;
            self.cursor = Some(cursor);
            let updates = updates?;
            if !updates.is_empty() {
                self.pending.extend(updates);
                continue;
            }

            let interval = if self.notifier.is_some() {
                self.options.safety_interval
            } else {
                self.options.poll_interval
            };
            let wait = match deadline {
                Some(deadline) => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return Ok(None);
                    }
                    interval.min(remaining)
                }
                None => interval,
            };
            match &self.notifier {
                Some(notifier) => {
                    let _ = tokio::time::timeout(wait, notifier.wake.notified()).await;
                }
                None => tokio::time::sleep(wait).await,
            }
        }
    }

    /// The next event, skipping warnings; waits without limit.
    ///
    /// # Errors
    /// See [`Follower::next_within`].
    pub async fn next_event(&mut self) -> Result<Event> {
        loop {
            if let Some(Update::Event(event)) = self.next_within(None).await? {
                return Ok(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use functor_model::{
        EventData, EventId, EventKind, EventSubject, NewEvent, RepositoryId, Timestamp,
    };

    use super::*;
    use crate::log::archive_name;
    use crate::wire::encode_event;

    fn line(seq: u64) -> String {
        let event = Event::new(
            EventId::from_bits(u128::from(seq)),
            seq,
            Timestamp::parse("2026-10-12T18:00:00Z").unwrap(),
            RepositoryId::from_bits(1),
            NewEvent::new(
                EventKind::ValidationCompleted,
                EventSubject::Repository(RepositoryId::from_bits(1)),
                EventData::new(),
            ),
        )
        .unwrap();
        format!("{}\n", encode_event(&event))
    }

    fn write(path: &Path, first: u64, last: u64) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, (first..=last).map(line).collect::<String>()).unwrap();
    }

    fn delivered(updates: &[Update]) -> Vec<u64> {
        updates
            .iter()
            .filter_map(|u| match u {
                Update::Event(event) => Some(event.seq),
                Update::Warning(_) => None,
            })
            .collect()
    }

    /// A rotation can complete between listing the archives and opening the current log. The log
    /// seen then starts after a gap, and the archive that fills the gap exists by now: the
    /// archives must be read again before the log, or those events are lost for good.
    #[test]
    fn a_current_log_that_starts_after_a_gap_sends_the_follower_back_to_the_archives() {
        let dir = tempfile::tempdir().unwrap();
        let layout = Layout::new(dir.path());
        write(
            &layout.events_archive_dir().join(archive_name(21, 25)),
            21,
            25,
        );
        write(&layout.events_log(), 26, 30);

        let mut cursor = Cursor::start(layout, EventPosition::After(20)).unwrap();
        // The archives were listed before the rotation: only now is the log read.
        cursor.check_pruned = false;
        let mut out = Vec::new();
        assert!(
            cursor.read_current(&mut out).unwrap(),
            "the gap is noticed and the archives are consulted again"
        );
        assert_eq!(
            delivered(&out),
            Vec::<u64>::new(),
            "nothing is delivered across the gap"
        );

        cursor.catch_up(&mut out).unwrap();
        assert!(!cursor.read_current(&mut out).unwrap());
        assert_eq!(delivered(&out), (21..=30).collect::<Vec<u64>>());
    }

    #[test]
    fn a_legitimate_gap_does_not_loop_forever() {
        // Events 1..=4 were pruned: the log starts at 5 and no archive holds the gap.
        let dir = tempfile::tempdir().unwrap();
        let layout = Layout::new(dir.path());
        write(&layout.events_log(), 5, 7);
        let mut cursor = Cursor::start(layout, EventPosition::Start).unwrap();
        let updates = cursor.poll().unwrap();
        assert_eq!(delivered(&updates), [5, 6, 7]);
        assert!(
            updates
                .iter()
                .any(|u| matches!(u, Update::Warning(w) if w == "events before seq 5 were pruned"))
        );
    }

    #[test]
    fn a_log_replaced_between_reads_is_followed_through_the_archive() {
        let dir = tempfile::tempdir().unwrap();
        let layout = Layout::new(dir.path());
        write(&layout.events_log(), 1, 3);
        let mut cursor = Cursor::start(layout.clone(), EventPosition::Start).unwrap();
        assert_eq!(delivered(&cursor.poll().unwrap()), [1, 2, 3]);

        // Rotation: the log becomes an archive (and holds 4 and 5 by then), a new log starts.
        write(&layout.events_log(), 1, 5);
        fs::create_dir_all(layout.events_archive_dir()).unwrap();
        fs::rename(
            layout.events_log(),
            layout.events_archive_dir().join(archive_name(1, 5)),
        )
        .unwrap();
        write(&layout.events_log(), 6, 7);
        assert_eq!(delivered(&cursor.poll().unwrap()), [4, 5, 6, 7]);
    }
}
