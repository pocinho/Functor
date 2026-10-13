//! Following the log: existing and new events, start positions, partial lines, rotation, pruning
//! and restarts.

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::io::Write;
use std::time::Duration;

use common::{Repo, created};
use functor_core::{ErrorKind, EventPosition};
use functor_events::{FollowOptions, Follower, Limits, Update, archive_name};

const IDLE: Duration = Duration::from_millis(300);

fn polling() -> FollowOptions {
    FollowOptions {
        notifications: false,
        poll_interval: Duration::from_millis(15),
        safety_interval: Duration::from_millis(100),
    }
}

fn notified() -> FollowOptions {
    FollowOptions {
        notifications: true,
        poll_interval: Duration::from_millis(15),
        safety_interval: Duration::from_millis(250),
    }
}

async fn follow(repo: &Repo, position: EventPosition, options: FollowOptions) -> Follower {
    Follower::open(repo.root(), position, options)
        .await
        .unwrap()
}

/// Everything the follower yields until it has been idle for `IDLE`.
async fn drain(follower: &mut Follower) -> Vec<Update> {
    let mut updates = Vec::new();
    while let Some(update) = follower.next_within(Some(IDLE)).await.unwrap() {
        updates.push(update);
    }
    updates
}

fn seqs(updates: &[Update]) -> Vec<u64> {
    updates
        .iter()
        .filter_map(|u| match u {
            Update::Event(event) => Some(event.seq),
            Update::Warning(_) => None,
        })
        .collect()
}

fn warnings(updates: &[Update]) -> Vec<&str> {
    updates
        .iter()
        .filter_map(|u| match u {
            Update::Warning(text) => Some(text.as_str()),
            Update::Event(_) => None,
        })
        .collect()
}

fn publish(repo: &Repo, from: u64, to: u64) {
    let log = repo.log();
    for n in from..=to {
        log.publish_blocking(created(n)).unwrap();
    }
}

// ---------------------------------------------------------------------------------------------
// start positions
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn start_delivers_the_existing_events_in_order_and_then_waits() {
    let repo = Repo::new();
    publish(&repo, 1, 3);
    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    let updates = drain(&mut follower).await;
    assert_eq!(seqs(&updates), [1, 2, 3]);
    assert_eq!(warnings(&updates), Vec::<&str>::new());
}

#[tokio::test]
async fn end_delivers_only_events_published_afterwards() {
    let repo = Repo::new();
    publish(&repo, 1, 3);
    let mut follower = follow(&repo, EventPosition::End, polling()).await;
    assert_eq!(seqs(&drain(&mut follower).await), Vec::<u64>::new());
    publish(&repo, 4, 5);
    assert_eq!(seqs(&drain(&mut follower).await), [4, 5]);
}

#[tokio::test]
async fn end_in_an_empty_repository_delivers_the_first_events() {
    let repo = Repo::new();
    let mut follower = follow(&repo, EventPosition::End, polling()).await;
    publish(&repo, 1, 2);
    assert_eq!(seqs(&drain(&mut follower).await), [1, 2]);
}

#[tokio::test]
async fn after_delivers_only_greater_sequence_numbers() {
    let repo = Repo::new();
    publish(&repo, 1, 5);
    let mut follower = follow(&repo, EventPosition::After(3), polling()).await;
    assert_eq!(seqs(&drain(&mut follower).await), [4, 5]);

    let mut beyond = follow(&repo, EventPosition::After(50), polling()).await;
    assert_eq!(seqs(&drain(&mut beyond).await), Vec::<u64>::new());
    publish(&repo, 6, 6);
    assert_eq!(
        seqs(&drain(&mut beyond).await),
        Vec::<u64>::new(),
        "6 is not greater than 50"
    );
}

#[tokio::test]
async fn an_event_is_never_delivered_twice() {
    let repo = Repo::new();
    publish(&repo, 1, 4);
    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    assert_eq!(seqs(&drain(&mut follower).await), [1, 2, 3, 4]);
    assert_eq!(seqs(&drain(&mut follower).await), Vec::<u64>::new());
    publish(&repo, 5, 5);
    assert_eq!(seqs(&drain(&mut follower).await), [5]);
}

// ---------------------------------------------------------------------------------------------
// waiting for new events
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn new_events_arrive_while_waiting_with_notifications() {
    let repo = Repo::new();
    fs::create_dir_all(repo.root().join("events")).unwrap();
    let mut follower = follow(&repo, EventPosition::End, notified()).await;
    assert!(follower.uses_notifications(), "the platform watcher starts");

    let log = repo.log();
    let writer = tokio::task::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(150));
        log.publish_blocking(created(1)).unwrap();
    });
    let event = tokio::time::timeout(Duration::from_secs(10), follower.next_event())
        .await
        .expect("the event arrives")
        .unwrap();
    assert_eq!(event.seq, 1);
    writer.await.unwrap();
}

#[tokio::test]
async fn new_events_arrive_while_polling_without_notifications() {
    let repo = Repo::new();
    let mut follower = follow(&repo, EventPosition::End, polling()).await;
    assert!(!follower.uses_notifications());
    let log = repo.log();
    let writer = tokio::task::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(100));
        log.publish_blocking(created(1)).unwrap();
    });
    let event = tokio::time::timeout(Duration::from_secs(10), follower.next_event())
        .await
        .expect("the event arrives")
        .unwrap();
    assert_eq!(event.seq, 1);
    writer.await.unwrap();
}

#[tokio::test]
async fn a_log_created_after_the_watch_started_is_followed() {
    let repo = Repo::new();
    fs::create_dir_all(repo.root().join("events")).unwrap();
    for options in [polling(), notified()] {
        let probe = Repo::new();
        fs::create_dir_all(probe.root().join("events")).unwrap();
        let mut follower = follow(&probe, EventPosition::Start, options).await;
        assert_eq!(seqs(&drain(&mut follower).await), Vec::<u64>::new());
        assert!(!probe.current().exists());
        publish(&probe, 1, 2);
        assert_eq!(seqs(&drain(&mut follower).await), [1, 2]);
    }
}

#[tokio::test]
async fn an_events_directory_created_after_the_watch_started_is_followed() {
    let repo = Repo::new();
    let mut follower = follow(&repo, EventPosition::Start, notified()).await;
    assert!(
        !follower.uses_notifications(),
        "there is nothing to watch yet: it polls"
    );
    publish(&repo, 1, 1);
    assert_eq!(seqs(&drain(&mut follower).await), [1]);
}

#[tokio::test]
async fn the_idle_timeout_ends_a_wait_with_no_events() {
    let repo = Repo::new();
    let mut follower = follow(&repo, EventPosition::End, polling()).await;
    let started = std::time::Instant::now();
    let nothing = follower
        .next_within(Some(Duration::from_millis(150)))
        .await
        .unwrap();
    assert!(nothing.is_none());
    assert!(started.elapsed() >= Duration::from_millis(140));
    assert!(started.elapsed() < Duration::from_secs(5));
}

// ---------------------------------------------------------------------------------------------
// partial and damaged lines
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_partial_last_line_is_held_back_until_its_newline_arrives() {
    let repo = Repo::new();
    publish(&repo, 1, 2);
    let line = fs::read_to_string(repo.current())
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_owned();
    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    assert_eq!(seqs(&drain(&mut follower).await), [1, 2]);

    // A third event arrives in two writes: first without its newline.
    let third = line.replacen("\"seq\":1", "\"seq\":3", 1);
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(repo.current())
        .unwrap();
    file.write_all(third.as_bytes()).unwrap();
    file.flush().unwrap();
    assert_eq!(
        seqs(&drain(&mut follower).await),
        Vec::<u64>::new(),
        "held back"
    );

    file.write_all(b"\n").unwrap();
    file.flush().unwrap();
    assert_eq!(seqs(&drain(&mut follower).await), [3]);
}

#[tokio::test]
async fn a_half_written_event_is_not_delivered_in_pieces() {
    let repo = Repo::new();
    publish(&repo, 1, 1);
    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    assert_eq!(seqs(&drain(&mut follower).await), [1]);

    let line = fs::read_to_string(repo.current())
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .replacen("\"seq\":1", "\"seq\":2", 1);
    let (head, tail) = line.split_at(line.len() / 2);
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(repo.current())
        .unwrap();
    file.write_all(b"\n").ok(); // the first line already ends with a newline
    file.write_all(head.as_bytes()).unwrap();
    file.flush().unwrap();
    let half_written = drain(&mut follower).await;
    assert_eq!(seqs(&half_written), Vec::<u64>::new());
    assert_eq!(
        warnings(&half_written),
        Vec::<&str>::new(),
        "no warning for a line still being written"
    );

    file.write_all(tail.as_bytes()).unwrap();
    file.write_all(b"\n").unwrap();
    file.flush().unwrap();
    assert_eq!(seqs(&drain(&mut follower).await), [2]);
}

#[tokio::test]
async fn an_unreadable_line_is_skipped_with_a_warning_and_delivery_continues() {
    let repo = Repo::new();
    publish(&repo, 1, 1);
    let mut text = fs::read_to_string(repo.current()).unwrap();
    text.push_str("this is not an event\n");
    fs::write(repo.current(), text).unwrap();
    publish(&repo, 2, 2);

    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    let updates = drain(&mut follower).await;
    assert_eq!(seqs(&updates), [1, 2]);
    let warnings = warnings(&updates);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("events/current.log"), "{warnings:?}");
}

#[tokio::test]
async fn a_torn_line_left_by_a_crash_is_skipped_once_the_writer_terminated_it() {
    let repo = Repo::new();
    publish(&repo, 1, 2);
    let mut text = fs::read_to_string(repo.current()).unwrap();
    text.push_str("{\"v\":1,\"id\":\"evt_0000");
    fs::write(repo.current(), text).unwrap();

    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    let first = drain(&mut follower).await;
    assert_eq!(
        seqs(&first),
        [1, 2],
        "the fragment is not delivered or reported yet"
    );
    assert_eq!(warnings(&first), Vec::<&str>::new());

    publish(&repo, 3, 3); // terminates the fragment, then appends
    let second = drain(&mut follower).await;
    assert_eq!(seqs(&second), [3]);
    assert_eq!(warnings(&second).len(), 1);
}

#[tokio::test]
async fn events_of_an_unknown_envelope_version_are_ignored_with_one_warning() {
    let repo = Repo::new();
    publish(&repo, 1, 1);
    let line = fs::read_to_string(repo.current())
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_owned();
    let mut text = fs::read_to_string(repo.current()).unwrap();
    for seq in 2..=3 {
        text.push_str(&line.replacen("\"v\":1", "\"v\":2", 1).replacen(
            "\"seq\":1",
            &format!("\"seq\":{seq}"),
            1,
        ));
        text.push('\n');
    }
    fs::write(repo.current(), text).unwrap();
    publish(&repo, 4, 4);

    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    let updates = drain(&mut follower).await;
    assert_eq!(seqs(&updates), [1, 4], "events 2 and 3 are ignored");
    let warnings = warnings(&updates);
    assert_eq!(warnings.len(), 1, "warned once: {warnings:?}");
    assert!(warnings[0].contains("version 2"));
}

// ---------------------------------------------------------------------------------------------
// rotation, pruning, restarts
// ---------------------------------------------------------------------------------------------

fn one_line(repo: &Repo) -> u64 {
    let probe = Repo::new();
    probe.log().publish_blocking(created(1)).unwrap();
    let _ = repo;
    fs::metadata(probe.current()).unwrap().len()
}

#[tokio::test]
async fn start_reads_archives_and_then_the_current_log() {
    let repo = Repo::new();
    let one = one_line(&repo);
    let log = repo.small_log(one * 2, 100);
    for n in 1..=9 {
        log.publish_blocking(created(n)).unwrap();
    }
    assert!(repo.archive_names().len() >= 3);
    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    let updates = drain(&mut follower).await;
    assert_eq!(seqs(&updates), (1..=9).collect::<Vec<u64>>());
    assert_eq!(warnings(&updates), Vec::<&str>::new());
}

#[tokio::test]
async fn after_inside_an_archive_resumes_in_the_middle_of_it() {
    let repo = Repo::new();
    let one = one_line(&repo);
    let log = repo.small_log(one * 3, 100);
    for n in 1..=10 {
        log.publish_blocking(created(n)).unwrap();
    }
    let mut follower = follow(&repo, EventPosition::After(5), polling()).await;
    assert_eq!(
        seqs(&drain(&mut follower).await),
        (6..=10).collect::<Vec<u64>>()
    );
}

#[tokio::test]
async fn rotation_while_following_loses_and_repeats_nothing() {
    let repo = Repo::new();
    let one = one_line(&repo);
    let log = repo.small_log(one * 2, 1000);
    log.publish_blocking(created(1)).unwrap();
    let mut follower = follow(&repo, EventPosition::End, polling()).await;

    // Many events, and several rotations, between two reads.
    for n in 2..=21 {
        log.publish_blocking(created(n)).unwrap();
    }
    assert!(repo.archive_names().len() >= 5);
    assert_eq!(
        seqs(&drain(&mut follower).await),
        (2..=21).collect::<Vec<u64>>()
    );

    // And one rotation per read.
    for n in 22..=30 {
        log.publish_blocking(created(n)).unwrap();
        let got = drain(&mut follower).await;
        assert_eq!(seqs(&got), [n]);
    }
}

#[tokio::test]
async fn a_follower_and_a_busy_publisher_agree_on_every_event_across_rotations() {
    let repo = Repo::new();
    let one = one_line(&repo);
    let root = repo.root().to_path_buf();
    let env = std::sync::Arc::new(functor_testkit::Env::new());
    let mut follower = follow(&repo, EventPosition::End, notified()).await;

    let workers: Vec<_> = (0..4_u64)
        .map(|worker| {
            let root = root.clone();
            let env = std::sync::Arc::clone(&env);
            std::thread::spawn(move || {
                let log = functor_events::FileEventLog::new(
                    &root,
                    functor_model::RepositoryId::from_bits(1),
                    env.clock(),
                    env.ids(),
                    functor_events::EventLogOptions::default()
                        .with_lock_timeout(Duration::from_secs(60))
                        .with_limits(Limits {
                            max_bytes: one * 5,
                            max_archives: 1000,
                        }),
                );
                for n in 0..25 {
                    log.publish_blocking(created(worker * 100 + n)).unwrap();
                }
            })
        })
        .collect();

    let mut got = Vec::new();
    while got.len() < 100 {
        let event = tokio::time::timeout(Duration::from_secs(30), follower.next_event())
            .await
            .expect("events keep arriving")
            .unwrap();
        got.push(event.seq);
    }
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(got, (1..=100).collect::<Vec<u64>>(), "in order, once each");
}

#[tokio::test]
async fn events_pruned_before_the_start_are_reported() {
    let repo = Repo::new();
    let one = one_line(&repo);
    let log = repo.small_log(one, 2);
    for n in 1..=8 {
        log.publish_blocking(created(n)).unwrap();
    }
    let oldest = repo.all_seqs()[0];
    assert!(oldest > 1, "the oldest events were pruned: {oldest}");

    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    let updates = drain(&mut follower).await;
    assert_eq!(
        warnings(&updates),
        [format!("events before seq {oldest} were pruned")]
    );
    assert_eq!(seqs(&updates), (oldest..=8).collect::<Vec<u64>>());
    assert!(
        matches!(updates[0], Update::Warning(_)),
        "the warning comes before the events"
    );

    // `--after` inside the pruned range warns too; at the edge it does not.
    let mut inside = follow(&repo, EventPosition::After(1), polling()).await;
    assert_eq!(warnings(&drain(&mut inside).await).len(), 1);
    let mut edge = follow(&repo, EventPosition::After(oldest - 1), polling()).await;
    let updates = drain(&mut edge).await;
    assert_eq!(warnings(&updates), Vec::<&str>::new());
    assert_eq!(seqs(&updates), (oldest..=8).collect::<Vec<u64>>());
}

#[tokio::test]
async fn an_unpruned_log_starting_at_1_never_warns() {
    let repo = Repo::new();
    publish(&repo, 1, 3);
    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    assert_eq!(warnings(&drain(&mut follower).await), Vec::<&str>::new());
}

#[tokio::test]
async fn a_deleted_log_whose_numbering_restarts_is_followed_again() {
    let repo = Repo::new();
    publish(&repo, 1, 3);
    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    assert_eq!(seqs(&drain(&mut follower).await), [1, 2, 3]);

    fs::remove_file(repo.current()).unwrap();
    assert_eq!(seqs(&drain(&mut follower).await), Vec::<u64>::new());
    publish(&repo, 10, 11); // numbering restarts at 1
    assert_eq!(
        seqs(&drain(&mut follower).await),
        [1, 2],
        "events after a reset are not mistaken for old ones"
    );
}

#[tokio::test]
async fn a_truncated_log_is_followed_from_its_new_start() {
    let repo = Repo::new();
    publish(&repo, 1, 4);
    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    assert_eq!(seqs(&drain(&mut follower).await), [1, 2, 3, 4]);

    fs::write(repo.current(), "").unwrap();
    assert_eq!(seqs(&drain(&mut follower).await), Vec::<u64>::new());
    publish(&repo, 20, 20);
    assert_eq!(seqs(&drain(&mut follower).await), [1]);
}

#[tokio::test]
async fn a_rotation_is_not_mistaken_for_a_reset() {
    let repo = Repo::new();
    let one = one_line(&repo);
    let log = repo.small_log(one, 100);
    log.publish_blocking(created(1)).unwrap();
    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    assert_eq!(seqs(&drain(&mut follower).await), [1]);
    for n in 2..=4 {
        log.publish_blocking(created(n)).unwrap();
    }
    assert_eq!(seqs(&drain(&mut follower).await), [2, 3, 4]);
    assert!(repo.archive_dir().join(archive_name(1, 1)).is_file());
}

// ---------------------------------------------------------------------------------------------
// errors
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn an_unreadable_log_is_a_storage_error_and_the_follower_recovers() {
    let repo = Repo::new();
    publish(&repo, 1, 1);
    let mut follower = follow(&repo, EventPosition::Start, polling()).await;
    assert_eq!(seqs(&drain(&mut follower).await), [1]);

    fs::remove_file(repo.current()).unwrap();
    fs::create_dir(repo.current()).unwrap();
    let error = follower.next_within(Some(IDLE)).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Storage);

    fs::remove_dir(repo.current()).unwrap();
    publish(&repo, 2, 2);
    assert_eq!(
        seqs(&drain(&mut follower).await),
        [1],
        "numbering restarted, and is followed"
    );
}
