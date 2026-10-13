//! Publishing: sequence numbers, the stored format, rotation, pruning and crash recovery.

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use common::{Repo, created, events_in, seqs_in};
use functor_core::{
    ErrorKind, EventPosition, EventPublisher, EventSource, FunctorError, IdGenerator,
};
use functor_events::{
    EventLogOptions, FileEventLog, Limits, MAX_ARCHIVES, MAX_LOG_BYTES, archive_name, decode_event,
};
use functor_model::{EventKind, RepositoryId, Timestamp};
use functor_repository::{Layout, WriteLock};
use functor_testkit::Env;

// ---------------------------------------------------------------------------------------------
// appending
// ---------------------------------------------------------------------------------------------

#[test]
fn the_documented_limits_are_the_defaults() {
    assert_eq!(MAX_LOG_BYTES, 10 * 1024 * 1024);
    assert_eq!(MAX_ARCHIVES, 10);
    assert_eq!(
        Limits::default(),
        Limits {
            max_bytes: 10 * 1024 * 1024,
            max_archives: 10
        }
    );
}

#[test]
fn the_first_event_has_sequence_1_and_numbering_continues() {
    let repo = Repo::new();
    let log = repo.log();
    for expected in 1..=5 {
        let published = log.publish_blocking(created(expected)).unwrap();
        assert_eq!(published.event.seq, expected);
        assert_eq!(published.warnings, Vec::<String>::new());
    }
    assert_eq!(repo.all_seqs(), [1, 2, 3, 4, 5]);
}

#[test]
fn events_are_stored_one_per_line_and_decode_to_what_was_published() {
    let repo = Repo::new();
    let log = repo.log();
    let first = log.publish_blocking(created(1)).unwrap().event;
    let second = log.publish_blocking(created(2)).unwrap().event;

    let text = fs::read_to_string(repo.current()).unwrap();
    assert!(text.ends_with('\n'));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(decode_event(lines[0]).unwrap(), first);
    assert_eq!(decode_event(lines[1]).unwrap(), second);
    assert_eq!(first.kind, EventKind::ArtifactCreated);
    assert_eq!(first.origin, RepositoryId::from_bits(1));
}

#[test]
fn ids_and_times_come_from_the_injected_collaborators() {
    let repo = Repo::new();
    let log = repo.log();
    let start = repo.env.now();
    let first = log.publish_blocking(created(1)).unwrap().event;
    repo.env.advance(90);
    let second = log.publish_blocking(created(2)).unwrap().event;
    assert_eq!(first.at, start);
    assert_eq!(second.at, Timestamp::parse("2026-10-12T18:01:30Z").unwrap());
    assert_ne!(first.id, second.id);
    // The injected generator is what supplies ids: a fresh environment repeats them.
    let other = Repo::new();
    let again = other.log();
    assert_eq!(
        again.publish_blocking(created(1)).unwrap().event.id,
        first.id
    );
}

#[test]
fn the_events_directory_is_created_when_missing() {
    let repo = Repo::new();
    assert!(!repo.root().join("events").exists());
    repo.log().publish_blocking(created(1)).unwrap();
    assert!(repo.current().is_file());
}

#[test]
fn publishing_leaves_nothing_but_the_log_and_the_lock_behind() {
    let repo = Repo::new();
    let log = repo.log();
    for n in 1..=3 {
        log.publish_blocking(created(n)).unwrap();
    }
    let mut names: Vec<String> = fs::read_dir(repo.root().join("events"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["current.log"]);
}

#[tokio::test]
async fn the_async_publisher_works_through_a_trait_object() {
    let repo = Repo::new();
    let log = repo.log();
    let publisher: &dyn EventPublisher = &log;
    let event = publisher.publish(created(1)).await.unwrap();
    assert_eq!(event.seq, 1);
    assert_eq!(publisher.publish(created(2)).await.unwrap().seq, 2);
}

#[tokio::test]
async fn published_events_can_be_read_back_through_the_event_source() {
    let repo = Repo::new();
    let log = repo.log();
    for n in 1..=3 {
        log.publish_blocking(created(n)).unwrap();
    }
    let source: &dyn EventSource = &log;
    let mut subscription = source.subscribe(EventPosition::Start).await.unwrap();
    for expected in 1..=3 {
        let event = subscription.next().await.unwrap().unwrap();
        assert_eq!(event.seq, expected);
    }
}

// ---------------------------------------------------------------------------------------------
// damaged logs
// ---------------------------------------------------------------------------------------------

#[test]
fn a_missing_log_with_no_archives_starts_at_1_even_if_the_directory_exists() {
    let repo = Repo::new();
    fs::create_dir_all(repo.root().join("events/archive")).unwrap();
    assert_eq!(
        repo.log().publish_blocking(created(1)).unwrap().event.seq,
        1
    );
}

#[test]
fn a_torn_last_line_is_terminated_before_the_next_event() {
    let repo = Repo::new();
    let log = repo.log();
    log.publish_blocking(created(1)).unwrap();
    log.publish_blocking(created(2)).unwrap();
    // A crash left half of an event on the last line.
    let mut text = fs::read_to_string(repo.current()).unwrap();
    text.push_str("{\"v\":1,\"id\":\"evt_000000000000000000000000");
    fs::write(repo.current(), &text).unwrap();

    let next = log.publish_blocking(created(3)).unwrap().event;
    assert_eq!(
        next.seq, 3,
        "numbering continues from the last complete event"
    );

    let after = fs::read_to_string(repo.current()).unwrap();
    assert!(after.ends_with('\n'));
    let lines: Vec<&str> = after.lines().collect();
    assert_eq!(lines.len(), 4, "the fragment is its own line");
    assert!(decode_event(lines[2]).is_err());
    assert_eq!(decode_event(lines[3]).unwrap().seq, 3);
}

#[test]
fn a_torn_line_that_is_complete_json_still_reserves_its_sequence_number() {
    let repo = Repo::new();
    let log = repo.log();
    log.publish_blocking(created(1)).unwrap();
    log.publish_blocking(created(2)).unwrap();
    // The crash cut off only the newline of the last event.
    let text = fs::read_to_string(repo.current()).unwrap();
    fs::write(repo.current(), text.trim_end_matches('\n')).unwrap();

    assert_eq!(log.publish_blocking(created(3)).unwrap().event.seq, 3);
    assert_eq!(repo.all_seqs(), [1, 2, 3]);
}

#[test]
fn garbage_before_the_last_valid_event_does_not_confuse_numbering() {
    let repo = Repo::new();
    let log = repo.log();
    log.publish_blocking(created(1)).unwrap();
    let mut text = fs::read_to_string(repo.current()).unwrap();
    text.push_str("not json at all\n\n{\"seq\":\"x\"}\n");
    fs::write(repo.current(), &text).unwrap();
    assert_eq!(log.publish_blocking(created(2)).unwrap().event.seq, 2);
}

#[test]
fn numbering_looks_past_a_long_tail_of_unusable_lines() {
    let repo = Repo::new();
    let log = repo.log();
    log.publish_blocking(created(1)).unwrap();
    let mut text = fs::read_to_string(repo.current()).unwrap();
    // More garbage than one scan window holds.
    for _ in 0..4000 {
        text.push_str("this line is not an event and is fairly long, to fill the tail\n");
    }
    fs::write(repo.current(), &text).unwrap();
    assert_eq!(log.publish_blocking(created(2)).unwrap().event.seq, 2);
}

#[test]
fn an_unusable_log_does_not_reuse_numbers_when_archives_exist() {
    let repo = Repo::new();
    let log = repo.small_log(200, 5);
    for n in 1..=6 {
        log.publish_blocking(created(n)).unwrap();
    }
    assert_ne!(repo.archive_names().len(), 0);
    // Event 6 was only in the log that was just destroyed; the newest archive ends at 5.
    fs::write(repo.current(), "garbage only\n").unwrap();
    let next = log.publish_blocking(created(7)).unwrap().event.seq;
    assert_eq!(next, 6, "numbering continues from the newest archive");
}

#[test]
fn a_log_that_cannot_be_appended_to_fails_with_storage() {
    let repo = Repo::new();
    let log = repo.log();
    log.publish_blocking(created(1)).unwrap();
    fs::remove_file(repo.current()).unwrap();
    fs::create_dir(repo.current()).unwrap();
    let error = log.publish_blocking(created(2)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Storage);
}

#[test]
fn a_held_write_lock_makes_publishing_fail_with_locked() {
    let repo = Repo::new();
    fs::create_dir_all(repo.root().join(".functor")).unwrap();
    let log =
        repo.log_with(EventLogOptions::default().with_lock_timeout(Duration::from_millis(100)));
    let layout = Layout::new(repo.root());
    let holder = WriteLock::new(&layout, Duration::from_secs(5));

    let (ready, wait) = std::sync::mpsc::channel();
    let (release, hold) = std::sync::mpsc::channel::<()>();
    let handle = thread::spawn(move || {
        holder
            .run(|| {
                ready.send(()).unwrap();
                hold.recv().unwrap();
                Ok(())
            })
            .unwrap();
    });
    wait.recv().unwrap();
    let error = log.publish_blocking(created(1)).unwrap_err();
    assert!(matches!(error, FunctorError::Locked { .. }), "{error:?}");
    assert!(!repo.current().exists(), "nothing was written");
    release.send(()).unwrap();
    handle.join().unwrap();
    assert_eq!(log.publish_blocking(created(1)).unwrap().event.seq, 1);
}

// ---------------------------------------------------------------------------------------------
// rotation
// ---------------------------------------------------------------------------------------------

/// The size of one published event line for these tests.
fn line_len(repo: &Repo) -> u64 {
    let probe = Repo::new();
    probe.log().publish_blocking(created(1)).unwrap();
    let _ = repo;
    fs::metadata(probe.current()).unwrap().len()
}

#[test]
fn the_log_rotates_when_it_reaches_the_limit_and_numbering_continues() {
    let repo = Repo::new();
    let one = line_len(&repo);
    let log = repo.small_log(one * 3, 10);

    for n in 1..=3 {
        log.publish_blocking(created(n)).unwrap();
    }
    assert_eq!(
        repo.archive_names(),
        Vec::<String>::new(),
        "at the limit, not yet rotated"
    );

    // The next append finds the log at the limit, rotates first, and starts a fresh log.
    let fourth = log.publish_blocking(created(4)).unwrap();
    assert_eq!(fourth.event.seq, 4);
    assert_eq!(fourth.warnings, Vec::<String>::new());
    assert_eq!(repo.archive_names(), [archive_name(1, 3)]);
    assert_eq!(
        seqs_in(&repo.archive_dir().join(archive_name(1, 3))),
        [1, 2, 3]
    );
    assert_eq!(
        seqs_in(&repo.current()),
        [4],
        "the new log starts with the new event"
    );
}

#[test]
fn the_current_log_is_never_empty_after_a_rotation() {
    let repo = Repo::new();
    let one = line_len(&repo);
    let log = repo.small_log(one * 2, 10);
    for n in 1..=20 {
        log.publish_blocking(created(n)).unwrap();
        let len = fs::metadata(repo.current()).unwrap().len();
        assert!(
            len >= one,
            "after event {n} the log holds at least one event"
        );
    }
}

#[test]
fn archive_names_hold_the_first_and_last_sequence_zero_padded() {
    assert_eq!(archive_name(1, 3), "events-000000000001-000000000003.log");
    assert_eq!(archive_name(41, 99), "events-000000000041-000000000099.log");
    let repo = Repo::new();
    let one = line_len(&repo);
    let log = repo.small_log(one * 2, 10);
    for n in 1..=9 {
        log.publish_blocking(created(n)).unwrap();
    }
    for name in repo.archive_names() {
        let seqs = seqs_in(&repo.archive_dir().join(&name));
        assert_eq!(
            name,
            archive_name(*seqs.first().unwrap(), *seqs.last().unwrap()),
            "each archive is named after what it holds"
        );
    }
}

#[test]
fn sequence_numbers_are_gap_free_across_the_current_log_and_every_archive() {
    let repo = Repo::new();
    let one = line_len(&repo);
    let log = repo.small_log(one * 3, 100);
    for n in 1..=50 {
        log.publish_blocking(created(n)).unwrap();
    }
    let all = repo.all_seqs();
    assert_eq!(all, (1..=50).collect::<Vec<u64>>());
}

#[test]
fn exactly_the_ten_newest_archives_are_retained_and_the_oldest_is_pruned() {
    let repo = Repo::new();
    let one = line_len(&repo);
    // The documented archive count, with a tiny size limit.
    let log = repo.small_log(one, MAX_ARCHIVES);
    for n in 1..=30 {
        log.publish_blocking(created(n)).unwrap();
    }
    let names = repo.archive_names();
    assert_eq!(names.len(), 10, "{names:?}");
    // One event per file: the archives hold the 10 events before the one in `current.log`.
    let seqs = repo.all_seqs();
    assert_eq!(seqs, (20..=30).collect::<Vec<u64>>());
    assert!(
        !names.contains(&archive_name(1, 1)),
        "the oldest archives are gone"
    );
}

#[test]
fn a_crash_between_the_rename_and_the_first_write_is_recovered() {
    let repo = Repo::new();
    let one = line_len(&repo);
    let log = repo.small_log(one * 2, 10);
    for n in 1..=3 {
        log.publish_blocking(created(n)).unwrap();
    }
    assert_eq!(repo.archive_names(), [archive_name(1, 2)]);
    // The process died after renaming the log and before writing the new one.
    fs::remove_file(repo.current()).unwrap();
    // Event 3 was in the removed file, so the newest archive decides: 2, then 3 again is not
    // possible; numbering continues from what is still on disk.
    let next = log.publish_blocking(created(4)).unwrap().event.seq;
    assert_eq!(next, 3);
    assert_eq!(repo.all_seqs(), [1, 2, 3]);
}

#[test]
fn a_rotation_that_cannot_complete_only_warns() {
    let repo = Repo::new();
    let one = line_len(&repo);
    let log = repo.small_log(one * 2, 10);
    log.publish_blocking(created(1)).unwrap();
    log.publish_blocking(created(2)).unwrap();
    // The archive the rotation would create already exists.
    fs::create_dir_all(repo.archive_dir()).unwrap();
    fs::write(repo.archive_dir().join(archive_name(1, 2)), "occupied").unwrap();

    let third = log.publish_blocking(created(3)).unwrap();
    assert_eq!(third.event.seq, 3, "the event is appended regardless");
    assert_eq!(third.warnings.len(), 1, "{:?}", third.warnings);
    assert!(
        third.warnings[0].contains("not rotated"),
        "{:?}",
        third.warnings
    );
    assert_eq!(
        seqs_in(&repo.current()),
        [1, 2, 3],
        "the log simply grows past the limit"
    );
}

#[test]
fn a_prune_that_cannot_complete_only_warns() {
    let repo = Repo::new();
    let one = line_len(&repo);
    let log = repo.small_log(one, 1);
    log.publish_blocking(created(1)).unwrap();
    log.publish_blocking(created(2)).unwrap(); // archive 1-1
    // Something that is not a removable file sits where the oldest archive would be pruned.
    fs::remove_file(repo.archive_dir().join(archive_name(1, 1))).unwrap();
    fs::create_dir(repo.archive_dir().join(archive_name(1, 1))).unwrap();

    let third = log.publish_blocking(created(3)).unwrap(); // rotates 2-2, must prune 1-1
    assert_eq!(third.event.seq, 3);
    assert_eq!(third.warnings.len(), 1, "{:?}", third.warnings);
    assert!(
        third.warnings[0].contains("not removed"),
        "{:?}",
        third.warnings
    );
}

#[test]
fn unrelated_files_in_the_archive_directory_are_ignored_and_kept() {
    let repo = Repo::new();
    let one = line_len(&repo);
    let log = repo.small_log(one, 1);
    fs::create_dir_all(repo.archive_dir()).unwrap();
    fs::write(repo.archive_dir().join("notes.txt"), "keep me").unwrap();
    for n in 1..=5 {
        log.publish_blocking(created(n)).unwrap();
    }
    assert!(repo.archive_dir().join("notes.txt").is_file());
    assert_eq!(
        repo.archive_names().len(),
        2,
        "one archive plus the unrelated file"
    );
}

// ---------------------------------------------------------------------------------------------
// concurrency
// ---------------------------------------------------------------------------------------------

#[test]
fn concurrent_publishers_never_duplicate_or_skip_a_sequence_number() {
    let repo = Repo::new();
    fs::create_dir_all(repo.root().join(".functor")).unwrap();
    let root = repo.root().to_path_buf();
    let env = Arc::new(Env::new());

    let workers: Vec<_> = (0..8_u64)
        .map(|worker| {
            let root = root.clone();
            let env = Arc::clone(&env);
            thread::spawn(move || {
                // Separate handles, like separate processes: only the file lock links them.
                let log = FileEventLog::new(
                    &root,
                    RepositoryId::from_bits(1),
                    env.clock(),
                    env.ids(),
                    EventLogOptions::default().with_lock_timeout(Duration::from_secs(60)),
                );
                (0..25)
                    .map(|n| {
                        log.publish_blocking(created(worker * 100 + n))
                            .unwrap()
                            .event
                            .seq
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let mut returned: Vec<u64> = workers
        .into_iter()
        .flat_map(|w| w.join().unwrap())
        .collect();
    returned.sort_unstable();
    assert_eq!(returned, (1..=200).collect::<Vec<u64>>());
    assert_eq!(
        repo.all_seqs(),
        (1..=200).collect::<Vec<u64>>(),
        "the file is in order too"
    );
}

#[test]
fn concurrent_publishers_stay_gap_free_across_rotations() {
    let repo = Repo::new();
    fs::create_dir_all(repo.root().join(".functor")).unwrap();
    let one = line_len(&repo);
    let root = repo.root().to_path_buf();
    let env = Arc::new(Env::new());
    let workers: Vec<_> = (0..4_u64)
        .map(|worker| {
            let root = root.clone();
            let env = Arc::clone(&env);
            thread::spawn(move || {
                let log = FileEventLog::new(
                    &root,
                    RepositoryId::from_bits(1),
                    env.clock(),
                    env.ids(),
                    EventLogOptions::default()
                        .with_lock_timeout(Duration::from_secs(60))
                        .with_limits(Limits {
                            max_bytes: one * 4,
                            max_archives: 1000,
                        }),
                );
                for n in 0..30 {
                    log.publish_blocking(created(worker * 100 + n)).unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(repo.all_seqs(), (1..=120).collect::<Vec<u64>>());
    assert!(
        repo.archive_names().len() >= 20,
        "rotation happened many times"
    );
}

#[test]
fn events_decode_from_every_file_after_rotation() {
    let repo = Repo::new();
    let one = line_len(&repo);
    let log = repo.small_log(one * 2, 10);
    for n in 1..=7 {
        log.publish_blocking(created(n)).unwrap();
    }
    let mut all = Vec::new();
    for name in repo.archive_names() {
        all.extend(events_in(&repo.archive_dir().join(name)));
    }
    all.extend(events_in(&repo.current()));
    let seqs: Vec<u64> = all.iter().map(|e| e.seq).collect();
    assert_eq!(seqs, (1..=7).collect::<Vec<u64>>());
    let ids: std::collections::BTreeSet<_> = all.iter().map(|e| e.id).collect();
    assert_eq!(ids.len(), 7, "every event has its own id");
    let _ = IdGenerator::event_id(&*Env::new().ids());
}
