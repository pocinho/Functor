//! End-to-end tests of event emission: one event per mutating command, gap-free numbering across
//! processes, failed appends that only warn, and rotation.
//!
//! These tests need deterministic time and ids, so they run only with the `test-support` feature
//! (`cargo test --all-features`, as CI does).

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::thread;

use common::{Out, Sandbox, T0, run_in};
use functor_events::{archive_name, decode_event, encode_event, peek_seq};
use functor_model::{
    ArtifactId, Event, EventData, EventId, EventKind, EventSubject, MetadataValue, NewEvent,
    RepositoryId, Timestamp,
};
use insta::assert_snapshot;

fn transcript(out: &Out) -> String {
    format!(
        "exit: {}\n--- stdout ---\n{}--- stderr ---\n{}",
        out.code, out.stdout, out.stderr
    )
}

/// The events of `events/current.log`, in order.
fn events(sandbox: &Sandbox) -> Vec<Event> {
    let path = sandbox.path().join("events/current.log");
    match fs::read_to_string(path) {
        Ok(text) => text.lines().map(|l| decode_event(l).unwrap()).collect(),
        Err(_) => Vec::new(),
    }
}

fn text(event: &Event, key: &str) -> String {
    match event.data.get(key) {
        Some(MetadataValue::String(value)) => value.clone(),
        other => panic!("no text field `{key}`: {other:?}"),
    }
}

/// `rfc-0000` (100), `rfc-0001` (101) and `rfc-0001 depends_on rfc-0000` (200).
fn populated() -> Sandbox {
    let sandbox = Sandbox::new();
    sandbox.run(&["artifact", "create", "rfc-0000"], 100).ok();
    sandbox.run(&["artifact", "create", "rfc-0001"], 101).ok();
    sandbox
        .run(
            &["relation", "add", "rfc-0001", "depends_on", "rfc-0000"],
            200,
        )
        .ok();
    sandbox
}

// ---------------------------------------------------------------------------------------------
// one event per mutating command
// ---------------------------------------------------------------------------------------------

#[test]
fn a_new_repository_has_no_events_and_init_emits_none() {
    let sandbox = Sandbox::new();
    assert_eq!(events(&sandbox), []);
    assert!(!sandbox.path().join("events/current.log").exists());
}

#[test]
fn artifact_create_emits_artifact_created() {
    let sandbox = Sandbox::new();
    sandbox
        .run(&["artifact", "create", "rfc-0000", "--type", "rfc"], 100)
        .ok();
    let events = events(&sandbox);
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.kind, EventKind::ArtifactCreated);
    assert_eq!(event.seq, 1);
    assert_eq!(
        event.subject,
        EventSubject::Artifact(ArtifactId::from_bits(100))
    );
    assert_eq!(text(event, "slug"), "rfc-0000");
    assert_eq!(text(event, "type"), "rfc");
    assert_eq!(event.data.get("revision"), Some(&MetadataValue::Integer(1)));
    assert_eq!(event.at, Timestamp::parse(T0).unwrap());
}

#[test]
fn artifact_update_emits_artifact_updated_listing_the_changes() {
    let sandbox = populated();
    sandbox
        .run(
            &[
                "artifact", "update", "rfc-0000", "--title", "New", "--tags", "a,b",
            ],
            300,
        )
        .ok();
    let events = events(&sandbox);
    let event = events.last().unwrap();
    assert_eq!(event.kind, EventKind::ArtifactUpdated);
    assert_eq!(event.data.get("revision"), Some(&MetadataValue::Integer(2)));
    assert_eq!(
        event.data.get("changed"),
        Some(&MetadataValue::StringList(vec![
            "title".to_owned(),
            "tags".to_owned()
        ]))
    );
    assert_eq!(events.len(), 4);
}

#[test]
fn a_no_op_update_emits_nothing() {
    let sandbox = populated();
    let before = events(&sandbox).len();
    sandbox
        .run(
            &["artifact", "update", "rfc-0000", "--title", "rfc-0000"],
            300,
        )
        .ok();
    assert_eq!(events(&sandbox).len(), before);
}

#[test]
fn relation_add_emits_relationship_created() {
    let sandbox = populated();
    let events = events(&sandbox);
    let event = &events[2];
    assert_eq!(event.kind, EventKind::RelationshipCreated);
    assert_eq!(text(event, "relationship_type"), "depends_on");
    assert_eq!(
        text(event, "source"),
        ArtifactId::from_bits(101).to_string()
    );
    assert_eq!(
        text(event, "target"),
        ArtifactId::from_bits(100).to_string()
    );
    assert!(matches!(event.subject, EventSubject::Relationship(_)));
}

#[test]
fn validate_always_emits_validation_completed_even_when_it_fails() {
    let sandbox = populated();
    sandbox.run(&["validate"], 400).ok();
    fs::remove_file(
        sandbox
            .path()
            .join(format!("artifacts/{}.toml", ArtifactId::from_bits(100))),
    )
    .unwrap();
    let failed = sandbox.run(&["validate", "--strict", "-o", "json"], 500);
    assert_eq!(failed.code, 6);

    let events = events(&sandbox);
    assert_eq!(events.len(), 5);
    let (passed, failed) = (&events[3], &events[4]);
    assert_eq!(passed.kind, EventKind::ValidationCompleted);
    assert_eq!(text(passed, "outcome"), "passed");
    assert_eq!(passed.data.get("errors"), Some(&MetadataValue::Integer(0)));
    assert_eq!(passed.data.get("strict"), Some(&MetadataValue::Bool(false)));
    assert_eq!(text(failed, "outcome"), "failed");
    assert_eq!(failed.data.get("errors"), Some(&MetadataValue::Integer(1)));
    assert_eq!(failed.data.get("strict"), Some(&MetadataValue::Bool(true)));
    assert!(matches!(failed.subject, EventSubject::Repository(_)));
}

#[test]
fn read_only_and_failed_commands_emit_nothing() {
    let sandbox = populated();
    let before = events(&sandbox).len();
    let commands: [&[&str]; 14] = [
        &["artifact", "show", "rfc-0000"],
        &["artifact", "list"],
        &["relation", "list"],
        &["query", "artifacts"],
        &["query", "dependencies", "rfc-0001"],
        &["artifact", "create", "rfc-0000"],   // duplicate slug
        &["artifact", "create", "Not A Slug"], // invalid input
        &["artifact", "update", "nope", "--title", "x"], // unknown artifact
        &["artifact", "update", "rfc-0000"],   // no change option
        &["relation", "add", "rfc-0001", "depends_on", "rfc-0000"], // duplicate
        &["relation", "add", "rfc-0000", "depends_on", "rfc-0000"], // self-reference
        &["relation", "add", "rfc-0000", "related_to", "rfc-0001"], // custom type
        &["validate", "nope"],                 // failed to start
        &["relation", "show", "rfc-0000"],
    ];
    for args in commands {
        let _ = sandbox.run(args, 900);
    }
    assert_eq!(events(&sandbox).len(), before);
}

#[test]
fn events_carry_the_repository_as_their_origin() {
    // `Sandbox::new` initializes the repository with id seed 1.
    let sandbox = Sandbox::new();
    sandbox.run(&["artifact", "create", "a"], 100).ok();
    let event = &events(&sandbox)[0];
    assert_eq!(event.origin, RepositoryId::from_bits(1));
    let repository_file = sandbox.read(".functor/repository.toml");
    assert!(
        repository_file.contains(&event.origin.to_string()),
        "the origin is the id in repository.toml: {repository_file}"
    );
}

#[test]
fn the_event_log_after_a_typical_session_is_stable() {
    let sandbox = populated();
    sandbox
        .run(
            &["artifact", "update", "rfc-0000", "--title", "Vision"],
            300,
        )
        .ok();
    sandbox.run(&["validate"], 400).ok();
    assert_snapshot!("session_log", sandbox.read("events/current.log"));
}

#[test]
fn events_do_not_change_what_commands_print() {
    let with_log = Sandbox::new();
    let first = with_log.run(&["artifact", "create", "a"], 100);
    let second = with_log.run(&["artifact", "create", "b"], 101);
    assert_eq!(first.stderr, "");
    assert_eq!(second.stderr, "");
    assert_eq!(
        first.stdout,
        "created artifact a (art_00000000000000000000000034)\n"
    );
}

// ---------------------------------------------------------------------------------------------
// numbering
// ---------------------------------------------------------------------------------------------

#[test]
fn sequence_numbers_count_up_across_commands_and_processes() {
    let sandbox = populated();
    sandbox.run(&["validate"], 400).ok();
    sandbox
        .run(&["artifact", "update", "rfc-0000", "--title", "T"], 500)
        .ok();
    let seqs: Vec<u64> = events(&sandbox).iter().map(|e| e.seq).collect();
    assert_eq!(seqs, [1, 2, 3, 4, 5]);
}

#[test]
fn concurrent_commands_never_duplicate_or_skip_a_sequence_number() {
    let sandbox = Sandbox::new();
    let path = sandbox.path().to_path_buf();
    let workers: Vec<_> = (0..8_u64)
        .map(|index| {
            let path = path.clone();
            thread::spawn(move || {
                let mut codes = Vec::new();
                for n in 0..4_u64 {
                    let slug = format!("item-{index}-{n}");
                    let out = run_in(
                        &path,
                        &["artifact", "create", &slug],
                        T0,
                        1000 + index * 100 + n * 10,
                    );
                    codes.push(out.code);
                }
                codes
            })
        })
        .collect();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), [0, 0, 0, 0]);
    }
    let seqs: Vec<u64> = events(&sandbox).iter().map(|e| e.seq).collect();
    assert_eq!(seqs, (1..=32).collect::<Vec<u64>>());
    let slugs: std::collections::BTreeSet<String> =
        events(&sandbox).iter().map(|e| text(e, "slug")).collect();
    assert_eq!(slugs.len(), 32, "one event per created artifact");
}

// ---------------------------------------------------------------------------------------------
// a failed append only warns
// ---------------------------------------------------------------------------------------------

/// Makes the log unwritable: a directory sits where `current.log` must be.
fn break_the_log(sandbox: &Sandbox) {
    let log = sandbox.path().join("events/current.log");
    let _ = fs::remove_file(&log);
    fs::create_dir_all(&log).unwrap();
}

#[test]
fn a_failed_append_warns_and_the_command_still_succeeds() {
    let sandbox = Sandbox::new();
    break_the_log(&sandbox);
    let out = sandbox.run(&["artifact", "create", "rfc-0000"], 100);
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert_eq!(
        out.stdout,
        "created artifact rfc-0000 (art_00000000000000000000000034)\n"
    );
    assert!(
        out.stderr.starts_with("warning: event not recorded: "),
        "{}",
        out.stderr
    );
    // The state change committed anyway.
    let listed = sandbox.run(&["artifact", "list", "-o", "json"], 200).ok();
    assert_eq!(listed.json()["artifacts"][0]["slug"], "rfc-0000");
}

#[test]
fn a_failed_append_warns_for_every_mutating_command_in_both_modes() {
    let sandbox = populated();
    break_the_log(&sandbox);
    for (args, seed) in [
        (vec!["artifact", "create", "c", "-o", "json"], 300_u64),
        (
            vec!["artifact", "update", "c", "--title", "T", "-o", "json"],
            310,
        ),
        (
            vec![
                "relation",
                "add",
                "c",
                "references",
                "rfc-0000",
                "-o",
                "json",
            ],
            320,
        ),
        (vec!["validate", "-o", "json"], 330),
    ] {
        let out = sandbox.run(&args, seed);
        assert_eq!(out.code, 0, "{args:?}: {}", transcript(&out));
        assert!(
            out.stderr.contains("warning: event not recorded:"),
            "{args:?}: {}",
            out.stderr
        );
        // stdout still holds exactly one JSON document.
        out.json();
    }
}

#[test]
fn a_failed_append_does_not_hide_the_exit_code_of_validate() {
    let sandbox = populated();
    break_the_log(&sandbox);
    fs::remove_file(
        sandbox
            .path()
            .join(format!("artifacts/{}.toml", ArtifactId::from_bits(100))),
    )
    .unwrap();
    let out = sandbox.run(&["validate"], 400);
    assert_eq!(
        out.code,
        6,
        "validation failure, not the warning, decides: {}",
        transcript(&out)
    );
    assert!(out.stderr.contains("warning: event not recorded:"));
}

// ---------------------------------------------------------------------------------------------
// rotation
// ---------------------------------------------------------------------------------------------

/// Writes a valid log of `count` events (sequence 1..=count), each padded to about `size` bytes.
fn write_log(path: &std::path::Path, first: u64, last: u64) -> u64 {
    let mut text = String::new();
    for seq in first..=last {
        let event = Event::new(
            EventId::from_bits(u128::from(seq)),
            seq,
            Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
            RepositoryId::from_bits(1),
            NewEvent::new(
                EventKind::ArtifactCreated,
                EventSubject::Artifact(ArtifactId::from_bits(u128::from(seq))),
                EventData::new().with("slug", "x".repeat(600).as_str()),
            ),
        )
        .unwrap();
        text.push_str(&encode_event(&event));
        text.push('\n');
    }
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, &text).unwrap();
    text.len() as u64
}

#[test]
fn the_log_rotates_at_10_mib_and_numbering_continues() {
    let sandbox = Sandbox::new();
    let current = sandbox.path().join("events/current.log");
    let size = write_log(&current, 1, 16_000);
    assert!(size >= 10 * 1024 * 1024, "{size}");

    let out = sandbox.run(&["artifact", "create", "after-rotation"], 100);
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert_eq!(out.stderr, "", "a clean rotation prints no warning");

    let archive = sandbox
        .path()
        .join("events/archive")
        .join(archive_name(1, 16_000));
    assert!(archive.is_file(), "the old log became {archive:?}");
    assert_eq!(fs::metadata(&archive).unwrap().len(), size);

    let events = events(&sandbox);
    assert_eq!(events.len(), 1, "the new log starts with the new event");
    assert_eq!(events[0].seq, 16_001);
    assert_eq!(text(&events[0], "slug"), "after-rotation");
}

#[test]
fn only_the_ten_newest_archives_survive_a_rotation() {
    let sandbox = Sandbox::new();
    let archive_dir = sandbox.path().join("events/archive");
    for n in 0..10_u64 {
        let (first, last) = (n * 10 + 1, n * 10 + 10);
        write_log(&archive_dir.join(archive_name(first, last)), first, first);
    }
    let current = sandbox.path().join("events/current.log");
    write_log(&current, 101, 16_100);

    sandbox.run(&["artifact", "create", "x"], 100).ok();

    let mut names: Vec<String> = fs::read_dir(&archive_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names.len(), 10, "{names:?}");
    assert!(
        !names.contains(&archive_name(1, 10)),
        "the oldest was pruned"
    );
    assert_eq!(names.last().unwrap(), &archive_name(101, 16_100));
    assert_eq!(
        peek_seq(
            fs::read_to_string(&current)
                .unwrap()
                .lines()
                .next()
                .unwrap()
        ),
        Some(16_101)
    );
}

#[test]
fn the_log_just_below_the_limit_is_not_rotated() {
    let sandbox = Sandbox::new();
    let current = sandbox.path().join("events/current.log");
    let size = write_log(&current, 1, 100);
    assert!(size < 10 * 1024 * 1024);
    sandbox.run(&["artifact", "create", "x"], 100).ok();
    assert!(!sandbox.path().join("events/archive").exists());
    assert_eq!(events(&sandbox).len(), 101);
}

#[test]
fn a_torn_last_line_is_repaired_by_the_next_command() {
    let sandbox = populated();
    let log = sandbox.path().join("events/current.log");
    let mut text = fs::read_to_string(&log).unwrap();
    text.push_str("{\"v\":1,\"id\":\"evt_00");
    fs::write(&log, text).unwrap();

    sandbox
        .run(&["artifact", "create", "after-crash"], 500)
        .ok();
    let lines: Vec<String> = fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(lines.len(), 5, "the fragment is its own line");
    assert_eq!(peek_seq(&lines[4]), Some(4));
    assert!(decode_event(&lines[3]).is_err());
}
