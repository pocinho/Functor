//! Shared helpers for the end-to-end tests: a sandbox repository, deterministic invocations and
//! JSON Schema validation.

// Test support code: panicking on unexpected errors is the point, and not every test file uses
// every helper.
#![allow(clippy::unwrap_used, clippy::expect_used, dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// The instant commands run at unless a test says otherwise.
pub const T0: &str = "2026-10-12T18:00:00Z";

/// What a command printed.
#[derive(Debug)]
pub struct Out {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Out {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {:?}", self.stdout))
    }

    pub fn error_json(&self) -> Value {
        serde_json::from_str(&self.stderr)
            .unwrap_or_else(|error| panic!("stderr is not JSON ({error}): {:?}", self.stderr))
    }

    /// Asserts success and returns self.
    pub fn ok(self) -> Self {
        assert_eq!(
            self.code, 0,
            "expected success\nstdout: {}\nstderr: {}",
            self.stdout, self.stderr
        );
        self
    }

    /// Asserts the exit code and that nothing was written to stdout.
    pub fn fails(self, code: i32) -> Self {
        assert_eq!(
            self.code, code,
            "unexpected exit code\nstdout: {}\nstderr: {}",
            self.stdout, self.stderr
        );
        assert_eq!(self.stdout, "", "a failing command must not print results");
        self
    }
}

/// A fresh repository in a temporary directory.
pub struct Sandbox {
    pub dir: TempDir,
}

fn base_command(dir: &Path, time: &str, seed: u64) -> Command {
    let mut command = Command::cargo_bin("functor").expect("binary builds");
    command
        .current_dir(dir)
        .env("FUNCTOR_TEST_FIXED_TIME", time)
        .env("FUNCTOR_TEST_ID_SEED", seed.to_string())
        .env_remove("FUNCTOR_REPO")
        .env_remove("FUNCTOR_LOCK_TIMEOUT")
        .env("NO_COLOR", "1");
    command
}

fn collect(output: std::process::Output) -> Out {
    Out {
        code: output.status.code().expect("exit code"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

/// Runs the binary in `dir` at a fixed time with ids counting from `seed`.
pub fn run_in(dir: &Path, args: &[&str], time: &str, seed: u64) -> Out {
    collect(
        base_command(dir, time, seed)
            .args(args)
            .output()
            .expect("binary runs"),
    )
}

impl Sandbox {
    /// Creates an empty directory and initializes a repository in it (repository id seed 1).
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        run_in(dir.path(), &["init", "--name", "Sandbox"], T0, 1).ok();
        Self { dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Runs a command at [`T0`] with ids counting from `seed`.
    pub fn run(&self, args: &[&str], seed: u64) -> Out {
        run_in(self.path(), args, T0, seed)
    }

    /// Runs a command at a chosen time.
    pub fn run_at(&self, args: &[&str], time: &str, seed: u64) -> Out {
        run_in(self.path(), args, time, seed)
    }

    pub fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }

    pub fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path().join(relative)).unwrap()
    }
}

fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../docs/schemas")
}

pub fn load_schema(name: &str) -> Value {
    let path = schemas_dir().join(format!("{name}.json"));
    serde_json::from_str(
        &fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    )
    .unwrap()
}

/// Asserts that `instance` conforms to the schema `docs/schemas/<name>.json`.
pub fn assert_schema(name: &str, instance: &Value) {
    let schema = load_schema(name);
    let validator = jsonschema::validator_for(&schema).expect("the schema is valid");
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|error| format!("{error} (at {})", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "{name}: {}\n{instance:#}",
        errors.join("\n")
    );
}

/// Asserts that `instance` does **not** conform (used to prove the schemas can fail).
pub fn assert_schema_rejects(name: &str, instance: &Value) {
    let schema = load_schema(name);
    let validator = jsonschema::validator_for(&schema).expect("the schema is valid");
    assert!(
        !validator.is_valid(instance),
        "{name} should reject {instance}"
    );
}
