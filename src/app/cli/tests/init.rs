//! End-to-end tests of `functor init`: arguments, output, exit codes and files on disk.

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

fn functor() -> Command {
    Command::cargo_bin("functor").expect("binary builds")
}

fn names(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    found
}

fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

#[test]
fn init_creates_a_repository_in_the_current_directory() {
    let dir = tempfile::tempdir().unwrap();
    functor()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("initialized repository \""))
        .stdout(predicate::str::contains("(repo_"))
        .stderr("");

    assert_eq!(
        names(dir.path()),
        [".functor", "artifacts", "events", "relationships"]
    );
    let toml = fs::read_to_string(dir.path().join(".functor/repository.toml")).unwrap();
    assert!(toml.contains("format_version = 1"));
    assert!(toml.contains("id = \"repo_"));
}

#[test]
fn init_takes_a_path_and_creates_missing_directories() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("deep").join("demo");
    functor()
        .arg("init")
        .arg(&target)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"demo\""));
    assert!(target.join(".functor/repository.toml").is_file());
}

#[test]
fn the_name_and_description_options_are_stored() {
    let dir = tempfile::tempdir().unwrap();
    functor()
        .current_dir(dir.path())
        .args([
            "init",
            "--name",
            "Functor Core",
            "--description",
            "Core RFC set",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"Functor Core\""));
    let toml = fs::read_to_string(dir.path().join(".functor/repository.toml")).unwrap();
    assert!(toml.contains("name = \"Functor Core\""));
    assert!(toml.contains("description = \"Core RFC set\""));
}

#[test]
fn repo_names_the_target_when_no_path_is_given_and_the_path_wins_otherwise() {
    let dir = tempfile::tempdir().unwrap();
    let via_repo = dir.path().join("via-repo");
    functor()
        .args(["--repo"])
        .arg(&via_repo)
        .arg("init")
        .assert()
        .success();
    assert!(via_repo.join(".functor").is_dir());

    let via_env = dir.path().join("via-env");
    functor()
        .env("FUNCTOR_REPO", &via_env)
        .arg("init")
        .assert()
        .success();
    assert!(via_env.join(".functor").is_dir());

    let via_path = dir.path().join("via-path");
    let ignored = dir.path().join("ignored");
    functor()
        .arg("--repo")
        .arg(&ignored)
        .arg("init")
        .arg(&via_path)
        .assert()
        .success();
    assert!(via_path.join(".functor").is_dir());
    assert!(!ignored.exists());
}

#[test]
fn json_output_follows_the_documented_schema() {
    let dir = tempfile::tempdir().unwrap();
    let assert = functor()
        .current_dir(dir.path())
        .args(["--output", "json", "init", "--name", "Demo"])
        .assert()
        .success()
        .stderr("");
    let document = json(&assert.get_output().stdout);

    assert_eq!(document["schema_version"], 1);
    let repository = &document["repository"];
    assert!(repository["id"].as_str().unwrap().starts_with("repo_"));
    assert_eq!(repository["name"], "Demo");
    assert_eq!(repository["format_version"], 1);
    let root = repository["root"].as_str().unwrap();
    assert!(Path::new(root).join(".functor/repository.toml").is_file());

    // The output is written in the documented key order (parsed JSON maps are sorted, so check
    // the raw text).
    let raw = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let positions: Vec<usize> = [
        "\"schema_version\"",
        "\"id\"",
        "\"name\"",
        "\"root\"",
        "\"format_version\"",
    ]
    .iter()
    .map(|key| raw.find(key).unwrap())
    .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{raw}");
}

#[test]
fn init_twice_is_a_conflict_with_exit_code_4_and_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    functor()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();
    let before = fs::read(dir.path().join(".functor/repository.toml")).unwrap();

    functor()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .code(4)
        .stdout("")
        .stderr(predicate::str::starts_with("error: "))
        .stderr(predicate::str::contains("already contains"));
    assert_eq!(
        fs::read(dir.path().join(".functor/repository.toml")).unwrap(),
        before
    );
}

#[test]
fn json_errors_go_to_stderr_and_stdout_stays_empty() {
    let dir = tempfile::tempdir().unwrap();
    functor()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();

    let assert = functor()
        .current_dir(dir.path())
        .args(["-o", "json", "init"])
        .assert()
        .code(4)
        .stdout("");
    let error = json(&assert.get_output().stderr);
    assert_eq!(error["error"]["code"], "conflict");
    assert_eq!(error["exit_code"], 4);
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("already contains")
    );
}

#[test]
fn nested_repositories_need_the_nested_option() {
    let dir = tempfile::tempdir().unwrap();
    functor()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();
    let inner = dir.path().join("inner");

    functor()
        .arg("init")
        .arg(&inner)
        .assert()
        .code(4)
        .stderr(predicate::str::contains("--nested"));
    assert!(!inner.exists());

    functor()
        .arg("init")
        .arg(&inner)
        .arg("--nested")
        .assert()
        .success();
    assert!(inner.join(".functor/repository.toml").is_file());
}

#[test]
fn an_invalid_name_is_an_input_error_with_exit_code_2() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("x");
    functor()
        .arg("init")
        .arg(&target)
        .args(["--name", "   "])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("error: invalid repository name"));
    assert!(!target.exists());
}

#[test]
fn usage_errors_exit_with_code_2() {
    functor().assert().code(2);
    functor().arg("nope").assert().code(2);
    functor().args(["init", "--bogus"]).assert().code(2);
    functor().args(["init", "-o", "xml"]).assert().code(2);
}

#[test]
fn help_lists_the_init_command() {
    functor()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("--output"));
    functor()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--nested"));
}

#[test]
fn a_new_repository_is_not_a_conflict_for_a_sibling_directory() {
    let dir = tempfile::tempdir().unwrap();
    functor()
        .arg("init")
        .arg(dir.path().join("a"))
        .assert()
        .success();
    functor()
        .arg("init")
        .arg(dir.path().join("b"))
        .assert()
        .success();
}

/// Release builds (default features) must not honor the test-only determinism variables. Run
/// with `cargo test -p functor-cli` (CI does); it is compiled out under `--all-features`.
#[cfg(not(feature = "test-support"))]
#[test]
fn test_only_variables_are_ignored_without_the_test_support_feature() {
    let dir = tempfile::tempdir().unwrap();
    functor()
        .current_dir(dir.path())
        .env("FUNCTOR_TEST_FIXED_TIME", "2001-01-01T00:00:00Z")
        .env("FUNCTOR_TEST_ID_SEED", "1")
        .arg("init")
        .assert()
        .success();
    let toml = fs::read_to_string(dir.path().join(".functor/repository.toml")).unwrap();
    assert!(
        !toml.contains("2001-01-01"),
        "the clock must be the real one: {toml}"
    );
    assert!(
        !toml.contains("repo_00000000000000000000000001"),
        "ids must be real ULIDs: {toml}"
    );
}
