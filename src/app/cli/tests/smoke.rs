//! End-to-end smoke tests for the `functor` binary.

#![allow(clippy::expect_used)]

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn version_flag_prints_version() {
    Command::cargo_bin("functor")
        .expect("binary builds")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("functor "));
}

#[test]
fn help_flag_succeeds() {
    Command::cargo_bin("functor")
        .expect("binary builds")
        .arg("--help")
        .assert()
        .success();
}
