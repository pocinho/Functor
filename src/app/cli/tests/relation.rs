//! End-to-end tests of `functor relation ...`: golden transcripts, JSON schemas, exit codes and
//! cross-process behavior.
//!
//! These tests need deterministic time and ids, so they run only with the `test-support` feature
//! (`cargo test --all-features`, as CI does).

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
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

/// `rfc-0000` (id seed 100), `rfc-0001` (101) and `notes` (102).
fn populated() -> Sandbox {
    let sandbox = Sandbox::new();
    for (seed, slug) in [(100, "rfc-0000"), (101, "rfc-0001"), (102, "notes")] {
        sandbox
            .run(&["artifact", "create", slug, "--type", "rfc"], seed)
            .ok();
    }
    sandbox
}

/// `rfc-0001 depends_on rfc-0000` (rel seed 200), `rfc-0001 references notes` (201) and
/// `notes extends rfc-0000` (202).
fn linked() -> Sandbox {
    let sandbox = populated();
    sandbox
        .run(
            &["relation", "add", "rfc-0001", "depends_on", "rfc-0000"],
            200,
        )
        .ok();
    sandbox
        .run(&["relation", "add", "rfc-0001", "references", "notes"], 201)
        .ok();
    sandbox
        .run(&["relation", "add", "notes", "extends", "rfc-0000"], 202)
        .ok();
    sandbox
}

fn relationship_files(sandbox: &Sandbox) -> Vec<String> {
    let dir = sandbox.path().join("relationships");
    if !dir.exists() {
        return Vec::new();
    }
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name != ".gitkeep")
        .collect();
    names.sort();
    names
}

// ---------------------------------------------------------------------------------------------
// add
// ---------------------------------------------------------------------------------------------

#[test]
fn add_prints_a_confirmation() {
    let sandbox = populated();
    let out = sandbox.run(
        &["relation", "add", "rfc-0001", "depends_on", "rfc-0000"],
        200,
    );
    assert_snapshot!("add_text", transcript(&out));
    out.ok();
}

#[test]
fn add_in_json_matches_the_schema() {
    let sandbox = populated();
    let out = sandbox
        .run(
            &[
                "relation",
                "add",
                "rfc-0001",
                "depends_on",
                "rfc-0000",
                "--meta",
                "reason=needs the vision",
                "-o",
                "json",
            ],
            200,
        )
        .ok();
    assert_snapshot!("add_json", transcript(&out));
    assert_schema("relationship-document", &out.json());
}

#[test]
fn add_accepts_slugs_ids_canonical_references_and_the_kebab_alias() {
    let sandbox = populated();
    let source = art(101);
    let target = format!("functor:artifact:{}", art(100));
    let out = sandbox
        .run(
            &[
                "relation",
                "add",
                &source,
                "depends-on",
                &target,
                "-o",
                "json",
            ],
            200,
        )
        .ok();
    let relationship = &out.json()["relationship"];
    assert_eq!(relationship["type"], "depends_on");
    assert_eq!(
        relationship["source"],
        json!({"id": art(101), "slug": "rfc-0001"})
    );
    assert_eq!(
        relationship["target"],
        json!({"id": art(100), "slug": "rfc-0000"})
    );
    assert_eq!(relationship["id"], rel(200));
    assert_eq!(
        relationship["ref"],
        format!("functor:relationship:{}", rel(200))
    );
    assert_eq!(relationship["created"], T0);
}

#[test]
fn add_stores_a_canonical_file_named_by_id() {
    let sandbox = populated();
    sandbox
        .run(
            &[
                "relation",
                "add",
                "rfc-0001",
                "depends_on",
                "rfc-0000",
                "--meta",
                "reason=needs the vision",
            ],
            200,
        )
        .ok();
    assert_eq!(relationship_files(&sandbox), [format!("{}.toml", rel(200))]);
    assert_snapshot!(
        "add_file",
        sandbox.read(&format!("relationships/{}.toml", rel(200)))
    );
}

#[test]
fn add_allows_both_directions_and_several_types_between_two_artifacts() {
    let sandbox = populated();
    for (seed, args) in [
        (200, ["rfc-0001", "depends_on", "rfc-0000"]),
        (201, ["rfc-0000", "depends_on", "rfc-0001"]),
        (202, ["rfc-0001", "references", "rfc-0000"]),
    ] {
        let mut full = vec!["relation", "add"];
        full.extend(args);
        sandbox.run(&full, seed).ok();
    }
    assert_eq!(relationship_files(&sandbox).len(), 3);
}

#[test]
fn a_custom_type_needs_allow_custom_type() {
    let sandbox = populated();
    let out = sandbox
        .run(
            &["relation", "add", "rfc-0001", "supports", "rfc-0000"],
            200,
        )
        .fails(2);
    assert_snapshot!("add_custom_type_rejected", transcript(&out));
    assert_eq!(relationship_files(&sandbox), Vec::<String>::new());

    let out = sandbox
        .run(
            &[
                "relation",
                "add",
                "rfc-0001",
                "supports",
                "rfc-0000",
                "--allow-custom-type",
                "-o",
                "json",
            ],
            200,
        )
        .ok();
    assert_eq!(out.json()["relationship"]["type"], "supports");
    assert_schema("relationship-document", &out.json());
}

#[test]
fn built_in_types_do_not_need_the_custom_flag_even_with_it() {
    let sandbox = populated();
    sandbox
        .run(
            &[
                "relation",
                "add",
                "rfc-0001",
                "extends",
                "rfc-0000",
                "--allow-custom-type",
            ],
            200,
        )
        .ok();
}

#[test]
fn an_unknown_endpoint_is_not_found_with_exit_code_3_and_writes_nothing() {
    let sandbox = populated();
    let out = sandbox
        .run(
            &["relation", "add", "rfc-0001", "depends_on", "rfc-9999"],
            200,
        )
        .fails(3);
    assert_snapshot!("add_unknown_target", transcript(&out));
    let out = sandbox
        .run(
            &["relation", "add", "rfc-9999", "depends_on", "rfc-0000"],
            200,
        )
        .fails(3);
    assert!(out.stderr.contains("rfc-9999"), "{}", out.stderr);
    let out = sandbox
        .run(
            &[
                "relation",
                "add",
                "rfc-0000",
                "depends_on",
                "rfc-9999",
                "-o",
                "json",
            ],
            200,
        )
        .fails(3);
    assert_eq!(out.error_json()["error"]["code"], "not_found");
    assert_eq!(out.error_json()["error"]["entity"], "artifact");
    assert_eq!(relationship_files(&sandbox), Vec::<String>::new());
}

#[test]
fn self_reference_of_an_acyclic_type_is_an_input_error() {
    let sandbox = populated();
    for relation_type in ["depends_on", "derived_from", "extends"] {
        let out = sandbox
            .run(
                &["relation", "add", "rfc-0000", relation_type, "rfc-0000"],
                200,
            )
            .fails(2);
        assert!(out.stderr.contains("own source"), "{}", out.stderr);
    }
    // The same artifact through another spelling is still a self-reference.
    let out = sandbox
        .run(
            &["relation", "add", "rfc-0000", "depends-on", &art(100)],
            200,
        )
        .fails(2);
    assert_snapshot!("add_self_reference", transcript(&out));
    assert_eq!(relationship_files(&sandbox), Vec::<String>::new());
}

#[test]
fn self_reference_is_allowed_for_references_and_custom_types() {
    let sandbox = populated();
    sandbox
        .run(
            &["relation", "add", "rfc-0000", "references", "rfc-0000"],
            200,
        )
        .ok();
    sandbox
        .run(
            &[
                "relation",
                "add",
                "rfc-0000",
                "mirrors",
                "rfc-0000",
                "--allow-custom-type",
            ],
            201,
        )
        .ok();
}

#[test]
fn a_duplicate_is_a_conflict_with_exit_code_4() {
    let sandbox = linked();
    let out = sandbox
        .run(
            &["relation", "add", "rfc-0001", "depends-on", &art(100)],
            300,
        )
        .fails(4);
    assert_snapshot!("add_duplicate", transcript(&out));
    assert_eq!(relationship_files(&sandbox).len(), 3);
}

#[test]
fn invalid_add_arguments_are_input_errors_with_exit_code_2() {
    let sandbox = populated();
    let cases: [(&[&str], &str); 4] = [
        (
            &["relation", "add", "rfc-0001", "Bad Type", "rfc-0000"],
            "invalid relationship type",
        ),
        (
            &[
                "relation",
                "add",
                "rfc-0001",
                "depends_on",
                "rfc-0000",
                "--meta",
                "novalue",
            ],
            "KEY=VALUE",
        ),
        (
            &["relation", "add", "Not A Slug", "depends_on", "rfc-0000"],
            "invalid slug",
        ),
        (
            &["relation", "add", "rfc-0001", "depends_on", "art_xyz"],
            "invalid",
        ),
    ];
    for (args, expected) in cases {
        let out = sandbox.run(args, 200).fails(2);
        assert!(out.stderr.contains(expected), "{args:?}: {}", out.stderr);
    }
    assert_eq!(relationship_files(&sandbox), Vec::<String>::new());
}

#[test]
fn missing_arguments_are_usage_errors() {
    let sandbox = populated();
    sandbox
        .run(&["relation", "add", "rfc-0001", "depends_on"], 200)
        .fails(2);
    sandbox.run(&["relation", "show"], 200).fails(2);
}

#[test]
fn quiet_silences_the_text_confirmation_but_not_json() {
    let sandbox = populated();
    let out = sandbox
        .run(
            &[
                "-q",
                "relation",
                "add",
                "rfc-0001",
                "depends_on",
                "rfc-0000",
            ],
            200,
        )
        .ok();
    assert_eq!(out.stdout, "");
    let out = sandbox
        .run(
            &[
                "-q",
                "-o",
                "json",
                "relation",
                "add",
                "rfc-0000",
                "references",
                "rfc-0001",
            ],
            201,
        )
        .ok();
    assert_schema("relationship-document", &out.json());
}

// ---------------------------------------------------------------------------------------------
// list
// ---------------------------------------------------------------------------------------------

#[test]
fn list_prints_a_table_ordered_by_id() {
    let sandbox = linked();
    let out = sandbox.run(&["relation", "list"], 300).ok();
    assert_snapshot!("list_text", transcript(&out));
}

#[test]
fn list_in_json_matches_the_schema() {
    let sandbox = linked();
    let out = sandbox.run(&["relation", "list", "-o", "json"], 300).ok();
    assert_snapshot!("list_json", transcript(&out));
    let document = out.json();
    assert_schema("relationship-list", &document);
    let ids: Vec<&str> = document["relationships"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [rel(200), rel(201), rel(202)]);
}

#[test]
fn list_is_ordered_by_id_not_by_creation_order() {
    let sandbox = populated();
    sandbox
        .run(&["relation", "add", "rfc-0001", "references", "notes"], 700)
        .ok();
    sandbox
        .run(
            &["relation", "add", "rfc-0001", "depends_on", "rfc-0000"],
            500,
        )
        .ok();
    let out = sandbox.run(&["relation", "list", "-o", "json"], 900).ok();
    let ids: Vec<String> = out.json()["relationships"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(ids, [rel(500), rel(700)]);
}

fn listed_ids(out: &Out) -> Vec<String> {
    out.json()["relationships"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn list_filters_by_type_source_and_target() {
    let sandbox = linked();
    let list = |extra: &[&str]| {
        let mut args = vec!["relation", "list", "-o", "json"];
        args.extend(extra);
        listed_ids(&sandbox.run(&args, 300).ok())
    };

    assert_eq!(list(&["--type", "depends_on"]), [rel(200)]);
    assert_eq!(list(&["--type", "depends-on"]), [rel(200)]);
    assert_eq!(
        list(&["--type", "depends_on", "--type", "extends"]),
        [rel(200), rel(202)]
    );
    assert_eq!(list(&["--source", "rfc-0001"]), [rel(200), rel(201)]);
    assert_eq!(list(&["--target", "rfc-0000"]), [rel(200), rel(202)]);
    assert_eq!(list(&["--target", &art(100)]), [rel(200), rel(202)]);
    assert_eq!(
        list(&["--source", "notes", "--target", "rfc-0000"]),
        [rel(202)]
    );
    assert_eq!(
        list(&["--source", "rfc-0001", "--type", "references"]),
        [rel(201)]
    );
    assert_eq!(list(&["--type", "supports"]), Vec::<String>::new());
    assert_eq!(list(&["--source", "rfc-0000"]), Vec::<String>::new());
}

#[test]
fn list_with_an_unknown_artifact_filter_is_not_found() {
    let sandbox = linked();
    let out = sandbox
        .run(&["relation", "list", "--source", "rfc-9999"], 300)
        .fails(3);
    assert_snapshot!("list_unknown_source", transcript(&out));
    sandbox
        .run(&["relation", "list", "--target", "rfc-9999"], 300)
        .fails(3);
    sandbox
        .run(&["relation", "list", "--type", "Bad Type"], 300)
        .fails(2);
}

#[test]
fn an_empty_list_is_success_in_both_modes() {
    let sandbox = populated();
    let out = sandbox.run(&["relation", "list"], 300).ok();
    assert_eq!(out.stdout, "no relationships found\n");
    let out = sandbox.run(&["relation", "list", "-o", "json"], 300).ok();
    assert_eq!(out.stdout, "{\"schema_version\":1,\"relationships\":[]}\n");
    assert_schema("relationship-list", &out.json());
}

// ---------------------------------------------------------------------------------------------
// show
// ---------------------------------------------------------------------------------------------

#[test]
fn show_describes_a_relationship() {
    let sandbox = populated();
    sandbox
        .run(
            &[
                "relation",
                "add",
                "rfc-0001",
                "depends_on",
                "rfc-0000",
                "--meta",
                "reason=needs the vision",
            ],
            200,
        )
        .ok();
    let out = sandbox.run(&["relation", "show", &rel(200)], 300).ok();
    assert_snapshot!("show_text", transcript(&out));
    let out = sandbox
        .run(&["relation", "show", &rel(200), "-o", "json"], 300)
        .ok();
    assert_snapshot!("show_json", transcript(&out));
    assert_schema("relationship-document", &out.json());
}

#[test]
fn show_accepts_the_canonical_reference_and_prints_the_same_document() {
    let sandbox = linked();
    let by_id = sandbox
        .run(&["relation", "show", &rel(201), "-o", "json"], 300)
        .ok();
    let canonical = format!("functor:relationship:{}", rel(201));
    let by_ref = sandbox
        .run(&["relation", "show", &canonical, "-o", "json"], 300)
        .ok();
    assert_eq!(by_id.stdout, by_ref.stdout);
}

#[test]
fn show_of_an_unknown_relationship_is_not_found() {
    let sandbox = linked();
    let out = sandbox.run(&["relation", "show", &rel(999)], 300).fails(3);
    assert_snapshot!("show_not_found", transcript(&out));
    let out = sandbox
        .run(&["relation", "show", &rel(999), "-o", "json"], 300)
        .fails(3);
    assert_eq!(out.error_json()["error"]["entity"], "relationship");
}

#[test]
fn show_needs_a_relationship_id_not_a_slug_or_an_artifact_id() {
    let sandbox = linked();
    for bad in ["rfc-0000", "rel_xyz", "functor:relationship:xyz"] {
        sandbox.run(&["relation", "show", bad], 300).fails(2);
    }
    let out = sandbox.run(&["relation", "show", &art(100)], 300).fails(2);
    assert!(
        out.stderr.contains("not a relationship id"),
        "{}",
        out.stderr
    );
    sandbox
        .run(
            &[
                "relation",
                "show",
                &format!("functor:artifact:{}", art(100)),
            ],
            300,
        )
        .fails(2);
}

#[test]
fn a_dangling_endpoint_makes_the_repository_inconsistent() {
    let sandbox = linked();
    fs::remove_file(sandbox.path().join(format!("artifacts/{}.toml", art(100)))).unwrap();

    for args in [
        vec!["relation", "show", &rel(200)],
        vec!["relation", "list"],
        vec!["relation", "add", "rfc-0001", "references", "rfc-0001"],
    ] {
        let out = sandbox.run(&args, 300).fails(5);
        assert!(
            out.stderr.contains("missing target"),
            "{args:?}: {}",
            out.stderr
        );
        assert!(out.stderr.contains("functor validate"), "{}", out.stderr);
    }
}

// ---------------------------------------------------------------------------------------------
// repository discovery and schemas
// ---------------------------------------------------------------------------------------------

#[test]
fn relation_commands_outside_a_repository_exit_with_code_5() {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        &["relation", "list"][..],
        &["relation", "show", "rel_00000000000000000000000000"],
        &["relation", "add", "a", "depends_on", "b"],
    ] {
        run_in(dir.path(), args, T0, 1).fails(5);
    }
}

#[test]
fn the_schemas_can_fail() {
    let sandbox = linked();
    let list = sandbox
        .run(&["relation", "list", "-o", "json"], 300)
        .ok()
        .json();
    let document = sandbox
        .run(&["relation", "show", &rel(200), "-o", "json"], 300)
        .ok()
        .json();

    let mut wrong_version = document.clone();
    wrong_version["schema_version"] = json!(2);
    assert_schema_rejects("relationship-document", &wrong_version);

    let mut extra = document.clone();
    extra["relationship"]["surprise"] = json!(true);
    assert_schema_rejects("relationship-document", &extra);

    let mut bad_id = document.clone();
    bad_id["relationship"]["id"] = json!(art(1));
    assert_schema_rejects("relationship-document", &bad_id);

    let mut missing_slug = document.clone();
    missing_slug["relationship"]["source"]
        .as_object_mut()
        .unwrap()
        .remove("slug");
    assert_schema_rejects("relationship-document", &missing_slug);

    let mut null_slug = document.clone();
    null_slug["relationship"]["target"]["slug"] = Value::Null;
    assert_schema_rejects("relationship-document", &null_slug);

    let mut bad_list = list.clone();
    bad_list["relationships"][0]["type"] = json!("Bad Type");
    assert_schema_rejects("relationship-list", &bad_list);
    assert_schema_rejects("relationship-list", &json!({"schema_version": 1}));
}

#[test]
fn the_shared_definitions_in_both_schemas_are_identical() {
    let document = load_schema("relationship-document");
    let list = load_schema("relationship-list");
    assert_eq!(document["$defs"], list["$defs"]);
}

#[test]
fn the_endpoint_slug_pattern_matches_the_artifact_schema() {
    let artifact = load_schema("artifact-document");
    let relationship = load_schema("relationship-document");
    let slug: &Value = &artifact["$defs"]["Artifact"]["properties"]["slug"];
    let endpoint = &relationship["$defs"]["Endpoint"]["properties"]["slug"];
    assert_eq!(slug, endpoint);
}

// ---------------------------------------------------------------------------------------------
// concurrency across processes
// ---------------------------------------------------------------------------------------------

#[test]
fn concurrent_processes_never_lose_relationships() {
    let sandbox = populated();
    let path = sandbox.path().to_path_buf();
    let workers: Vec<_> = [
        ["rfc-0001", "depends_on", "rfc-0000"],
        ["rfc-0001", "references", "rfc-0000"],
        ["rfc-0001", "extends", "notes"],
        ["notes", "references", "rfc-0001"],
        ["notes", "derived_from", "rfc-0000"],
        ["rfc-0000", "references", "notes"],
    ]
    .into_iter()
    .enumerate()
    .map(|(index, triple)| {
        let path = path.clone();
        thread::spawn(move || {
            let mut args = vec!["relation", "add"];
            args.extend(triple);
            run_in(&path, &args, T0, 1000 + u64::try_from(index).unwrap() * 10)
        })
    })
    .collect();
    for worker in workers {
        worker.join().unwrap().ok();
    }
    let out = sandbox.run(&["relation", "list", "-o", "json"], 5000).ok();
    assert_eq!(listed_ids(&out).len(), 6);
}

#[test]
fn racing_processes_adding_one_relationship_have_exactly_one_winner() {
    let sandbox = populated();
    let path = sandbox.path().to_path_buf();
    let workers: Vec<_> = (0..4_u64)
        .map(|index| {
            let path = path.clone();
            thread::spawn(move || {
                run_in(
                    &path,
                    &["relation", "add", "rfc-0001", "depends_on", "rfc-0000"],
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
    assert_eq!(relationship_files(&sandbox).len(), 1);
}

// ---------------------------------------------------------------------------------------------
// the milestone's acceptance scenario
// ---------------------------------------------------------------------------------------------

#[test]
fn artifacts_form_a_navigable_graph_on_disk() {
    let sandbox = populated();
    // Slugs, ids and the kebab alias all work for the roadmap's example.
    sandbox
        .run(
            &["relation", "add", "rfc-0001", "depends-on", "rfc-0000"],
            200,
        )
        .ok();
    // The graph is on disk, readable by a fresh process with nothing but the repository.
    let shown = sandbox.run(&["relation", "show", &rel(200)], 300).ok();
    assert!(shown.stdout.contains("rfc-0001"), "{}", shown.stdout);
    let outgoing = sandbox
        .run(
            &["relation", "list", "--source", "rfc-0001", "-o", "json"],
            300,
        )
        .ok();
    assert_eq!(listed_ids(&outgoing), [rel(200)]);
    let incoming = sandbox
        .run(
            &["relation", "list", "--target", &art(100), "-o", "json"],
            300,
        )
        .ok();
    assert_eq!(listed_ids(&incoming), [rel(200)]);
}
