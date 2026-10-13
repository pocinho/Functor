//! End-to-end tests of `functor watch`: existing and new events, start positions, filters,
//! limits, damaged logs and the guarantee that watching never writes.
//!
//! These tests need deterministic time and ids, so they run only with the `test-support` feature
//! (`cargo test --all-features`, as CI does).

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use common::{Out, Sandbox, T0, finish, run_in};
use functor_events::{archive_name, encode_event};
use functor_model::{
    ArtifactId, Event, EventData, EventId, EventKind, EventSubject, NewEvent, RepositoryId,
    Timestamp,
};
use insta::assert_snapshot;

fn transcript(out: &Out) -> String {
    format!(
        "exit: {}\n--- stdout ---\n{}--- stderr ---\n{}",
        out.code, out.stdout, out.stderr
    )
}

fn lines(out: &Out) -> Vec<&str> {
    out.stdout.lines().collect()
}

/// Five events: create `rfc-0000` (1), create `rfc-0001` (2), `rfc-0001 depends_on rfc-0000` (3),
/// retitle `rfc-0000` (4) and validate (5).
fn session() -> Sandbox {
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
        .run(
            &["artifact", "update", "rfc-0000", "--title", "Vision"],
            300,
        )
        .ok();
    sandbox.run(&["validate"], 400).ok();
    sandbox
}

fn watch(sandbox: &Sandbox, args: &[&str]) -> Out {
    let mut full = vec!["watch"];
    full.extend(args);
    sandbox.run(&full, 900)
}

/// Sequence numbers at the start of each text line.
fn seqs(out: &Out) -> Vec<u64> {
    out.stdout
        .lines()
        .map(|line| line.split("  ").next().unwrap().parse().unwrap())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// existing events and their format
// ---------------------------------------------------------------------------------------------

#[test]
fn from_start_prints_every_event_one_line_each() {
    let sandbox = session();
    let out = watch(&sandbox, &["--from-start", "--count", "5"]);
    assert_snapshot!("from_start_text", transcript(&out));
    assert_eq!(out.code, 0);
}

#[test]
fn json_output_is_exactly_the_log() {
    let sandbox = session();
    let out = watch(&sandbox, &["--from-start", "--count", "5", "-o", "json"]);
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert_eq!(out.stdout, sandbox.read("events/current.log"));
    for line in out.stdout.lines() {
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(value["v"], 1);
    }
}

#[test]
fn text_lines_follow_the_documented_columns() {
    let sandbox = session();
    let out = watch(&sandbox, &["--from-start", "--count", "5"]);
    let all = lines(&out);
    assert_eq!(
        all[0],
        "1  2026-10-12T18:00:00Z  artifact.created  rfc-0000"
    );
    assert_eq!(
        all[2],
        "3  2026-10-12T18:00:00Z  relationship.created  rfc-0001 depends_on rfc-0000"
    );
    assert_eq!(
        all[3],
        "4  2026-10-12T18:00:00Z  artifact.updated  rfc-0000"
    );
    assert_eq!(
        all[4],
        "5  2026-10-12T18:00:00Z  validation.completed  passed"
    );
}

#[test]
fn slugs_are_resolved_when_the_line_is_printed() {
    let sandbox = Sandbox::new();
    sandbox.run(&["artifact", "create", "old-name"], 100).ok();
    sandbox
        .run(
            &["artifact", "update", "old-name", "--slug", "new-name"],
            200,
        )
        .ok();
    let out = watch(&sandbox, &["--from-start", "--count", "2"]);
    assert_eq!(
        lines(&out),
        [
            "1  2026-10-12T18:00:00Z  artifact.created  new-name",
            "2  2026-10-12T18:00:00Z  artifact.updated  new-name",
        ]
    );
}

#[test]
fn an_artifact_that_no_longer_resolves_is_printed_by_id() {
    let sandbox = Sandbox::new();
    sandbox.run(&["artifact", "create", "gone"], 100).ok();
    let id = ArtifactId::from_bits(100);
    fs::remove_file(sandbox.path().join(format!("artifacts/{id}.toml"))).unwrap();
    let out = watch(&sandbox, &["--from-start", "--count", "1"]);
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert!(
        out.stdout.trim_end().ends_with(&id.to_string()),
        "{}",
        out.stdout
    );
}

#[test]
fn an_empty_log_prints_nothing_and_a_timeout_ends_the_wait() {
    let sandbox = Sandbox::new();
    let started = Instant::now();
    let out = watch(&sandbox, &["--from-start", "--timeout", "1"]);
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert_eq!(out.stdout, "");
    assert_eq!(out.stderr, "");
    assert!(started.elapsed() >= Duration::from_millis(900));
    assert!(started.elapsed() < Duration::from_secs(20));
}

#[test]
fn quiet_does_not_silence_watch() {
    let sandbox = session();
    let out = sandbox.run(&["-q", "watch", "--from-start", "--count", "1"], 900);
    assert_eq!(out.code, 0);
    assert_eq!(lines(&out).len(), 1);
}

// ---------------------------------------------------------------------------------------------
// start positions, filters and limits
// ---------------------------------------------------------------------------------------------

#[test]
fn after_skips_events_up_to_and_including_the_sequence_number() {
    let sandbox = session();
    let out = watch(&sandbox, &["--after", "3", "--count", "2"]);
    assert_eq!(seqs(&out), [4, 5]);
    let out = watch(&sandbox, &["--after", "5", "--timeout", "1"]);
    assert_eq!(out.stdout, "");
    let out = watch(&sandbox, &["--after", "0", "--count", "5"]);
    assert_eq!(seqs(&out), [1, 2, 3, 4, 5]);
}

#[test]
fn count_stops_after_that_many_events() {
    let sandbox = session();
    assert_eq!(
        seqs(&watch(&sandbox, &["--from-start", "--count", "1"])),
        [1]
    );
    assert_eq!(
        seqs(&watch(&sandbox, &["--from-start", "--count", "3"])),
        [1, 2, 3]
    );
}

#[test]
fn type_filters_match_names_and_prefixes() {
    let sandbox = session();
    let by = |types: &[&str]| {
        let mut args = vec!["--from-start", "--timeout", "1"];
        for t in types {
            args.extend(["--type", t]);
        }
        seqs(&watch(&sandbox, &args))
    };
    assert_eq!(by(&["artifact."]), [1, 2, 4]);
    assert_eq!(by(&["artifact.created"]), [1, 2]);
    assert_eq!(by(&["relationship."]), [3]);
    assert_eq!(by(&["validation"]), [5]);
    assert_eq!(by(&["artifact.updated", "validation.completed"]), [4, 5]);
    assert_eq!(by(&["nothing."]), Vec::<u64>::new());
}

#[test]
fn count_counts_only_events_that_pass_the_filters() {
    let sandbox = session();
    let out = watch(
        &sandbox,
        &["--from-start", "--type", "artifact.", "--count", "2"],
    );
    assert_eq!(seqs(&out), [1, 2]);
    let out = watch(
        &sandbox,
        &["--after", "1", "--type", "artifact.", "--count", "2"],
    );
    assert_eq!(seqs(&out), [2, 4]);
}

#[test]
fn subject_shows_only_events_about_that_artifact() {
    let sandbox = session();
    let by = |subject: &str| {
        seqs(&watch(
            &sandbox,
            &["--from-start", "--subject", subject, "--timeout", "1"],
        ))
    };
    assert_eq!(by("rfc-0000"), [1, 4]);
    assert_eq!(by("rfc-0001"), [2]);
    assert_eq!(by(&ArtifactId::from_bits(100).to_string()), [1, 4]);
    assert_eq!(by(&ArtifactId::from_bits(100).canonical()), [1, 4]);
}

#[test]
fn an_unknown_subject_is_not_found() {
    let sandbox = session();
    let out = watch(&sandbox, &["--subject", "rfc-9999", "--timeout", "1"]);
    assert_eq!(out.code, 3, "{}", transcript(&out));
    assert_eq!(out.stdout, "");
    assert!(
        out.stderr.contains("artifact not found: rfc-9999"),
        "{}",
        out.stderr
    );
}

#[test]
fn conflicting_and_invalid_options_are_usage_errors() {
    let sandbox = session();
    for args in [
        &["--from-start", "--after", "1"][..],
        &["--count", "0"],
        &["--count", "x"],
        &["--after", "-1"],
        &["--timeout", "soon"],
    ] {
        let out = watch(&sandbox, args);
        assert_eq!(out.code, 2, "{args:?}: {}", transcript(&out));
        assert_eq!(out.stdout, "");
    }
}

#[test]
fn watch_outside_a_repository_exits_with_code_5() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_in(dir.path(), &["watch", "--timeout", "1"], T0, 1);
    assert_eq!(out.code, 5);
}

// ---------------------------------------------------------------------------------------------
// following live
// ---------------------------------------------------------------------------------------------

#[test]
fn watch_shows_events_published_after_it_started_by_other_processes() {
    let sandbox = Sandbox::new();
    sandbox.run(&["artifact", "create", "before"], 100).ok();
    let child = sandbox.spawn(&["watch", "--count", "2", "--timeout", "30"], 900);
    // Give the watcher time to start and to read the end of the log.
    thread::sleep(Duration::from_millis(2000));
    sandbox.run(&["artifact", "create", "first-new"], 200).ok();
    sandbox.run(&["artifact", "create", "second-new"], 300).ok();

    let out = finish(child);
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert_eq!(
        seqs(&out),
        [2, 3],
        "the event that existed before is not shown"
    );
    assert!(out.stdout.contains("first-new") && out.stdout.contains("second-new"));
}

#[test]
fn a_log_created_after_the_watch_started_is_followed() {
    let sandbox = Sandbox::new();
    assert!(!sandbox.path().join("events/current.log").exists());
    let child = sandbox.spawn(
        &["watch", "--from-start", "--count", "1", "--timeout", "30"],
        900,
    );
    thread::sleep(Duration::from_millis(1500));
    sandbox.run(&["artifact", "create", "late"], 100).ok();
    let out = finish(child);
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert_eq!(seqs(&out), [1]);
}

#[test]
fn the_idle_timeout_restarts_with_every_event() {
    let sandbox = Sandbox::new();
    let child = sandbox.spawn(&["watch", "--count", "3", "--timeout", "3"], 900);
    thread::sleep(Duration::from_millis(1500));
    for (n, seed) in [(0_u64, 100_u64), (1, 200), (2, 300)] {
        sandbox
            .run(&["artifact", "create", &format!("spaced-{n}")], seed)
            .ok();
        thread::sleep(Duration::from_millis(1800));
    }
    let out = finish(child);
    // Six seconds passed in total, more than the 3 second limit, but never 3 idle seconds.
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert_eq!(seqs(&out), [1, 2, 3]);
}

#[test]
fn a_closed_output_pipe_ends_watch_quietly_with_exit_code_0() {
    let sandbox = Sandbox::new();
    let log = sandbox.path().join("events/current.log");
    write_log(&log, 1, 3000);
    let mut child = sandbox.spawn(&["watch", "--from-start"], 900);
    let stdout = child.stdout.take().unwrap();
    let mut first = String::new();
    BufReader::new(stdout).read_line(&mut first).unwrap();
    assert!(first.starts_with("1  "), "{first:?}");
    // The reader goes away while the command still has lines to print.
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

// ---------------------------------------------------------------------------------------------
// damaged and rotated logs
// ---------------------------------------------------------------------------------------------

/// Writes a valid log with sequence numbers `first..=last`.
fn write_log(path: &Path, first: u64, last: u64) {
    let mut text = String::new();
    for seq in first..=last {
        let event = Event::new(
            EventId::from_bits(u128::from(seq)),
            seq,
            Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
            RepositoryId::from_bits(1),
            NewEvent::new(
                EventKind::ValidationCompleted,
                EventSubject::Repository(RepositoryId::from_bits(1)),
                EventData::new().with("outcome", "passed"),
            ),
        )
        .unwrap();
        text.push_str(&encode_event(&event));
        text.push('\n');
    }
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn a_partial_last_line_is_not_shown_and_not_reported() {
    let sandbox = session();
    let log = sandbox.path().join("events/current.log");
    let mut file = fs::OpenOptions::new().append(true).open(&log).unwrap();
    file.write_all(b"{\"v\":1,\"id\":\"evt_0000").unwrap();
    drop(file);
    let out = watch(&sandbox, &["--from-start", "--timeout", "1"]);
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert_eq!(seqs(&out), [1, 2, 3, 4, 5]);
    assert_eq!(out.stderr, "");
}

#[test]
fn an_unreadable_line_warns_on_stderr_and_the_rest_is_shown() {
    let sandbox = session();
    let log = sandbox.path().join("events/current.log");
    let mut text = fs::read_to_string(&log).unwrap();
    text.push_str("this is not an event\n");
    fs::write(&log, text).unwrap();
    let out = watch(&sandbox, &["--from-start", "--timeout", "1"]);
    assert_eq!(out.code, 0);
    assert_eq!(seqs(&out), [1, 2, 3, 4, 5]);
    assert!(
        out.stderr
            .starts_with("warning: skipped an unreadable line in events/current.log"),
        "{}",
        out.stderr
    );
}

#[test]
fn archives_are_read_before_the_current_log() {
    let sandbox = Sandbox::new();
    write_log(
        &sandbox
            .path()
            .join("events/archive")
            .join(archive_name(1, 3)),
        1,
        3,
    );
    write_log(&sandbox.path().join("events/current.log"), 4, 6);
    let out = watch(&sandbox, &["--from-start", "--timeout", "1"]);
    assert_eq!(seqs(&out), [1, 2, 3, 4, 5, 6]);
    assert_eq!(out.stderr, "");

    let out = watch(&sandbox, &["--after", "2", "--timeout", "1"]);
    assert_eq!(seqs(&out), [3, 4, 5, 6], "resumes inside the archive");
}

#[test]
fn events_that_were_pruned_are_reported() {
    let sandbox = Sandbox::new();
    write_log(&sandbox.path().join("events/current.log"), 5, 7);
    let out = watch(&sandbox, &["--from-start", "--timeout", "1"]);
    assert_eq!(seqs(&out), [5, 6, 7]);
    assert_eq!(out.stderr, "warning: events before seq 5 were pruned\n");

    let out = watch(&sandbox, &["--after", "2", "--timeout", "1"]);
    assert_eq!(out.stderr, "warning: events before seq 5 were pruned\n");
    let out = watch(&sandbox, &["--after", "4", "--timeout", "1"]);
    assert_eq!(out.stderr, "", "nothing is missing after seq 4");
}

#[test]
fn an_unreadable_log_exits_with_code_7() {
    let sandbox = session();
    let log = sandbox.path().join("events/current.log");
    fs::remove_file(&log).unwrap();
    fs::create_dir(&log).unwrap();
    let out = watch(&sandbox, &["--from-start", "--timeout", "1"]);
    assert_eq!(out.code, 7, "{}", transcript(&out));
    assert_eq!(out.stdout, "");
}

// ---------------------------------------------------------------------------------------------
// watch never writes
// ---------------------------------------------------------------------------------------------

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, root: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, files);
            } else {
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(name, fs::read(&path).unwrap());
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

#[test]
fn watch_never_writes_and_never_takes_the_write_lock() {
    let sandbox = Sandbox::new();
    write_log(&sandbox.path().join("events/current.log"), 1, 20);
    let before = tree(sandbox.path());
    assert!(!before.contains_key(".functor/lock"));

    let out = watch(&sandbox, &["--from-start", "--timeout", "1"]);
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert_eq!(seqs(&out).len(), 20);
    assert_eq!(
        tree(sandbox.path()),
        before,
        "no file was created or changed"
    );
}

#[test]
fn watch_works_while_a_writer_holds_the_lock() {
    // Watching must never wait for the repository lock.
    let sandbox = session();
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(sandbox.path().join(".functor/lock"))
        .unwrap();
    lock.lock().unwrap();
    let started = Instant::now();
    let out = watch(&sandbox, &["--from-start", "--count", "5"]);
    assert_eq!(out.code, 0, "{}", transcript(&out));
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "it did not wait for the lock"
    );
    lock.unlock().unwrap();
}
