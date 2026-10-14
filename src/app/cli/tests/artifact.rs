//! End-to-end tests of `functor artifact ...`: golden transcripts, JSON schemas, exit codes and
//! cross-process behavior.
//!
//! These tests need deterministic time and ids, so they run only with the `test-support` feature
//! (`cargo test --all-features`, as CI does).

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::thread;

use common::{Out, Sandbox, T0, assert_schema, assert_schema_rejects, load_schema, run_in};
use insta::assert_snapshot;
use serde_json::{Value, json};

fn transcript(out: &Out) -> String {
    format!(
        "exit: {}\n--- stdout ---\n{}--- stderr ---\n{}",
        out.code, out.stdout, out.stderr
    )
}

const RFC_0000: [&str; 17] = [
    "artifact",
    "create",
    "rfc-0000",
    "--type",
    "rfc",
    "--title",
    "Functor Vision",
    "--description",
    "Platform vision and goals",
    "--tag",
    "core",
    "--tag",
    "architecture",
    "--meta",
    "author=Paulo",
    "--meta",
    "status=draft",
];

/// `rfc-0000` with metadata, tags and inline content (ids 100), `rfc-0001` (101), `notes` (102).
fn populated() -> Sandbox {
    let sandbox = Sandbox::new();
    let mut args = RFC_0000.to_vec();
    args.extend(["--content-text", "# Vision", "--content-format", "markdown"]);
    sandbox.run(&args, 100).ok();
    sandbox
        .run(
            &[
                "artifact",
                "create",
                "rfc-0001",
                "--type",
                "rfc",
                "--title",
                "Core Semantic Model",
            ],
            101,
        )
        .ok();
    sandbox.run(&["artifact", "create", "notes"], 102).ok();
    sandbox
}

// ---------------------------------------------------------------------------------------------
// create
// ---------------------------------------------------------------------------------------------

#[test]
fn create_prints_a_confirmation() {
    let sandbox = Sandbox::new();
    let out = sandbox.run(&["artifact", "create", "rfc-0000"], 100);
    assert_snapshot!("create_text", transcript(&out));
    out.ok();
}

#[test]
fn create_with_every_option_in_json() {
    let sandbox = Sandbox::new();
    let mut args = RFC_0000.to_vec();
    args.extend([
        "--content-text",
        "# Vision",
        "--content-format",
        "markdown",
        "-o",
        "json",
    ]);
    let out = sandbox.run(&args, 100).ok();
    assert_snapshot!("create_json", transcript(&out));
    assert_schema("artifact-document", &out.json());
}

#[test]
fn create_stores_a_canonical_file_named_by_id() {
    let sandbox = Sandbox::new();
    sandbox.run(&RFC_0000, 100).ok();
    let id = "art_00000000000000000000000034"; // 100 in Crockford base32
    let text = sandbox.read(&format!("artifacts/{id}.toml"));
    assert_snapshot!("create_file", text);
}

#[test]
fn create_defaults_the_title_to_the_slug_and_the_type_to_document() {
    let sandbox = Sandbox::new();
    let out = sandbox
        .run(&["artifact", "create", "plain", "-o", "json"], 100)
        .ok();
    let artifact = &out.json()["artifact"];
    assert_eq!(artifact["title"], "plain");
    assert_eq!(artifact["type"], "document");
    assert_eq!(artifact["revision"], 1);
    assert_eq!(artifact["created"], T0);
    assert_eq!(artifact["updated"], T0);
}

#[test]
fn create_references_a_file_inside_the_repository_without_copying_it() {
    let sandbox = Sandbox::new();
    sandbox.write("content/rfc-0000.md", "# Vision\n");
    let out = sandbox
        .run(
            &[
                "artifact",
                "create",
                "rfc-0000",
                "--content-path",
                "content/rfc-0000.md",
                "-o",
                "json",
            ],
            100,
        )
        .ok();
    let content = &out.json()["artifact"]["content"];
    assert_eq!(
        content,
        &json!({"format": "markdown", "path": "content/rfc-0000.md"})
    );
    assert_eq!(
        sandbox.read("content/rfc-0000.md"),
        "# Vision\n",
        "the file is not touched"
    );
}

#[test]
fn content_paths_are_resolved_from_the_current_directory() {
    let sandbox = Sandbox::new();
    sandbox.write("docs/notes.txt", "hello");
    let out = common::run_in(
        &sandbox.path().join("docs"),
        &[
            "artifact",
            "create",
            "notes",
            "--content-path",
            "notes.txt",
            "-o",
            "json",
        ],
        T0,
        100,
    )
    .ok();
    assert_eq!(
        out.json()["artifact"]["content"],
        json!({"format": "text", "path": "docs/notes.txt"})
    );
}

#[test]
fn the_content_format_is_guessed_from_the_extension() {
    let sandbox = Sandbox::new();
    for (index, (file, format)) in [
        ("a.md", "markdown"),
        ("b.toml", "toml"),
        ("c.json", "json"),
        ("d.bin", "text"),
    ]
    .into_iter()
    .enumerate()
    {
        sandbox.write(file, "x");
        let slug = file.replace('.', "-");
        let out = sandbox
            .run(
                &[
                    "artifact",
                    "create",
                    &slug,
                    "--content-path",
                    file,
                    "-o",
                    "json",
                ],
                200 + index as u64 * 10,
            )
            .ok();
        assert_eq!(
            out.json()["artifact"]["content"]["format"],
            format,
            "{file}"
        );
    }
}

#[test]
fn create_rejects_a_duplicate_slug_with_exit_code_4() {
    let sandbox = populated();
    let out = sandbox
        .run(&["artifact", "create", "rfc-0000"], 150)
        .fails(4);
    assert_snapshot!("create_duplicate_slug", transcript(&out));
}

#[test]
fn invalid_create_arguments_are_input_errors_with_exit_code_2() {
    let sandbox = Sandbox::new();
    let cases: [(&[&str], &str); 7] = [
        (&["artifact", "create", "Not A Slug"], "invalid slug"),
        (
            &["artifact", "create", "ok", "--type", "Bad Type"],
            "invalid artifact type",
        ),
        (
            &["artifact", "create", "ok", "--tag", "Bad Tag"],
            "invalid tag",
        ),
        (
            &["artifact", "create", "ok", "--meta", "novalue"],
            "KEY=VALUE",
        ),
        (
            &["artifact", "create", "ok", "--meta", "Bad Key=v"],
            "invalid metadata key",
        ),
        (
            &["artifact", "create", "ok", "--title", "   "],
            "invalid title",
        ),
        (
            &["artifact", "create", "ok", "--content-format", "markdown"],
            "requires --content-text",
        ),
    ];
    for (args, expected) in cases {
        let out = sandbox.run(args, 100).fails(2);
        assert!(out.stderr.contains(expected), "{args:?}: {}", out.stderr);
        assert!(
            out.stderr.starts_with("error: "),
            "{args:?}: {}",
            out.stderr
        );
    }
    let listed = sandbox
        .run(&["artifact", "list", "-o", "json"], 0)
        .ok()
        .json();
    assert_eq!(
        listed["artifacts"],
        json!([]),
        "failed creates left nothing behind"
    );
}

#[test]
fn content_paths_must_exist_be_files_and_stay_inside_the_repository() {
    let sandbox = Sandbox::new();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.md"), "x").unwrap();
    let outside_path = outside.path().join("secret.md");
    std::fs::create_dir(sandbox.path().join("adir")).unwrap();

    let cases = [
        ("missing.md".to_owned(), "does not exist"),
        ("adir".to_owned(), "is not a file"),
        (outside_path.display().to_string(), "outside the repository"),
        ("../escape.md".to_owned(), "does not exist"),
    ];
    for (path, expected) in cases {
        let out = sandbox
            .run(&["artifact", "create", "x", "--content-path", &path], 100)
            .fails(2);
        assert!(out.stderr.contains(expected), "{path}: {}", out.stderr);
    }
}

#[test]
fn conflicting_content_options_are_usage_errors() {
    let sandbox = Sandbox::new();
    let out = sandbox.run(
        &[
            "artifact",
            "create",
            "x",
            "--content-text",
            "a",
            "--content-path",
            "b",
        ],
        100,
    );
    out.fails(2);
}

// ---------------------------------------------------------------------------------------------
// show
// ---------------------------------------------------------------------------------------------

#[test]
fn show_text_lists_what_is_present() {
    let sandbox = populated();
    let out = sandbox.run(&["artifact", "show", "rfc-0000"], 0).ok();
    assert_snapshot!("show_text", transcript(&out));
}

#[test]
fn show_text_of_a_minimal_artifact() {
    let sandbox = populated();
    let out = sandbox.run(&["artifact", "show", "notes"], 0).ok();
    assert_snapshot!("show_text_minimal", transcript(&out));
}

#[test]
fn show_json_and_every_way_to_name_an_artifact() {
    let sandbox = populated();
    let by_slug = sandbox
        .run(&["artifact", "show", "rfc-0000", "-o", "json"], 0)
        .ok();
    assert_snapshot!("show_json", transcript(&by_slug));
    assert_schema("artifact-document", &by_slug.json());

    let id = by_slug.json()["artifact"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let canonical = by_slug.json()["artifact"]["ref"]
        .as_str()
        .unwrap()
        .to_owned();
    for reference in [id, canonical] {
        let out = sandbox
            .run(&["artifact", "show", &reference, "-o", "json"], 0)
            .ok();
        assert_eq!(out.stdout, by_slug.stdout, "{reference}");
    }
}

#[test]
fn show_content_prints_inline_and_referenced_text() {
    let sandbox = populated();
    let inline = sandbox
        .run(&["artifact", "show", "rfc-0000", "--content"], 0)
        .ok();
    assert!(
        inline.stdout.ends_with("\n\n# Vision\n"),
        "{}",
        inline.stdout
    );

    sandbox.write("content/notes.md", "# Notes\nline two\n");
    sandbox
        .run(
            &[
                "artifact",
                "update",
                "notes",
                "--content-path",
                "content/notes.md",
            ],
            0,
        )
        .ok();
    let referenced = sandbox
        .run(&["artifact", "show", "notes", "--content"], 0)
        .ok();
    assert!(
        referenced.stdout.ends_with("\n\n# Notes\nline two\n"),
        "{:?}",
        referenced.stdout
    );
    let json = sandbox
        .run(&["artifact", "show", "notes", "--content", "-o", "json"], 0)
        .ok()
        .json();
    assert_eq!(json["artifact"]["content"]["text"], "# Notes\nline two\n");
    assert_eq!(json["artifact"]["content"]["path"], "content/notes.md");
    assert_schema("artifact-document", &json);

    let without = sandbox
        .run(&["artifact", "show", "notes", "-o", "json"], 0)
        .ok()
        .json();
    assert!(
        without["artifact"]["content"].get("text").is_none(),
        "text only with --content"
    );
}

#[test]
fn show_content_of_an_artifact_without_content_prints_just_the_description() {
    let sandbox = populated();
    let plain = sandbox.run(&["artifact", "show", "notes"], 0).ok();
    let with_flag = sandbox
        .run(&["artifact", "show", "notes", "--content"], 0)
        .ok();
    assert_eq!(plain.stdout, with_flag.stdout);
}

#[test]
fn show_neutralizes_control_characters_from_repository_data() {
    let sandbox = Sandbox::new();
    sandbox
        .run(
            &[
                "artifact",
                "create",
                "evil",
                "--description",
                "red \u{1b}[31mtext\u{7}",
            ],
            100,
        )
        .ok();
    let out = sandbox.run(&["artifact", "show", "evil"], 0).ok();
    assert!(
        !out.stdout.contains('\u{1b}') && !out.stdout.contains('\u{7}'),
        "{:?}",
        out.stdout
    );
    assert!(out.stdout.contains("red \\u{1b}[31mtext\\u{7}"));
}

#[test]
fn show_of_an_unknown_slug_suggests_close_ones() {
    let sandbox = populated();
    let out = sandbox.run(&["artifact", "show", "rfc-0002"], 0).fails(3);
    assert_snapshot!("show_not_found", transcript(&out));
}

#[test]
fn show_of_an_unknown_id_or_far_slug_has_no_suggestions() {
    let sandbox = populated();
    let by_id = sandbox
        .run(&["artifact", "show", "art_0000ZZZ"], 0)
        .fails(2); // not a valid id: the ULID part has the wrong length
    assert!(
        by_id.stderr.contains("invalid artifact id"),
        "{}",
        by_id.stderr
    );

    let unknown_id = sandbox
        .run(&["artifact", "show", "art_7ZZZZZZZZZZZZZZZZZZZZZZZZZ"], 0)
        .fails(3);
    assert_snapshot!("show_unknown_id", transcript(&unknown_id));
    let far = sandbox
        .run(&["artifact", "show", "completely-unrelated"], 0)
        .fails(3);
    assert!(!far.stderr.contains("did you mean"));
}

#[test]
fn malformed_references_are_input_errors() {
    let sandbox = populated();
    for reference in ["art_xyz", "functor:artifact:xyz", "Not A Slug", "x--y"] {
        let out = sandbox.run(&["artifact", "show", reference], 0).fails(2);
        assert!(
            out.stderr.starts_with("error: invalid "),
            "{reference}: {}",
            out.stderr
        );
    }
}

// ---------------------------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------------------------

#[test]
fn list_orders_by_slug_with_a_table() {
    let sandbox = populated();
    let out = sandbox.run(&["artifact", "list"], 0).ok();
    assert_snapshot!("list_text", transcript(&out));
}

#[test]
fn list_long_adds_ids_and_update_times() {
    let sandbox = populated();
    let out = sandbox.run(&["artifact", "list", "--long"], 0).ok();
    assert_snapshot!("list_long", transcript(&out));
}

#[test]
fn list_json_follows_the_schema() {
    let sandbox = populated();
    let out = sandbox.run(&["artifact", "list", "-o", "json"], 0).ok();
    assert_snapshot!("list_json", transcript(&out));
    assert_schema("artifact-list", &out.json());
}

#[test]
fn list_of_an_empty_repository() {
    let sandbox = Sandbox::new();
    let text = sandbox.run(&["artifact", "list"], 0).ok();
    assert_eq!(text.stdout, "no artifacts found\n");
    let json = sandbox.run(&["artifact", "list", "-o", "json"], 0).ok();
    assert_eq!(json.json(), json!({"schema_version": 1, "artifacts": []}));
    assert_schema("artifact-list", &json.json());
}

#[test]
fn list_filters_by_type_and_tag() {
    let sandbox = populated();
    let slugs = |args: &[&str]| -> Vec<String> {
        let mut all = vec!["artifact", "list", "-o", "json"];
        all.extend(args);
        sandbox.run(&all, 0).ok().json()["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["slug"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(slugs(&["--type", "rfc"]), ["rfc-0000", "rfc-0001"]);
    assert_eq!(slugs(&["--type", "document"]), ["notes"]);
    assert_eq!(slugs(&["--tag", "core"]), ["rfc-0000"]);
    assert_eq!(slugs(&["--type", "rfc", "--tag", "core"]), ["rfc-0000"]);
    assert_eq!(slugs(&["--type", "dataset"]), Vec::<String>::new());
    let bad = sandbox
        .run(&["artifact", "list", "--tag", "Bad Tag"], 0)
        .fails(2);
    assert!(bad.stderr.contains("invalid tag"));
}

// ---------------------------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------------------------

const T1: &str = "2026-10-14T09:30:00Z";

#[test]
fn update_reports_the_new_revision() {
    let sandbox = populated();
    let out = sandbox
        .run_at(
            &["artifact", "update", "rfc-0000", "--title", "Vision"],
            T1,
            0,
        )
        .ok();
    assert_snapshot!("update_text", transcript(&out));
}

#[test]
fn update_json_lists_the_changed_fields() {
    let sandbox = populated();
    let out = sandbox
        .run_at(
            &[
                "artifact",
                "update",
                "rfc-0000",
                "--title",
                "Vision",
                "--description",
                "New",
                "-o",
                "json",
            ],
            T1,
            0,
        )
        .ok();
    assert_snapshot!("update_json", transcript(&out));
    assert_schema("artifact-document", &out.json());
    assert_eq!(out.json()["changed"], json!(["title", "description"]));
    assert_eq!(out.json()["artifact"]["revision"], 2);
    assert_eq!(out.json()["artifact"]["updated"], T1);
    assert_eq!(out.json()["artifact"]["created"], T0);
}

#[test]
fn an_update_that_changes_nothing_says_so_and_writes_nothing() {
    let sandbox = populated();
    let before = sandbox.read("artifacts/art_00000000000000000000000034.toml");
    let out = sandbox
        .run_at(
            &[
                "artifact",
                "update",
                "rfc-0000",
                "--title",
                "Functor Vision",
            ],
            T1,
            0,
        )
        .ok();
    assert_eq!(out.stdout, "no changes\n");
    let json = sandbox
        .run_at(
            &[
                "artifact",
                "update",
                "rfc-0000",
                "--title",
                "Functor Vision",
                "-o",
                "json",
            ],
            T1,
            0,
        )
        .ok()
        .json();
    assert_eq!(json["changed"], json!([]));
    assert_eq!(json["artifact"]["revision"], 1);
    assert_eq!(
        sandbox.read("artifacts/art_00000000000000000000000034.toml"),
        before
    );
}

#[test]
fn update_renames_the_slug_and_frees_the_old_one() {
    let sandbox = populated();
    sandbox
        .run_at(&["artifact", "update", "notes", "--slug", "scratch"], T1, 0)
        .ok();
    sandbox.run(&["artifact", "show", "scratch"], 0).ok();
    let gone = sandbox.run(&["artifact", "show", "notes"], 0).fails(3);
    assert!(
        gone.stderr.contains("did you mean: scratch") || gone.stderr.contains("artifact not found")
    );
    sandbox.run(&["artifact", "create", "notes"], 300).ok();
}

#[test]
fn renaming_to_a_taken_slug_conflicts() {
    let sandbox = populated();
    let out = sandbox
        .run(&["artifact", "update", "notes", "--slug", "rfc-0000"], 0)
        .fails(4);
    assert!(
        out.stderr.contains("slug `rfc-0000` is already taken"),
        "{}",
        out.stderr
    );
}

#[test]
fn update_replaces_tags_and_merges_metadata() {
    let sandbox = populated();
    let out = sandbox
        .run_at(
            &[
                "artifact",
                "update",
                "rfc-0000",
                "--tags",
                "alpha, beta",
                "--meta",
                "status=final",
                "--meta",
                "owner=me",
                "--unset-meta",
                "author",
                "-o",
                "json",
            ],
            T1,
            0,
        )
        .ok();
    let artifact = &out.json()["artifact"];
    assert_eq!(artifact["tags"], json!(["alpha", "beta"]));
    assert_eq!(
        artifact["metadata"],
        json!({"owner": "me", "status": "final"})
    );
    assert_eq!(out.json()["changed"], json!(["tags", "metadata"]));

    let cleared = sandbox
        .run_at(
            &["artifact", "update", "rfc-0000", "--tags", "", "-o", "json"],
            T1,
            0,
        )
        .ok();
    assert_eq!(cleared.json()["artifact"]["tags"], json!([]));
}

#[test]
fn update_can_clear_the_description_and_the_content() {
    let sandbox = populated();
    let out = sandbox
        .run_at(
            &[
                "artifact",
                "update",
                "rfc-0000",
                "--clear-description",
                "--clear-content",
                "-o",
                "json",
            ],
            T1,
            0,
        )
        .ok();
    let artifact = &out.json()["artifact"];
    assert_eq!(artifact["description"], Value::Null);
    assert_eq!(artifact["content"], Value::Null);
    assert_eq!(out.json()["changed"], json!(["description", "content"]));
}

#[test]
fn update_replaces_content_with_text_or_a_file() {
    let sandbox = populated();
    let text = sandbox
        .run_at(
            &[
                "artifact",
                "update",
                "notes",
                "--content-text",
                "hello",
                "--content-format",
                "text",
                "-o",
                "json",
            ],
            T1,
            0,
        )
        .ok();
    assert_eq!(
        text.json()["artifact"]["content"],
        json!({"format": "text", "text": "hello"})
    );

    sandbox.write("n.md", "x");
    let file = sandbox
        .run_at(
            &[
                "artifact",
                "update",
                "notes",
                "--content-path",
                "n.md",
                "-o",
                "json",
            ],
            T1,
            0,
        )
        .ok();
    assert_eq!(
        file.json()["artifact"]["content"],
        json!({"format": "markdown", "path": "n.md"})
    );
}

#[test]
fn expect_revision_guards_against_stale_updates() {
    let sandbox = populated();
    let stale = sandbox
        .run_at(
            &[
                "artifact",
                "update",
                "rfc-0000",
                "--title",
                "X",
                "--expect-revision",
                "2",
            ],
            T1,
            0,
        )
        .fails(4);
    assert_snapshot!("update_stale_revision", transcript(&stale));
    let ok = sandbox
        .run_at(
            &[
                "artifact",
                "update",
                "rfc-0000",
                "--title",
                "X",
                "--expect-revision",
                "1",
                "-o",
                "json",
            ],
            T1,
            0,
        )
        .ok();
    assert_eq!(ok.json()["artifact"]["revision"], 2);
    let zero = sandbox
        .run(
            &[
                "artifact",
                "update",
                "rfc-0000",
                "--title",
                "Y",
                "--expect-revision",
                "0",
            ],
            0,
        )
        .fails(2);
    assert!(zero.stderr.contains("invalid revision"), "{}", zero.stderr);
}

#[test]
fn update_requires_a_change_option_and_a_known_artifact() {
    let sandbox = populated();
    let none = sandbox.run(&["artifact", "update", "rfc-0000"], 0).fails(2);
    assert_snapshot!("update_nothing_to_do", transcript(&none));
    let missing = sandbox
        .run(&["artifact", "update", "rfc-9999", "--title", "x"], 0)
        .fails(3);
    assert!(missing.stderr.contains("artifact not found: rfc-9999"));
    let bad = sandbox
        .run(&["artifact", "update", "rfc-0000", "--slug", "Bad Slug"], 0)
        .fails(2);
    assert!(bad.stderr.contains("invalid slug"));
    let unset = sandbox
        .run(
            &["artifact", "update", "rfc-0000", "--unset-meta", "Bad Key"],
            0,
        )
        .fails(2);
    assert!(unset.stderr.contains("invalid metadata key"));
}

#[test]
fn update_preserves_comments_in_hand_edited_files() {
    let sandbox = populated();
    let file = "artifacts/art_00000000000000000000000035.toml"; // rfc-0001
    let original = sandbox.read(file);
    sandbox.write(
        file,
        &format!(
            "# reviewed by hand\n{}",
            original.replace("revision = 1", "revision = 1 # first")
        ),
    );
    sandbox
        .run_at(
            &["artifact", "update", "rfc-0001", "--description", "added"],
            T1,
            0,
        )
        .ok();
    let edited = sandbox.read(file);
    assert!(edited.starts_with("# reviewed by hand\n"), "{edited}");
    assert!(edited.contains("revision = 2 # first\n"), "{edited}");
    assert!(edited.contains("description = \"added\""));
}

// ---------------------------------------------------------------------------------------------
// global options, discovery and environment
// ---------------------------------------------------------------------------------------------

#[test]
fn commands_outside_a_repository_exit_with_code_5() {
    let empty = tempfile::tempdir().unwrap();
    let out = run_in(empty.path(), &["artifact", "list"], T0, 1).fails(5);
    assert!(
        out.stderr.starts_with("error: not a functor repository: "),
        "{}",
        out.stderr
    );
    assert!(
        out.stderr
            .ends_with("hint: run `functor init` to create a repository here\n")
    );
}

#[test]
fn the_repository_is_discovered_from_a_subdirectory() {
    let sandbox = populated();
    let deep = sandbox.path().join("a").join("b");
    std::fs::create_dir_all(&deep).unwrap();
    let out = run_in(&deep, &["artifact", "list", "-o", "json"], T0, 0).ok();
    assert_eq!(out.json()["artifacts"].as_array().unwrap().len(), 3);
}

#[test]
fn repo_and_the_environment_variable_name_the_repository() {
    let sandbox = populated();
    let elsewhere = tempfile::tempdir().unwrap();
    let path = sandbox.path().to_str().unwrap();

    let via_flag = run_in(
        elsewhere.path(),
        &["--repo", path, "artifact", "list", "-o", "json"],
        T0,
        0,
    )
    .ok();
    assert_eq!(via_flag.json()["artifacts"].as_array().unwrap().len(), 3);

    let mut command = assert_cmd::Command::cargo_bin("functor").unwrap();
    let output = command
        .current_dir(elsewhere.path())
        .env("FUNCTOR_REPO", path)
        .args(["artifact", "list", "-o", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let not_a_repo = run_in(
        elsewhere.path(),
        &[
            "--repo",
            elsewhere.path().to_str().unwrap(),
            "artifact",
            "list",
        ],
        T0,
        0,
    );
    not_a_repo.fails(5);
}

#[test]
fn quiet_silences_text_confirmations_but_not_results_or_json() {
    let sandbox = Sandbox::new();
    let quiet = sandbox.run(&["-q", "artifact", "create", "a"], 100).ok();
    assert_eq!(quiet.stdout, "");
    let json = sandbox
        .run(&["-q", "-o", "json", "artifact", "create", "b"], 101)
        .ok();
    assert_eq!(json.json()["artifact"]["slug"], "b");
    let list = sandbox.run(&["-q", "artifact", "list"], 0).ok();
    assert!(
        list.stdout.contains("SLUG"),
        "query results are still printed"
    );
    let update = sandbox
        .run(&["-q", "artifact", "update", "a", "--title", "A"], 0)
        .ok();
    assert_eq!(update.stdout, "");
}

#[test]
fn an_invalid_lock_timeout_is_an_input_error() {
    let sandbox = Sandbox::new();
    let mut command = assert_cmd::Command::cargo_bin("functor").unwrap();
    let output = command
        .current_dir(sandbox.path())
        .env("FUNCTOR_LOCK_TIMEOUT", "soon")
        .args(["artifact", "list"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("FUNCTOR_LOCK_TIMEOUT"));
}

#[test]
fn json_errors_are_valid_documents_on_stderr() {
    let sandbox = populated();
    for (args, code, kind) in [
        (
            vec!["artifact", "show", "rfc-0002", "-o", "json"],
            3,
            "not_found",
        ),
        (
            vec!["artifact", "create", "rfc-0000", "-o", "json"],
            4,
            "conflict",
        ),
        (
            vec!["artifact", "create", "Bad Slug", "-o", "json"],
            2,
            "invalid_input",
        ),
    ] {
        let out = sandbox.run(&args, 500).fails(code);
        let error = out.error_json();
        assert_schema("error", &error);
        assert_eq!(error["error"]["code"], kind);
        assert_eq!(error["exit_code"], code);
    }
    let missing = sandbox
        .run(&["artifact", "show", "rfc-0002", "-o", "json"], 0)
        .fails(3)
        .error_json();
    assert_eq!(missing["error"]["entity"], "artifact");
    assert_eq!(missing["error"]["reference"], "rfc-0002");
    assert_eq!(
        missing["error"]["suggestions"],
        json!(["rfc-0000", "rfc-0001"])
    );
}

// ---------------------------------------------------------------------------------------------
// schemas
// ---------------------------------------------------------------------------------------------

#[test]
fn init_output_follows_its_schema() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_in(dir.path(), &["init", "-o", "json"], T0, 1).ok();
    assert_schema("init", &out.json());
}

#[test]
fn the_schemas_can_actually_fail() {
    assert_schema_rejects("artifact-list", &json!({"schema_version": 1}));
    assert_schema_rejects(
        "artifact-list",
        &json!({"schema_version": 2, "artifacts": []}),
    );
    assert_schema_rejects(
        "artifact-list",
        &json!({"schema_version": 1, "artifacts": [{"id": "x"}]}),
    );
    assert_schema_rejects("artifact-document", &json!({"schema_version": 1}));
    assert_schema_rejects(
        "error",
        &json!({"error": {"code": "weird", "message": "m"}, "exit_code": 3}),
    );
    assert_schema_rejects(
        "error",
        &json!({"error": {"code": "conflict", "message": "m"}, "exit_code": 10}),
    );
    assert_schema_rejects(
        "init",
        &json!({"schema_version": 1, "repository": {"id": "repo_x"}}),
    );
}

#[test]
fn the_artifact_definitions_in_both_schemas_are_identical() {
    let document = load_schema("artifact-document");
    let list = load_schema("artifact-list");
    assert_eq!(document["$defs"]["Artifact"], list["$defs"]["Artifact"]);
}

// ---------------------------------------------------------------------------------------------
// concurrency across processes
// ---------------------------------------------------------------------------------------------

#[test]
fn concurrent_processes_never_lose_or_corrupt_artifacts() {
    let sandbox = Sandbox::new();
    let path = sandbox.path().to_path_buf();
    let workers: Vec<_> = (0..6_u64)
        .map(|index| {
            let path = path.clone();
            thread::spawn(move || {
                let slug = format!("item-{index}");
                run_in(&path, &["artifact", "create", &slug], T0, 1000 + index * 10)
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap().ok();
    }
    let list = sandbox
        .run(&["artifact", "list", "-o", "json"], 0)
        .ok()
        .json();
    assert_eq!(list["artifacts"].as_array().unwrap().len(), 6);
}

#[test]
fn racing_processes_creating_one_slug_have_exactly_one_winner() {
    let sandbox = Sandbox::new();
    let path = sandbox.path().to_path_buf();
    let workers: Vec<_> = (0..4_u64)
        .map(|index| {
            let path = path.clone();
            thread::spawn(move || {
                run_in(
                    &path,
                    &["artifact", "create", "contested"],
                    T0,
                    2000 + index * 10,
                )
            })
        })
        .collect();
    let codes: Vec<i32> = workers
        .into_iter()
        .map(|w| w.join().unwrap().code)
        .collect();
    assert_eq!(codes.iter().filter(|c| **c == 0).count(), 1, "{codes:?}");
    assert_eq!(codes.iter().filter(|c| **c == 4).count(), 3, "{codes:?}");
}
