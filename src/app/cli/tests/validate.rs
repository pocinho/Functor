//! End-to-end tests of `functor validate`: golden transcripts, the exit-code policy, schemas,
//! scoping and discovery of damaged repositories.
//!
//! These tests need deterministic time and ids, so they run only with the `test-support` feature
//! (`cargo test --all-features`, as CI does).

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use common::{Out, Sandbox, T0, assert_schema, assert_schema_rejects, load_schema, run_in};
use insta::assert_snapshot;
use serde_json::{Value, json};

fn transcript(out: &Out) -> String {
    format!(
        "exit: {}\n--- stdout ---\n{}--- stderr ---\n{}",
        out.code, out.stdout, out.stderr
    )
}

/// The id an id source seeded with `seed` hands out first, with the given prefix.
fn id(prefix: &str, seed: u64) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut digits = [b'0'; 26];
    let mut rest = seed;
    for digit in digits.iter_mut().rev() {
        *digit = ALPHABET[usize::try_from(rest % 32).unwrap()];
        rest /= 32;
    }
    format!("{prefix}{}", String::from_utf8(digits.to_vec()).unwrap())
}

fn art(seed: u64) -> String {
    id("art_", seed)
}

fn rel(seed: u64) -> String {
    id("rel_", seed)
}

/// `rfc-0001 depends_on rfc-0000`: artifacts 100 and 101, relationship 200.
fn healthy() -> Sandbox {
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

fn validate(sandbox: &Sandbox, args: &[&str]) -> Out {
    let mut full = vec!["validate"];
    full.extend(args);
    sandbox.run(&full, 900)
}

/// Asserts the exit code only: a validation report is a result, so it is printed to stdout even
/// when the run fails.
fn exits(out: Out, code: i32) -> Out {
    assert_eq!(
        out.code, code,
        "unexpected exit code\nstdout: {}\nstderr: {}",
        out.stdout, out.stderr
    );
    out
}

fn delete(sandbox: &Sandbox, relative: &str) {
    fs::remove_file(sandbox.path().join(relative)).unwrap();
}

fn finding_codes(out: &Out) -> Vec<String> {
    out.json()["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap().to_owned())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// outcomes and exit codes
// ---------------------------------------------------------------------------------------------

#[test]
fn a_healthy_repository_passes_with_exit_code_0() {
    let sandbox = healthy();
    let out = validate(&sandbox, &[]);
    assert_snapshot!("passed_text", transcript(&out));
    exits(out, 0);
}

#[test]
fn a_healthy_repository_in_json_matches_the_schema() {
    let sandbox = healthy();
    let out = exits(validate(&sandbox, &["-o", "json"]), 0);
    assert_snapshot!("passed_json", transcript(&out));
    assert_schema("validate", &out.json());
}

#[test]
fn an_empty_repository_passes() {
    let sandbox = Sandbox::new();
    let out = exits(validate(&sandbox, &[]), 0);
    assert_eq!(
        out.stdout,
        "validated 0 artifacts, 0 relationships\noutcome: passed\n"
    );
}

#[test]
fn a_dangling_relationship_fails_with_exit_code_6() {
    let sandbox = healthy();
    delete(&sandbox, &format!("artifacts/{}.toml", art(100)));
    let out = validate(&sandbox, &[]);
    assert_snapshot!("failed_text", transcript(&out));
    exits(out, 6);
}

#[test]
fn a_failed_report_in_json_matches_the_schema_and_keeps_stderr_empty() {
    let sandbox = healthy();
    delete(&sandbox, &format!("artifacts/{}.toml", art(100)));
    let out = exits(validate(&sandbox, &["-o", "json"]), 6);
    assert_snapshot!("failed_json", transcript(&out));
    assert_schema("validate", &out.json());
    assert_eq!(out.stderr, "");
    assert_eq!(out.json()["outcome"], "failed");
}

#[test]
fn warnings_pass_unless_strict() {
    let sandbox = healthy();
    sandbox
        .run(
            &[
                "relation",
                "add",
                "rfc-0000",
                "supports",
                "rfc-0001",
                "--allow-custom-type",
            ],
            201,
        )
        .ok();

    let out = exits(validate(&sandbox, &[]), 0);
    assert_snapshot!("warning_text", transcript(&out));

    let out = exits(validate(&sandbox, &["--strict"]), 6);
    assert!(out.stdout.contains("outcome: warning"), "{}", out.stdout);

    let json = exits(validate(&sandbox, &["--strict", "-o", "json"]), 6);
    assert_eq!(json.json()["strict"], true);
    assert_eq!(json.json()["outcome"], "warning");
    assert_schema("validate", &json.json());

    let json = exits(validate(&sandbox, &["-o", "json"]), 0);
    assert_eq!(json.json()["strict"], false);
}

#[test]
fn info_findings_never_change_the_outcome() {
    let sandbox = healthy();
    let file = format!("artifacts/{}.toml", art(100));
    sandbox.write(&file, &format!("{}future = 1\n", sandbox.read(&file)));
    let out = exits(validate(&sandbox, &["--strict"]), 0);
    assert_snapshot!("info_text", transcript(&out));
    assert!(
        out.stdout
            .contains("outcome: passed (0 errors, 0 warnings, 1 info)")
    );
}

#[test]
fn an_unreadable_directory_gives_the_unknown_outcome_which_fails() {
    let sandbox = healthy();
    // `relationships` is a file, so it cannot be listed; nothing else is wrong.
    fs::remove_dir_all(sandbox.path().join("relationships")).unwrap();
    sandbox.write("relationships", "not a directory");
    let out = validate(&sandbox, &["-o", "json"]);
    let out = exits(out, 6);
    assert_eq!(out.json()["outcome"], "unknown");
    assert_eq!(finding_codes(&out), ["V0019"]);
    assert_schema("validate", &out.json());
    exits(validate(&sandbox, &[]), 6);
}

#[test]
fn several_problems_are_reported_together_in_code_order() {
    let sandbox = healthy();
    sandbox.write("artifacts/junk.toml", "= = =");
    sandbox.write("artifacts/notes.txt", "x");
    delete(&sandbox, &format!("artifacts/{}.toml", art(100)));
    let out = exits(validate(&sandbox, &["-o", "json"]), 6);
    assert_eq!(finding_codes(&out), ["V0001", "V0007", "V0016"]);
    assert_eq!(out.json()["counts"]["errors"], 2);
    assert_eq!(out.json()["counts"]["warnings"], 1);
}

// ---------------------------------------------------------------------------------------------
// what the findings say
// ---------------------------------------------------------------------------------------------

#[test]
fn duplicate_slugs_name_the_other_file_as_related() {
    let sandbox = healthy();
    let first = format!("artifacts/{}.toml", art(100));
    let other = art(500);
    sandbox.write(
        &format!("artifacts/{other}.toml"),
        &sandbox.read(&first).replace(&art(100), &other),
    );
    let out = validate(&sandbox, &[]);
    assert_snapshot!("duplicate_slug_text", transcript(&out));
    let json = exits(validate(&sandbox, &["-o", "json"]), 6);
    assert_schema("validate", &json.json());
    assert_eq!(
        json.json()["findings"][0]["related"],
        json!([{ "kind": "file", "path": first }])
    );
    exits(out, 6);
}

#[test]
fn a_cycle_is_one_finding_listing_its_artifacts() {
    let sandbox = healthy();
    sandbox
        .run(
            &["relation", "add", "rfc-0000", "depends_on", "rfc-0001"],
            201,
        )
        .ok();
    let out = exits(validate(&sandbox, &["-o", "json"]), 6);
    assert_eq!(finding_codes(&out), ["V0011"]);
    let finding = &out.json()["findings"][0];
    assert_eq!(
        finding["subject"],
        json!({"kind": "artifact", "id": art(100)})
    );
    assert_eq!(
        finding["related"],
        json!([
            {"kind": "artifact", "id": art(100)},
            {"kind": "artifact", "id": art(101)}
        ])
    );
    assert_eq!(
        finding["message"],
        "cycle in `depends_on` relationships among rfc-0000, rfc-0001"
    );
    assert_schema("validate", &out.json());
}

#[test]
fn a_malformed_file_message_is_one_line() {
    let sandbox = healthy();
    sandbox.write("artifacts/junk.toml", "this is = = not toml\n");
    let out = exits(validate(&sandbox, &["-o", "json"]), 6);
    let message = out.json()["findings"][0]["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        !message.contains('\n') && !message.ends_with('.'),
        "{message:?}"
    );
    let text = exits(validate(&sandbox, &[]), 6);
    assert_eq!(
        text.stdout
            .lines()
            .filter(|l| l.starts_with("error V0001"))
            .count(),
        1
    );
}

// ---------------------------------------------------------------------------------------------
// scoped validation
// ---------------------------------------------------------------------------------------------

#[test]
fn scoping_reports_only_findings_about_the_artifact() {
    let sandbox = healthy();
    sandbox.run(&["artifact", "create", "rfc-0002"], 102).ok();
    sandbox
        .run(
            &[
                "relation",
                "add",
                "rfc-0002",
                "supports",
                "rfc-0001",
                "--allow-custom-type",
            ],
            201,
        )
        .ok();
    sandbox.write("artifacts/notes.txt", "x");

    let whole = exits(validate(&sandbox, &["-o", "json"]), 0);
    assert_eq!(finding_codes(&whole), ["V0012", "V0016"]);

    let scoped = |reference: &str| exits(validate(&sandbox, &[reference, "-o", "json"]), 0);
    assert_eq!(finding_codes(&scoped("rfc-0002")), ["V0012"]);
    assert_eq!(
        finding_codes(&scoped("rfc-0001")),
        ["V0012"],
        "it touches the relationship"
    );
    assert_eq!(finding_codes(&scoped("rfc-0000")), Vec::<String>::new());
    assert_eq!(scoped("rfc-0000").json()["outcome"], "passed");
    // Counts still describe the whole repository.
    assert_eq!(scoped("rfc-0000").json()["counts"]["artifacts"], 3);
    assert_eq!(scoped("rfc-0000").json()["counts"]["relationships"], 2);
}

#[test]
fn a_scoped_text_report_says_what_it_is_about() {
    let sandbox = healthy();
    let out = exits(validate(&sandbox, &["rfc-0001"]), 0);
    assert_snapshot!("scoped_text", transcript(&out));
}

#[test]
fn scoped_runs_accept_slugs_ids_and_canonical_references() {
    let sandbox = healthy();
    let by_slug = exits(validate(&sandbox, &["rfc-0001", "-o", "json"]), 0);
    let by_id = exits(validate(&sandbox, &[&art(101), "-o", "json"]), 0);
    let canonical = format!("functor:artifact:{}", art(101));
    let by_ref = exits(validate(&sandbox, &[&canonical, "-o", "json"]), 0);
    assert_eq!(by_slug.stdout, by_id.stdout);
    assert_eq!(by_slug.stdout, by_ref.stdout);
}

#[test]
fn scoping_to_an_unknown_artifact_is_not_found_with_suggestions() {
    let sandbox = healthy();
    let out = validate(&sandbox, &["rfc-0003"]);
    assert_snapshot!("scoped_unknown", transcript(&out));
    assert_eq!(out.code, 3);
    assert_eq!(out.stdout, "");
    exits(validate(&sandbox, &[&art(999)]), 3);
    exits(validate(&sandbox, &["Not A Slug"]), 2);
}

#[test]
fn scoping_still_includes_repository_level_findings() {
    let sandbox = healthy();
    sandbox.write(".functor/repository.toml", "format_version = 9\n");
    let out = exits(validate(&sandbox, &["rfc-0001", "-o", "json"]), 6);
    assert_eq!(finding_codes(&out), ["V0013"]);
}

// ---------------------------------------------------------------------------------------------
// damaged repositories are validated, not refused
// ---------------------------------------------------------------------------------------------

#[test]
fn an_unsupported_format_version_is_a_finding_for_validate_but_an_error_elsewhere() {
    let sandbox = healthy();
    sandbox.write(".functor/repository.toml", "format_version = 9\n");

    let out = validate(&sandbox, &[]);
    assert_snapshot!("unsupported_format_text", transcript(&out));
    exits(out, 6);

    // Every other command still refuses with exit code 5.
    sandbox.run(&["artifact", "list"], 900).fails(5);
    sandbox.run(&["query", "artifacts"], 900).fails(5);
}

#[test]
fn a_malformed_repository_file_is_a_finding_for_validate() {
    let sandbox = healthy();
    sandbox.write(".functor/repository.toml", "= = =");
    let out = exits(validate(&sandbox, &["-o", "json"]), 6);
    assert_eq!(finding_codes(&out), ["V0001"]);
    assert_eq!(
        out.json()["findings"][0]["subject"],
        json!({"kind": "file", "path": ".functor/repository.toml"})
    );
    assert_eq!(
        out.json()["counts"]["artifacts"],
        2,
        "entities are still checked"
    );
}

#[test]
fn missing_entity_directories_are_structure_findings() {
    let sandbox = healthy();
    fs::remove_dir_all(sandbox.path().join("relationships")).unwrap();
    let out = exits(validate(&sandbox, &["-o", "json"]), 6);
    assert_eq!(finding_codes(&out), ["V0014"]);
}

#[test]
fn validate_finds_the_repository_from_a_subdirectory_and_with_repo() {
    let sandbox = healthy();
    fs::create_dir_all(sandbox.path().join("docs/deep")).unwrap();
    let nested = sandbox.path().join("docs/deep");
    let out = run_in(&nested, &["validate"], T0, 900);
    exits(out, 0);

    let elsewhere = tempfile::tempdir().unwrap();
    let repo = sandbox.path().to_str().unwrap();
    exits(
        run_in(elsewhere.path(), &["--repo", repo, "validate"], T0, 900),
        0,
    );
}

#[test]
fn validate_outside_a_repository_exits_with_code_5() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_in(dir.path(), &["validate"], T0, 1).fails(5);
    assert!(out.stderr.contains("functor init"), "{}", out.stderr);
}

// ---------------------------------------------------------------------------------------------
// guarantees
// ---------------------------------------------------------------------------------------------

/// Every file of the repository except the event log, which `validate` appends to by design.
fn repository_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
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
                if !name.starts_with("events/") {
                    files.insert(name, fs::read(&path).unwrap());
                }
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

#[test]
fn validation_never_changes_the_repository_state() {
    let sandbox = healthy();
    sandbox.write("artifacts/junk.toml", "= = =");
    sandbox.write("artifacts/notes.txt", "x");
    let before = repository_files(sandbox.path());
    for args in [
        &[][..],
        &["rfc-0001"],
        &["--strict", "-o", "json"],
        &["nope"],
    ] {
        let _ = validate(&sandbox, args);
    }
    assert_eq!(repository_files(sandbox.path()), before);
}

#[test]
fn the_same_repository_always_produces_the_same_bytes() {
    let sandbox = healthy();
    sandbox.write("artifacts/junk.toml", "= = =");
    sandbox.write("artifacts/notes.txt", "x");
    delete(&sandbox, &format!("artifacts/{}.toml", art(100)));
    let first = validate(&sandbox, &["-o", "json"]);
    // A different clock and id source must not matter either.
    let second = sandbox.run_at(&["validate", "-o", "json"], "2030-01-01T00:00:00Z", 12345);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stdout, validate(&sandbox, &["-o", "json"]).stdout);
}

#[test]
fn quiet_does_not_silence_the_report() {
    let sandbox = healthy();
    let out = sandbox.run(&["-q", "validate"], 900);
    assert!(out.stdout.contains("outcome: passed"), "{}", out.stdout);
    assert_eq!(out.code, 0);
}

#[test]
fn the_exit_code_policy_holds_for_every_outcome() {
    // (setup, strict code, lenient code)
    let healthy_repo = healthy();
    exits(validate(&healthy_repo, &["--strict"]), 0);

    let warning = healthy();
    warning
        .run(
            &[
                "relation",
                "add",
                "rfc-0000",
                "supports",
                "rfc-0001",
                "--allow-custom-type",
            ],
            201,
        )
        .ok();
    exits(validate(&warning, &[]), 0);
    exits(validate(&warning, &["--strict"]), 6);

    let failed = healthy();
    delete(&failed, &format!("artifacts/{}.toml", art(100)));
    exits(validate(&failed, &[]), 6);
    exits(validate(&failed, &["--strict"]), 6);
}

// ---------------------------------------------------------------------------------------------
// the schema
// ---------------------------------------------------------------------------------------------

#[test]
fn the_schema_can_fail() {
    let sandbox = healthy();
    delete(&sandbox, &format!("artifacts/{}.toml", art(100)));
    let document = exits(validate(&sandbox, &["-o", "json"]), 6).json();

    let mut wrong_outcome = document.clone();
    wrong_outcome["outcome"] = json!("great");
    assert_schema_rejects("validate", &wrong_outcome);

    let mut missing_counts = document.clone();
    missing_counts.as_object_mut().unwrap().remove("counts");
    assert_schema_rejects("validate", &missing_counts);

    let mut bad_code = document.clone();
    bad_code["findings"][0]["code"] = json!("X1");
    assert_schema_rejects("validate", &bad_code);

    let mut bad_severity = document.clone();
    bad_severity["findings"][0]["severity"] = json!("fatal");
    assert_schema_rejects("validate", &bad_severity);

    let mut bad_subject = document.clone();
    bad_subject["findings"][0]["subject"] = json!({"kind": "artifact", "path": "x"});
    assert_schema_rejects("validate", &bad_subject);

    let mut extra = document.clone();
    extra["timestamp"] = json!("2026-10-12T18:00:00Z");
    assert_schema_rejects("validate", &extra);

    let mut wrong_id = document.clone();
    wrong_id["findings"][0]["subject"] = json!({"kind": "artifact", "id": rel(1)});
    assert_schema_rejects("validate", &wrong_id);
}

#[test]
fn the_schema_accepts_every_subject_kind() {
    let schema = load_schema("validate");
    assert_eq!(
        schema["$defs"]["Subject"]["oneOf"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let sample = |subject: Value| {
        json!({
            "schema_version": 1, "outcome": "failed", "strict": false,
            "counts": {"artifacts": 0, "relationships": 0, "errors": 1, "warnings": 0, "infos": 0},
            "findings": [{
                "code": "V0001", "severity": "error", "subject": subject,
                "message": "x", "related": []
            }]
        })
    };
    for subject in [
        json!({"kind": "repository"}),
        json!({"kind": "artifact", "id": art(1)}),
        json!({"kind": "relationship", "id": rel(1)}),
        json!({"kind": "file", "path": "artifacts/x.toml"}),
    ] {
        assert_schema("validate", &sample(subject));
    }
}
