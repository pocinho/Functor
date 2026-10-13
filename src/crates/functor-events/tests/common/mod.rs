//! Shared helpers for the event log tests.

// Test support code: panicking on unexpected errors is the point, and not every test file uses
// every helper.
#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use functor_events::{EventLogOptions, FileEventLog, Limits, decode_event, peek_seq};
use functor_model::{
    ArtifactId, Event, EventData, EventKind, EventSubject, NewEvent, RepositoryId,
};
use functor_testkit::Env;
use tempfile::TempDir;

pub struct Repo {
    pub dir: TempDir,
    pub env: Env,
}

impl Repo {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        // The write lock lives in `.functor/`, which every repository has.
        fs::create_dir_all(dir.path().join(".functor")).unwrap();
        Self {
            dir,
            env: Env::new(),
        }
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn log(&self) -> FileEventLog {
        self.log_with(EventLogOptions::default())
    }

    pub fn log_with(&self, options: EventLogOptions) -> FileEventLog {
        FileEventLog::new(
            self.root(),
            RepositoryId::from_bits(1),
            self.env.clock(),
            self.env.ids(),
            options,
        )
    }

    /// A log that rotates at `max_bytes` and keeps `max_archives` archives.
    pub fn small_log(&self, max_bytes: u64, max_archives: usize) -> FileEventLog {
        self.log_with(EventLogOptions::default().with_limits(Limits {
            max_bytes,
            max_archives,
        }))
    }

    pub fn current(&self) -> PathBuf {
        self.root().join("events/current.log")
    }

    pub fn archive_dir(&self) -> PathBuf {
        self.root().join("events/archive")
    }

    pub fn archive_names(&self) -> Vec<String> {
        let mut names: Vec<String> = match fs::read_dir(self.archive_dir()) {
            Ok(entries) => entries
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect(),
            Err(_) => Vec::new(),
        };
        names.sort();
        names
    }

    /// Every sequence number in the archives (oldest first) followed by `current.log`.
    pub fn all_seqs(&self) -> Vec<u64> {
        let mut seqs = Vec::new();
        for name in self.archive_names() {
            seqs.extend(seqs_in(&self.archive_dir().join(name)));
        }
        seqs.extend(seqs_in(&self.current()));
        seqs
    }
}

pub fn seqs_in(path: &Path) -> Vec<u64> {
    match fs::read_to_string(path) {
        Ok(text) => text.lines().filter_map(peek_seq).collect(),
        Err(_) => Vec::new(),
    }
}

/// An `artifact.created` event about artifact number `n`.
pub fn created(n: u64) -> NewEvent {
    NewEvent::new(
        EventKind::ArtifactCreated,
        EventSubject::Artifact(ArtifactId::from_bits(u128::from(n))),
        EventData::new()
            .with("slug", format!("artifact-{n}").as_str())
            .with("type", "document")
            .with("revision", 1_i64),
    )
}

/// The events stored in a file, in order, skipping unusable lines.
pub fn events_in(path: &Path) -> Vec<Event> {
    match fs::read_to_string(path) {
        Ok(text) => text
            .lines()
            .filter_map(|line| decode_event(line).ok())
            .collect(),
        Err(_) => Vec::new(),
    }
}
