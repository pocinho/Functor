//! The acceptance test: the MVP success path of `docs/cli/getting-started.md` on a clean directory
//! with fixed time and ids, checked step by step against golden transcripts, run twice to prove
//! it is reproducible, and compared with the committed `examples/getting-started/` repository.
//!
//! To regenerate the example after a deliberate change, run with `FUNCTOR_UPDATE_EXAMPLE=1` and
//! review the diff (`docs/quality/testing-strategy.md`).

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use common::{Out, Sandbox, T0, assert_schema, run_in};
use insta::assert_snapshot;

const T1: &str = "2026-10-12T18:00:05Z";
const T2: &str = "2026-10-12T18:05:00Z";
const T3: &str = "2026-10-12T18:06:00Z";

/// One step of the walkthrough: a name, the arguments, the instant and the id seed.
struct Step {
    name: &'static str,
    args: &'static [&'static str],
    time: &'static str,
    seed: u64,
}

const STEPS: &[Step] = &[
    Step {
        name: "create_rfc_0000",
        args: &[
            "artifact",
            "create",
            "rfc-0000",
            "--type",
            "rfc",
            "--title",
            "Functor Vision",
        ],
        time: T0,
        seed: 100,
    },
    Step {
        name: "create_rfc_0001",
        args: &[
            "artifact",
            "create",
            "rfc-0001",
            "--type",
            "rfc",
            "--title",
            "Core Semantic Model",
        ],
        time: T1,
        seed: 200,
    },
    Step {
        name: "relation_add",
        args: &["relation", "add", "rfc-0001", "depends_on", "rfc-0000"],
        time: T2,
        seed: 300,
    },
    Step {
        name: "query_dependencies",
        args: &["query", "dependencies", "rfc-0001"],
        time: T2,
        seed: 400,
    },
    Step {
        name: "query_dependents",
        args: &["query", "dependents", "rfc-0000"],
        time: T2,
        seed: 410,
    },
    Step {
        name: "query_dependencies_json",
        args: &["query", "dependencies", "rfc-0001", "--output", "json"],
        time: T2,
        seed: 420,
    },
    Step {
        name: "validate",
        args: &["validate"],
        time: T3,
        seed: 500,
    },
    Step {
        name: "watch",
        args: &["watch", "--from-start", "--count", "4", "--timeout", "30"],
        time: T3,
        seed: 600,
    },
];

fn transcript(out: &Out) -> String {
    format!(
        "exit: {}\n--- stdout ---\n{}--- stderr ---\n{}",
        out.code, out.stdout, out.stderr
    )
}

/// Replaces the temporary directory (which differs on every run) in what a command printed.
fn without_path(text: &str, dir: &Path) -> String {
    let mut spellings = vec![dir.display().to_string()];
    if let Ok(canonical) = dir.canonicalize() {
        spellings.push(canonical.display().to_string());
        spellings.push(
            canonical
                .display()
                .to_string()
                .trim_start_matches(r"\\?\")
                .to_owned(),
        );
    }
    let mut result = text.to_owned();
    for spelling in spellings {
        result = result.replace(&spelling, "<repo>");
    }
    result.replace(r"\\", "/")
}

/// A fresh repository named "Functor Demo".
fn demo() -> Sandbox {
    let dir = tempfile::tempdir().unwrap();
    let out = run_in(dir.path(), &["init", "--name", "Functor Demo"], T0, 1).ok();
    assert!(
        out.stdout
            .starts_with("initialized repository \"Functor Demo\"")
    );
    Sandbox { dir }
}

/// Runs the whole walkthrough and returns the transcript of every step, in order.
fn walkthrough(sandbox: &Sandbox) -> Vec<(&'static str, Out)> {
    STEPS
        .iter()
        .map(|step| {
            let out = sandbox.run_at(step.args, step.time, step.seed).ok();
            (step.name, out)
        })
        .collect()
}

/// Every file of a repository by relative path with `/` separators and LF line endings.
fn tree(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, into: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, into);
            } else if path.file_name().is_some_and(|name| name == "lock") {
                // The lock file is runtime state, not repository content.
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let text = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
                into.insert(relative, text);
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

fn example_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../examples/getting-started")
}

#[test]
fn the_walkthrough_matches_its_golden_transcripts() {
    let sandbox = demo();
    for (name, out) in walkthrough(&sandbox) {
        assert_snapshot!(name, without_path(&transcript(&out), sandbox.path()));
    }
}

#[test]
fn the_walkthrough_is_reproducible() {
    let first = demo();
    let second = demo();
    let a = walkthrough(&first);
    let b = walkthrough(&second);
    for ((name, left), (_, right)) in a.iter().zip(&b) {
        assert_eq!(
            without_path(&transcript(left), first.path()),
            without_path(&transcript(right), second.path()),
            "{name}"
        );
    }
    assert_eq!(
        tree(first.path()),
        tree(second.path()),
        "the repositories are byte-for-byte equal"
    );
}

#[test]
fn the_walkthrough_json_documents_match_their_schemas() {
    let sandbox = demo();
    for (name, out) in walkthrough(&sandbox) {
        if name == "query_dependencies_json" {
            assert_schema("query-traversal", &out.json());
        }
    }
}

#[test]
fn the_getting_started_example_is_what_the_walkthrough_produces() {
    let sandbox = demo();
    walkthrough(&sandbox);
    let produced = tree(sandbox.path());

    let dir = example_dir();
    if std::env::var_os("FUNCTOR_UPDATE_EXAMPLE").is_some() {
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        for (relative, text) in &produced {
            let path = dir.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
    }
    assert_eq!(
        tree(&dir),
        produced,
        "examples/getting-started is out of date; regenerate it with FUNCTOR_UPDATE_EXAMPLE=1 \
         cargo test -p functor-cli --all-features --test acceptance and review the diff"
    );
}

#[test]
fn the_example_is_a_healthy_repository_that_the_cli_can_read() {
    // Copy it, so that the test never writes into the working tree.
    let copy = tempfile::tempdir().unwrap();
    for (relative, text) in tree(&example_dir()) {
        let path = copy.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let out = run_in(copy.path(), &["validate", "--output", "json"], T3, 900).ok();
    assert_eq!(out.json()["outcome"], "passed");
    let out = run_in(copy.path(), &["query", "dependencies", "rfc-0001"], T3, 910).ok();
    assert!(out.stdout.contains("rfc-0000"), "{}", out.stdout);
}

#[test]
fn the_negative_variant_fails_validation_with_exit_code_6_and_v0007() {
    let sandbox = demo();
    walkthrough(&sandbox);
    let id = {
        // The first id of seed 100, with the artifact prefix.
        const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        let mut digits = [b'0'; 26];
        let mut rest = 100_u64;
        for digit in digits.iter_mut().rev() {
            *digit = ALPHABET[usize::try_from(rest % 32).unwrap()];
            rest /= 32;
        }
        format!("art_{}", String::from_utf8(digits.to_vec()).unwrap())
    };
    let file = sandbox.path().join("artifacts").join(format!("{id}.toml"));
    assert!(file.exists(), "{}", file.display());
    fs::remove_file(&file).unwrap();

    let out = sandbox.run_at(&["validate", "--output", "json"], T3, 700);
    assert_eq!(out.code, 6, "{}", transcript(&out));
    assert_eq!(out.json()["outcome"], "failed");
    assert!(out.stdout.contains("V0007"), "{}", out.stdout);
    assert_schema("validate", &out.json());
}
