//! End-to-end tests of `functor query ...`: golden transcripts, JSON schemas, filters, exit codes
//! and the guarantee that queries never change the repository.
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

/// Four artifacts (ids 100 to 103) and four relationships (ids 200 to 203):
///
/// ```text
/// rfc-0001 depends_on   rfc-0000      (200)
/// rfc-0002 depends_on   rfc-0001      (201)
/// notes    references   rfc-0002      (202)
/// rfc-0002 derived_from rfc-0000      (203)
/// ```
fn graph() -> Sandbox {
    let sandbox = Sandbox::new();
    let create = |seed: u64, args: &[&str]| {
        let mut full = vec!["artifact", "create"];
        full.extend(args);
        sandbox.run(&full, seed).ok();
    };
    create(
        100,
        &[
            "rfc-0000",
            "--type",
            "rfc",
            "--title",
            "Functor Vision",
            "--description",
            "Platform vision and goals",
            "--tag",
            "core",
            "--meta",
            "status=draft",
        ],
    );
    create(
        101,
        &[
            "rfc-0001",
            "--type",
            "rfc",
            "--title",
            "Core Semantic Model",
            "--tag",
            "core",
            "--meta",
            "status=accepted",
        ],
    );
    create(
        102,
        &[
            "rfc-0002",
            "--type",
            "rfc",
            "--title",
            "Query Model",
            "--tag",
            "core",
            "--tag",
            "architecture",
            "--meta",
            "status=draft",
        ],
    );
    create(103, &["notes", "--title", "Meeting Notes"]);
    for (seed, source, relationship_type, target) in [
        (200, "rfc-0001", "depends_on", "rfc-0000"),
        (201, "rfc-0002", "depends_on", "rfc-0001"),
        (202, "notes", "references", "rfc-0002"),
        (203, "rfc-0002", "derived_from", "rfc-0000"),
    ] {
        sandbox
            .run(
                &["relation", "add", source, relationship_type, target],
                seed,
            )
            .ok();
    }
    sandbox
}

fn query(sandbox: &Sandbox, args: &[&str]) -> Out {
    let mut full = vec!["query"];
    full.extend(args);
    sandbox.run(&full, 900)
}

fn slugs(out: &Out) -> Vec<String> {
    out.json()["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["slug"].as_str().unwrap().to_owned())
        .collect()
}

/// `(slug, depth)` of every node of a traversal document.
fn nodes(out: &Out) -> Vec<(String, u64)> {
    out.json()["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            (
                n["artifact"]["slug"].as_str().unwrap().to_owned(),
                n["depth"].as_u64().unwrap(),
            )
        })
        .collect()
}

fn pairs(items: &[(&str, u64)]) -> Vec<(String, u64)> {
    items.iter().map(|(s, d)| ((*s).to_owned(), *d)).collect()
}

// ---------------------------------------------------------------------------------------------
// query artifacts
// ---------------------------------------------------------------------------------------------

#[test]
fn artifacts_without_criteria_lists_everything_ordered_by_slug() {
    let sandbox = graph();
    let out = query(&sandbox, &["artifacts"]).ok();
    assert_snapshot!("artifacts_all_text", transcript(&out));
}

#[test]
fn artifacts_in_json_matches_the_schema() {
    let sandbox = graph();
    let out = query(&sandbox, &["artifacts", "--tag", "core", "-o", "json"]).ok();
    assert_snapshot!("artifacts_json", transcript(&out));
    assert_schema("artifact-list", &out.json());
}

#[test]
fn artifacts_filters_by_type_tag_text_and_metadata() {
    let sandbox = graph();
    let find = |args: &[&str]| {
        let mut full = vec!["artifacts", "-o", "json"];
        full.extend(args);
        slugs(&query(&sandbox, &full).ok())
    };
    assert_eq!(
        find(&["--type", "rfc"]),
        ["rfc-0000", "rfc-0001", "rfc-0002"]
    );
    assert_eq!(find(&["--type", "document"]), ["notes"]);
    assert_eq!(find(&["--tag", "architecture"]), ["rfc-0002"]);
    assert_eq!(find(&["--tag", "missing"]), Vec::<String>::new());
    assert_eq!(find(&["--text", "vision"]), ["rfc-0000"]);
    assert_eq!(find(&["--text", "SEMANTIC"]), ["rfc-0001"]);
    assert_eq!(find(&["--text", "meeting"]), ["notes"]);
    assert_eq!(
        find(&["--text", "rfc-000"]),
        ["rfc-0000", "rfc-0001", "rfc-0002"]
    );
    assert_eq!(find(&["--meta", "status=draft"]), ["rfc-0000", "rfc-0002"]);
    assert_eq!(find(&["--meta", "status=accepted"]), ["rfc-0001"]);
    assert_eq!(find(&["--meta", "status=Draft"]), Vec::<String>::new());
}

#[test]
fn artifact_criteria_combine_with_and() {
    let sandbox = graph();
    let find = |args: &[&str]| {
        let mut full = vec!["artifacts", "-o", "json"];
        full.extend(args);
        slugs(&query(&sandbox, &full).ok())
    };
    assert_eq!(
        find(&["--type", "rfc", "--tag", "core", "--meta", "status=draft"]),
        ["rfc-0000", "rfc-0002"]
    );
    assert_eq!(
        find(&[
            "--meta",
            "status=draft",
            "--tag",
            "architecture",
            "--text",
            "query"
        ]),
        ["rfc-0002"]
    );
    // Every `--meta` must match.
    assert_eq!(
        find(&["--meta", "status=draft", "--meta", "status=accepted"]),
        Vec::<String>::new()
    );
    assert_eq!(
        find(&["--type", "document", "--tag", "core"]),
        Vec::<String>::new()
    );
}

#[test]
fn an_empty_result_is_success_in_both_modes() {
    let sandbox = graph();
    let out = query(&sandbox, &["artifacts", "--tag", "missing"]).ok();
    assert_eq!(out.stdout, "no artifacts found\n");
    let out = query(&sandbox, &["artifacts", "--tag", "missing", "-o", "json"]).ok();
    assert_eq!(out.stdout, "{\"schema_version\":1,\"artifacts\":[]}\n");
    assert_schema("artifact-list", &out.json());
}

#[test]
fn query_artifacts_equals_artifact_list_for_the_same_filters() {
    let sandbox = graph();
    for filter in [&["--type", "rfc"][..], &["--tag", "core"], &[]] {
        let mut queried = vec!["artifacts", "-o", "json"];
        queried.extend(filter);
        let mut listed = vec!["artifact", "list", "-o", "json"];
        listed.extend(filter);
        assert_eq!(
            query(&sandbox, &queried).ok().stdout,
            sandbox.run(&listed, 900).ok().stdout
        );
    }
}

#[test]
fn malformed_artifact_criteria_are_input_errors() {
    let sandbox = graph();
    for (args, expected) in [
        (
            &["artifacts", "--type", "Bad Type"][..],
            "invalid artifact type",
        ),
        (&["artifacts", "--tag", "Bad Tag"], "invalid tag"),
        (&["artifacts", "--meta", "novalue"], "KEY=VALUE"),
        (
            &["artifacts", "--meta", "Bad Key=v"],
            "invalid metadata key",
        ),
    ] {
        let out = query(&sandbox, args).fails(2);
        assert!(out.stderr.contains(expected), "{args:?}: {}", out.stderr);
    }
}

// ---------------------------------------------------------------------------------------------
// query dependencies and dependents
// ---------------------------------------------------------------------------------------------

#[test]
fn dependencies_default_to_direct_depends_on_neighbours() {
    let sandbox = graph();
    let out = query(&sandbox, &["dependencies", "rfc-0002"]).ok();
    assert_snapshot!("dependencies_text", transcript(&out));
}

#[test]
fn dependencies_depth_all_is_the_transitive_closure() {
    let sandbox = graph();
    let out = query(&sandbox, &["dependencies", "rfc-0002", "--depth", "all"]).ok();
    assert_snapshot!("dependencies_all_text", transcript(&out));
}

#[test]
fn dependencies_in_json_matches_the_schema() {
    let sandbox = graph();
    let out = query(
        &sandbox,
        &["dependencies", "rfc-0002", "--depth", "all", "-o", "json"],
    )
    .ok();
    assert_snapshot!("dependencies_all_json", transcript(&out));
    assert_schema("query-traversal", &out.json());

    let limited = query(&sandbox, &["dependencies", "rfc-0002", "-o", "json"]).ok();
    assert_schema("query-traversal", &limited.json());
    assert_eq!(limited.json()["max_depth"], 1);
    assert_eq!(out.json()["max_depth"], Value::Null);
    assert_eq!(out.json()["direction"], "outbound");
    assert_eq!(out.json()["relationship_types"], json!(["depends_on"]));
    assert_eq!(
        out.json()["start"],
        json!({"id": art(102), "slug": "rfc-0002"})
    );
}

#[test]
fn dependency_depths_and_edges() {
    let sandbox = graph();
    let run = |depth: &str| {
        query(
            &sandbox,
            &["dependencies", "rfc-0002", "--depth", depth, "-o", "json"],
        )
        .ok()
    };
    assert_eq!(nodes(&run("0")), pairs(&[]));
    assert_eq!(run("0").json()["edges"], json!([]));
    assert_eq!(nodes(&run("1")), pairs(&[("rfc-0001", 1)]));
    assert_eq!(nodes(&run("2")), pairs(&[("rfc-0001", 1), ("rfc-0000", 2)]));
    assert_eq!(nodes(&run("99")), nodes(&run("all")));
    assert_eq!(nodes(&run("ALL")), nodes(&run("all")));

    let all = run("all").json();
    let edge_ids: Vec<&str> = all["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(edge_ids, [rel(200), rel(201)]);
    // `via` names the edge that reached the node.
    assert_eq!(all["nodes"][0]["via"]["id"], rel(201));
    assert_eq!(all["nodes"][1]["via"]["id"], rel(200));
}

#[test]
fn dependents_follow_edges_backwards() {
    let sandbox = graph();
    let out = query(&sandbox, &["dependents", "rfc-0000", "--depth", "all"]).ok();
    assert_snapshot!("dependents_all_text", transcript(&out));

    let json = query(
        &sandbox,
        &["dependents", "rfc-0000", "--depth", "all", "-o", "json"],
    )
    .ok();
    assert_schema("query-traversal", &json.json());
    assert_eq!(json.json()["direction"], "inbound");
    assert_eq!(nodes(&json), pairs(&[("rfc-0001", 1), ("rfc-0002", 2)]));
    // Direct dependents only by default.
    let direct = query(&sandbox, &["dependents", "rfc-0000", "-o", "json"]).ok();
    assert_eq!(nodes(&direct), pairs(&[("rfc-0001", 1)]));
}

#[test]
fn the_documented_direction_semantics_hold() {
    // `rfc-0001 depends_on rfc-0000`: dependencies of 0001 is 0000; dependents of 0000 is 0001.
    let sandbox = graph();
    let deps = query(&sandbox, &["dependencies", "rfc-0001", "-o", "json"]).ok();
    assert_eq!(nodes(&deps), pairs(&[("rfc-0000", 1)]));
    let dependents = query(&sandbox, &["dependents", "rfc-0000", "-o", "json"]).ok();
    assert_eq!(nodes(&dependents), pairs(&[("rfc-0001", 1)]));
}

#[test]
fn type_replaces_the_default_and_is_repeatable() {
    let sandbox = graph();
    let run = |args: &[&str]| {
        let mut full = vec!["dependencies", "rfc-0002", "--depth", "all", "-o", "json"];
        full.extend(args);
        query(&sandbox, &full).ok()
    };
    let derived = run(&["--type", "derived_from"]);
    assert_eq!(nodes(&derived), pairs(&[("rfc-0000", 1)]));
    assert_eq!(
        derived.json()["relationship_types"],
        json!(["derived_from"])
    );

    let kebab = run(&["--type", "derived-from"]);
    assert_eq!(kebab.json()["relationship_types"], json!(["derived_from"]));

    let both = run(&["--type", "depends_on", "--type", "derived_from"]);
    assert_eq!(nodes(&both), pairs(&[("rfc-0000", 1), ("rfc-0001", 1)]));
    assert_eq!(
        both.json()["relationship_types"],
        json!(["depends_on", "derived_from"])
    );
    assert_schema("query-traversal", &both.json());

    let none = run(&["--type", "extends"]);
    assert_eq!(nodes(&none), pairs(&[]));
}

#[test]
fn an_artifact_without_dependencies_prints_the_empty_line() {
    let sandbox = graph();
    let out = query(&sandbox, &["dependencies", "rfc-0000"]).ok();
    assert_eq!(out.stdout, "no dependencies found\n");
    let out = query(&sandbox, &["dependents", "notes"]).ok();
    assert_eq!(out.stdout, "no dependents found\n");
    let out = query(&sandbox, &["dependencies", "rfc-0000", "-o", "json"]).ok();
    assert_schema("query-traversal", &out.json());
    assert_eq!(out.json()["nodes"], json!([]));
}

#[test]
fn start_artifacts_resolve_by_slug_id_and_canonical_reference() {
    let sandbox = graph();
    let by_slug = query(&sandbox, &["dependencies", "rfc-0002", "-o", "json"]).ok();
    let by_id = query(&sandbox, &["dependencies", &art(102), "-o", "json"]).ok();
    let canonical = format!("functor:artifact:{}", art(102));
    let by_ref = query(&sandbox, &["dependencies", &canonical, "-o", "json"]).ok();
    assert_eq!(by_slug.stdout, by_id.stdout);
    assert_eq!(by_slug.stdout, by_ref.stdout);
}

#[test]
fn cycles_terminate() {
    let sandbox = Sandbox::new();
    for (seed, slug) in [(100, "a"), (101, "b"), (102, "c")] {
        sandbox.run(&["artifact", "create", slug], seed).ok();
    }
    for (seed, source, target) in [(200, "a", "b"), (201, "b", "c"), (202, "c", "a")] {
        sandbox
            .run(&["relation", "add", source, "depends_on", target], seed)
            .ok();
    }
    let out = query(
        &sandbox,
        &["dependencies", "a", "--depth", "all", "-o", "json"],
    )
    .ok();
    assert_eq!(nodes(&out), pairs(&[("b", 1), ("c", 2)]));
    assert_eq!(out.json()["edges"].as_array().unwrap().len(), 3);
    assert_schema("query-traversal", &out.json());
}

#[test]
fn unknown_start_artifacts_are_not_found_with_suggestions() {
    let sandbox = graph();
    let out = query(&sandbox, &["dependencies", "rfc-0003"]).fails(3);
    assert_snapshot!("dependencies_unknown", transcript(&out));
    query(&sandbox, &["dependents", "nope"]).fails(3);
    query(&sandbox, &["dependencies", &art(999)]).fails(3);
    let out = query(&sandbox, &["relationships", "nope", "-o", "json"]).fails(3);
    assert_eq!(out.error_json()["error"]["entity"], "artifact");
}

#[test]
fn invalid_traversal_arguments_are_usage_or_input_errors() {
    let sandbox = graph();
    for args in [
        &["dependencies", "rfc-0002", "--depth", "x"][..],
        &["dependencies", "rfc-0002", "--depth", "-1"],
        &["dependencies", "rfc-0002", "--depth", "1.5"],
        &["dependencies", "rfc-0002", "--type", "Bad Type"],
        &["dependents", "Not A Slug"],
        &["dependencies"],
        &["relationships", "rfc-0002", "--direction", "sideways"],
    ] {
        query(&sandbox, args).fails(2);
    }
}

// ---------------------------------------------------------------------------------------------
// query relationships
// ---------------------------------------------------------------------------------------------

#[test]
fn relationships_lists_both_directions_ordered_by_id() {
    let sandbox = graph();
    let out = query(&sandbox, &["relationships", "rfc-0002"]).ok();
    assert_snapshot!("relationships_text", transcript(&out));
}

#[test]
fn relationships_in_json_matches_the_schema() {
    let sandbox = graph();
    let out = query(&sandbox, &["relationships", "rfc-0002", "-o", "json"]).ok();
    assert_snapshot!("relationships_json", transcript(&out));
    assert_schema("query-relationships", &out.json());
    assert_eq!(
        out.json()["artifact"],
        json!({"id": art(102), "slug": "rfc-0002"})
    );
}

#[test]
fn relationships_filter_by_direction_and_type() {
    let sandbox = graph();
    let ids = |args: &[&str]| -> Vec<String> {
        let mut full = vec!["relationships", "rfc-0002", "-o", "json"];
        full.extend(args);
        query(&sandbox, &full).ok().json()["relationships"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(ids(&[]), [rel(201), rel(202), rel(203)]);
    assert_eq!(ids(&["--direction", "both"]), ids(&[]));
    assert_eq!(ids(&["--direction", "out"]), [rel(201), rel(203)]);
    assert_eq!(ids(&["--direction", "in"]), [rel(202)]);
    assert_eq!(ids(&["--type", "depends_on"]), [rel(201)]);
    assert_eq!(
        ids(&["--type", "depends-on", "--type", "references"]),
        [rel(201), rel(202)]
    );
    assert_eq!(
        ids(&["--direction", "out", "--type", "references"]),
        Vec::<String>::new()
    );
}

#[test]
fn relationships_of_an_unconnected_artifact_is_empty() {
    let sandbox = Sandbox::new();
    sandbox.run(&["artifact", "create", "lonely"], 100).ok();
    let out = query(&sandbox, &["relationships", "lonely"]).ok();
    assert_eq!(out.stdout, "no relationships found\n");
    let out = query(&sandbox, &["relationships", "lonely", "-o", "json"]).ok();
    assert_schema("query-relationships", &out.json());
    assert_eq!(out.json()["relationships"], json!([]));
}

// ---------------------------------------------------------------------------------------------
// cross-cutting
// ---------------------------------------------------------------------------------------------

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
                files.insert(name, fs::read(&path).unwrap());
            }
        }
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files);
    files
}

#[test]
fn queries_never_change_the_repository() {
    let sandbox = graph();
    let before = repository_files(sandbox.path());
    for args in [
        &["artifacts"][..],
        &["artifacts", "--tag", "core", "-o", "json"],
        &["dependencies", "rfc-0002", "--depth", "all"],
        &["dependents", "rfc-0000", "--depth", "all", "-o", "json"],
        &["relationships", "rfc-0002"],
        &["dependencies", "missing"],
    ] {
        let _ = query(&sandbox, args);
    }
    assert_eq!(repository_files(sandbox.path()), before);
}

#[test]
fn quiet_does_not_silence_query_results() {
    let sandbox = graph();
    let out = sandbox.run(&["-q", "query", "artifacts"], 900).ok();
    assert!(out.stdout.contains("rfc-0000"), "{}", out.stdout);
    let out = sandbox
        .run(&["-q", "query", "dependencies", "rfc-0002"], 900)
        .ok();
    assert!(out.stdout.contains("rfc-0001"), "{}", out.stdout);
}

#[test]
fn query_commands_outside_a_repository_exit_with_code_5() {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        &["query", "artifacts"][..],
        &["query", "dependencies", "a"],
        &["query", "dependents", "a"],
        &["query", "relationships", "a"],
    ] {
        run_in(dir.path(), args, T0, 1).fails(5);
    }
}

#[test]
fn a_dangling_relationship_makes_queries_fail_as_inconsistent() {
    let sandbox = graph();
    fs::remove_file(sandbox.path().join(format!("artifacts/{}.toml", art(100)))).unwrap();
    for args in [
        &["artifacts"][..],
        &["dependencies", "rfc-0002"],
        &["relationships", "rfc-0002"],
    ] {
        let out = query(&sandbox, args).fails(5);
        assert!(
            out.stderr.contains("functor validate"),
            "{args:?}: {}",
            out.stderr
        );
    }
}

#[test]
fn the_schemas_can_fail() {
    let sandbox = graph();
    let traversal = query(
        &sandbox,
        &["dependencies", "rfc-0002", "--depth", "all", "-o", "json"],
    )
    .ok()
    .json();
    let relationships = query(&sandbox, &["relationships", "rfc-0002", "-o", "json"])
        .ok()
        .json();

    let mut wrong_direction = traversal.clone();
    wrong_direction["direction"] = json!("sideways");
    assert_schema_rejects("query-traversal", &wrong_direction);

    let mut zero_depth_node = traversal.clone();
    zero_depth_node["nodes"][0]["depth"] = json!(0);
    assert_schema_rejects("query-traversal", &zero_depth_node);

    let mut missing_via = traversal.clone();
    missing_via["nodes"][0]
        .as_object_mut()
        .unwrap()
        .remove("via");
    assert_schema_rejects("query-traversal", &missing_via);

    let mut no_types = traversal.clone();
    no_types["relationship_types"] = json!([]);
    assert_schema_rejects("query-traversal", &no_types);

    let mut extra = traversal.clone();
    extra["surprise"] = json!(1);
    assert_schema_rejects("query-traversal", &extra);

    let mut bad_artifact = relationships.clone();
    bad_artifact["artifact"]["id"] = json!(rel(1));
    assert_schema_rejects("query-relationships", &bad_artifact);
    assert_schema_rejects("query-relationships", &json!({"schema_version": 1}));
}

#[test]
fn shared_schema_definitions_are_identical_everywhere() {
    let artifact_list = load_schema("artifact-list");
    let relationship_list = load_schema("relationship-list");
    let traversal = load_schema("query-traversal");
    let relationships = load_schema("query-relationships");

    assert_eq!(
        traversal["$defs"]["Artifact"],
        artifact_list["$defs"]["Artifact"]
    );
    for (name, schema) in [
        ("query-traversal", &traversal),
        ("query-relationships", &relationships),
    ] {
        for definition in ["Endpoint", "Relationship"] {
            assert_eq!(
                schema["$defs"][definition], relationship_list["$defs"][definition],
                "{name}: {definition}"
            );
        }
    }
    assert_eq!(
        relationships["$defs"].as_object().unwrap().len(),
        2,
        "query-relationships defines exactly Endpoint and Relationship"
    );
    assert_eq!(traversal["$defs"].as_object().unwrap().len(), 3);
}
