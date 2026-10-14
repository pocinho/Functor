//! CLI execution contracts, using a real bounded child and reopening evidence in a library.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::fs;
use std::path::Path;

use common::{Out, Sandbox, T0, assert_schema, assert_schema_rejects, run_in};
use functor_core::{
    ArtifactRepository, EvidenceRecorder, QueryEngine, ReportRecorder, RepositoryReader,
};
use functor_model::{ArtifactRef, Content, Direction, RelationshipType, RunOutcome, Traversal};
use functor_query::SnapshotQueryEngine;
use functor_repository::FileRepositoryManager;
use functor_store::{FileEvidence, FileStore};
use functor_testkit::Env;
use serde_json::json;

fn world(args: &[&str], outputs: bool) -> Sandbox {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/scratch");
    fs::create_dir_all(&root).unwrap();
    let dir = tempfile::Builder::new()
        .prefix("cli-run-")
        .tempdir_in(root)
        .unwrap();
    let sandbox = Sandbox { dir };
    run_in(sandbox.path(), &["init", "--name", "Execution"], T0, 1).ok();
    fs::create_dir_all(sandbox.path().join("tools")).unwrap();
    let tool = format!("tools/fixture{}", std::env::consts::EXE_SUFFIX);
    let target = sandbox.path().join(&tool);
    let fixture = env!("CARGO_BIN_EXE_execution-fixture");
    if fs::hard_link(fixture, &target).is_err() {
        fs::copy(fixture, &target).unwrap();
    }
    let args = args
        .iter()
        .map(|arg| format!("{arg:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let output = if outputs {
        "[outputs]\nresult = { type = \"dataset\", format = \"text\", required = true }\n"
    } else {
        ""
    };
    sandbox.write("workflow.toml", &format!("schema = \"functor.workflow/1\"\nprovider = \"local-process\"\nprogram = \"{tool}\"\nargs = [{args}]\n[inputs]\ndata = {{ required = true }}\n{output}"));
    sandbox
        .run(
            &[
                "artifact",
                "create",
                "workflow",
                "--type",
                "workflow",
                "--content-path",
                "workflow.toml",
            ],
            100,
        )
        .ok();
    sandbox
        .run(
            &["artifact", "create", "input", "--content-text", "hello"],
            200,
        )
        .ok();
    sandbox
}

fn echo() -> Sandbox {
    world(&["echo-input", "inputs/data", "outputs/result"], true)
}

fn start(sandbox: &Sandbox, extra: &[&str], seed: u64) -> Out {
    let mut args = vec![
        "run",
        "start",
        "workflow",
        "--input",
        "data=input",
        "--grant-exec",
        "-o",
        "json",
    ];
    args.extend(extra);
    sandbox.run(&args, seed)
}

fn exits(out: Out, code: i32) -> Out {
    assert_eq!(
        out.code, code,
        "stdout: {}\nstderr: {}",
        out.stdout, out.stderr
    );
    out
}

fn golden(name: &str, out: &Out) {
    let mut document = out.json();
    if let Some(run) = document.get_mut("run") {
        run["program"]["path"] = json!("<program>");
        if run["program"]["sha256"].is_string() {
            run["program"]["sha256"] = json!("<sha256>");
        }
        run["platform"] = json!("<platform>");
        run["env_names"] = json!(["<environment names>"]);
    }
    if document.get("program").is_some() {
        document["program"] = json!("<program>");
        document["env_names"] = json!(["<environment names>"]);
    }
    insta::assert_snapshot!(
        name,
        format!(
            "exit: {}\n{}",
            out.code,
            serde_json::to_string_pretty(&document).unwrap()
        )
    );
}

fn evidence(path: &Path) -> FileEvidence {
    let env = Env::new();
    let manager = FileRepositoryManager::new(env.clock(), env.ids());
    let handle = manager.open_blocking(path).unwrap();
    FileEvidence::open(
        FileStore::new(&handle, env.clock(), env.ids()),
        env.clock(),
        env.ids(),
    )
}

#[test]
fn no_grant_prints_a_plan_and_writes_no_evidence_even_when_quiet() {
    let sandbox = echo();
    let before = sandbox.read("events/current.log");
    let out = sandbox.run(
        &[
            "run",
            "start",
            "workflow",
            "--input",
            "data=input",
            "--quiet",
            "-o",
            "json",
        ],
        300,
    );
    exits(
        Out {
            code: out.code,
            stdout: out.stdout.clone(),
            stderr: out.stderr.clone(),
        },
        9,
    );
    assert_schema("run-plan", &out.json());
    assert_schema("error", &out.error_json());
    assert_eq!(out.json()["network"], "not_enforced");
    golden("cli_plan_denied", &out);
    insta::assert_snapshot!(
        "permission_denied_error",
        serde_json::to_string_pretty(&out.error_json()).unwrap()
    );
    assert_eq!(sandbox.read("events/current.log"), before);
    let list = sandbox.run(&["run", "list", "-o", "json"], 400).ok().json();
    assert_eq!(list["runs"], json!([]));
    let artifacts = sandbox
        .run(&["artifact", "list", "-o", "json"], 400)
        .ok()
        .json();
    assert_eq!(artifacts["artifacts"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn execution_show_and_list_cross_read_with_the_library_after_restart() {
    let sandbox = echo();
    let out = start(
        &sandbox,
        &["--request-id", "req-1", "--validate-outputs"],
        300,
    )
    .ok();
    let document = out.json();
    assert_schema("run-start", &document);
    assert_eq!(document["run"]["state"], "succeeded");
    assert_eq!(document["run"]["streams"]["stdout"], json!([111, 107, 10]));
    assert_eq!(document["validations"][0]["report"]["outcome"], "passed");
    assert!(document["validations"][0]["saved_report"].is_string());
    assert_eq!(out.stderr, "");
    let id = document["run"]["id"].as_str().unwrap();
    let show = sandbox
        .run(&["run", "show", id, "-o", "json"], 900)
        .ok()
        .json();
    assert_schema("run-show", &show);
    assert_eq!(show["run"], document["run"]);
    let list = sandbox.run(&["run", "list", "-o", "json"], 900).ok().json();
    assert_schema("run-list", &list);
    assert_eq!(list["runs"][0]["id"], id);
    assert!(list["runs"][0]["streams"].is_null());

    let evidence = evidence(sandbox.path());
    let record = evidence
        .run(&ArtifactRef::parse(id).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(record.intent().request_id.as_str(), "req-1");
    assert_eq!(record.result().unwrap().1.outputs().len(), 1);
    let saved = document["validations"][0]["saved_report"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(evidence.load_report(&saved).await.unwrap().is_some());

    let before = sandbox.read("events/current.log");
    let slug = ArtifactRepository::get(evidence.store(), record.run())
        .await
        .unwrap()
        .unwrap()
        .slug()
        .to_string();
    let text = sandbox.run(&["run", "show", &slug], 900).ok();
    assert!(text.stdout.contains("stdout: ok\\n"));
    assert_eq!(
        sandbox.read("events/current.log"),
        before,
        "inspection never emits events"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn phase_one_cli_acceptance_traces_claims_and_replays_captured_inputs() {
    let sandbox = echo();
    sandbox
        .run(
            &[
                "artifact",
                "create",
                "claim",
                "--type",
                "claim",
                "--content-text",
                "output echoes hello",
            ],
            250,
        )
        .ok();
    let first = start(&sandbox, &["--request-id", "acceptance-1"], 300)
        .ok()
        .json();
    assert_schema("run-start", &first);
    let run = first["run"]["id"].as_str().unwrap();
    let output = first["run"]["result"]["outputs"][0].as_str().unwrap();
    let capture = first["run"]["inputs"]["data"].as_str().unwrap();
    let workflow_capture = first["run"]["workflow_capture"].as_str().unwrap();
    let validated = sandbox
        .run(&["validate", output, "--save-report", "-o", "json"], 1000)
        .ok()
        .json();
    assert_schema("validate", &validated);
    assert_eq!(validated["outcome"], "passed");
    let saved = validated["saved_report"]["id"].as_str().unwrap();
    for (kind, seed) in [("supports", 1100), ("contradicts", 1200)] {
        sandbox
            .run(&["relation", "add", output, kind, "claim"], seed)
            .ok();
    }
    let contested = sandbox.run(&["validate", "-o", "json"], 1300).ok().json();
    assert_eq!(contested["outcome"], "warning");
    assert!(
        contested["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "V0023")
    );
    let upstream = sandbox
        .run(
            &[
                "query",
                "dependencies",
                saved,
                "--type",
                "validates",
                "--type",
                "derived_from",
                "--type",
                "depends_on",
                "--depth",
                "all",
                "-o",
                "json",
            ],
            1400,
        )
        .ok()
        .json();
    assert_schema("query-traversal", &upstream);
    let reached = |doc: &serde_json::Value, id: &str| {
        doc["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["artifact"]["id"] == id)
    };
    for id in [output, run, capture, workflow_capture] {
        assert!(reached(&upstream, id), "missing upstream {id}");
    }
    for slug in ["input", "workflow"] {
        assert!(
            upstream["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["artifact"]["slug"] == slug)
        );
    }
    let downstream = sandbox
        .run(
            &[
                "query",
                "dependents",
                "input",
                "--type",
                "validates",
                "--type",
                "derived_from",
                "--type",
                "depends_on",
                "--depth",
                "all",
                "-o",
                "json",
            ],
            1400,
        )
        .ok()
        .json();
    for id in [run, output, saved] {
        assert!(reached(&downstream, id), "missing downstream {id}");
    }
    sandbox
        .run(
            &["artifact", "update", "input", "--content-text", "changed"],
            1500,
        )
        .ok();
    let replay = sandbox
        .run(
            &[
                "run",
                "start",
                workflow_capture,
                "--input",
                &format!("data={capture}"),
                "--grant-exec",
                "--request-id",
                "acceptance-2",
                "-o",
                "json",
            ],
            2000,
        )
        .ok()
        .json();
    assert_eq!(replay["run"]["state"], "succeeded");
    assert_ne!(replay["run"]["id"], first["run"]["id"]);
    let replay_output = replay["run"]["result"]["outputs"][0].as_str().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let reopened = evidence(sandbox.path());
        let record = reopened.run(&ArtifactRef::parse(run).unwrap()).await.unwrap().unwrap();
        assert_eq!(record.result().unwrap().1.outcome(), RunOutcome::Succeeded);
        assert_eq!(reopened.read_capture(&record.intent().inputs["data"]).await.unwrap(), b"hello");
        for id in [output, replay_output] {
            let artifact = ArtifactRepository::get(reopened.store(), &id.parse().unwrap()).await.unwrap().unwrap();
            let Content::Path { path, .. } = artifact.content() else { panic!("file content") };
            assert_eq!(fs::read(sandbox.path().join(path.as_str())).unwrap(), b"out:hello");
        }
        let (report, subject) = reopened.load_report(&saved.parse().unwrap()).await.unwrap().unwrap();
        assert_eq!(report.outcome().as_str(), validated["outcome"].as_str().unwrap());
        assert!(matches!(subject, functor_core::ReportSubject::Artifact { id, digest: Some(_), .. } if id.to_string() == output));
        let snapshot = RepositoryReader::snapshot(reopened.store()).await.unwrap();
        let claim = snapshot.resolve(&ArtifactRef::parse("claim").unwrap()).unwrap();
        for kind in [RelationshipType::Supports, RelationshipType::Contradicts] {
            assert!(snapshot.incoming(claim.id()).any(|r| r.source().to_string() == output && r.relationship_type() == &kind));
        }
        let query = SnapshotQueryEngine::new(reopened.store());
        let found = query.traverse(&Traversal::new(ArtifactRef::parse("input").unwrap()).with_direction(Direction::Inbound)).await.unwrap();
        assert!(found.nodes.iter().any(|n| n.artifact.id().to_string() == saved));
    });
}

#[cfg(windows)]
#[test]
fn killing_the_cli_releases_the_lease_kills_descendants_and_recovers_without_rerunning() {
    use std::time::{Duration, Instant};

    struct Owner(std::process::Child);
    impl Drop for Owner {
        fn drop(&mut self) {
            if self.0.try_wait().unwrap().is_none() {
                self.0.kill().unwrap();
                self.0.wait().unwrap();
            }
        }
    }

    let sandbox = world(&["spawn-child", "unused", "60000"], false);
    let heartbeat = sandbox.path().join("heartbeat");
    let workflow = sandbox
        .read("workflow.toml")
        .replace("\"unused\"", &format!("{:?}", heartbeat.to_str().unwrap()));
    sandbox.write("workflow.toml", &workflow);
    let mut owner = Owner(sandbox.spawn(
        &[
            "run",
            "start",
            "workflow",
            "--input",
            "data=input",
            "--grant-exec",
            "--request-id",
            "killed",
            "-o",
            "json",
        ],
        300,
    ));
    let deadline = Instant::now() + Duration::from_secs(30);
    while !heartbeat.exists() {
        assert!(
            owner.0.try_wait().unwrap().is_none(),
            "CLI exited before child started"
        );
        assert!(Instant::now() < deadline, "child did not start");
        std::thread::sleep(Duration::from_millis(20));
    }
    let list = sandbox.run(&["run", "list", "-o", "json"], 900).ok().json();
    assert_eq!(list["runs"][0]["state"], "running");
    let run = list["runs"][0]["id"].as_str().unwrap();
    owner.0.kill().unwrap();
    owner.0.wait().unwrap();
    std::thread::sleep(Duration::from_millis(300));
    let stopped = fs::read(&heartbeat).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        fs::read(&heartbeat).unwrap(),
        stopped,
        "descendant survived its owner"
    );
    let show = sandbox
        .run(&["run", "show", run, "-o", "json"], 900)
        .ok()
        .json();
    assert_eq!(show["run"]["state"], "interrupted_unknown");
    assert!(show["run"]["result"].is_null());
    let repeated = exits(start(&sandbox, &["--request-id", "killed"], 1000), 8).json();
    assert_eq!(repeated["run"]["id"], run);
    assert!(!repeated["created"].as_bool().unwrap());
    let recovered = sandbox
        .run(&["run", "recover", "-o", "json"], 1100)
        .ok()
        .json();
    assert_schema("run-recover", &recovered);
    let show = sandbox
        .run(&["run", "show", run, "-o", "json"], 1200)
        .ok()
        .json();
    assert_eq!(show["run"]["state"], "interrupted");
    let log = sandbox.read("events/current.log");
    assert_eq!(log.matches("\"type\":\"run.started\"").count(), 1);
    assert_eq!(log.matches("\"type\":\"run.completed\"").count(), 1);
    assert_eq!(
        fs::read(heartbeat).unwrap(),
        stopped,
        "recovery must never rerun a child"
    );
}

#[test]
fn each_child_outcome_has_a_result_and_the_documented_exit_code() {
    for (args, output, expected) in [
        (vec!["exit", "0"], false, "succeeded"),
        (vec!["stderr-exit", "3"], false, "failed"),
        (vec!["exit", "0"], true, "failed"),
        (vec!["sleep", "60000"], false, "timed_out"),
    ] {
        let sandbox = world(&args, output);
        let extra = if expected == "timed_out" {
            vec!["--timeout", "1"]
        } else {
            vec![]
        };
        let out = exits(
            start(&sandbox, &extra, 300),
            if expected == "succeeded" { 0 } else { 8 },
        );
        assert_schema("run-start", &out.json());
        assert_eq!(out.json()["run"]["state"], expected);
        assert_eq!(out.json()["run"]["result"]["outcome"], expected);
        let name = if output && expected == "failed" {
            "cli_missing_output".to_owned()
        } else {
            format!("cli_{expected}")
        };
        golden(&name, &out);
        if expected != "succeeded" {
            assert_eq!(out.json()["run"]["result"]["outputs"], json!([]));
        }
    }
}

#[test]
fn a_missing_program_is_a_start_failed_outcome_not_an_infrastructure_error() {
    let sandbox = echo();
    fs::remove_file(
        sandbox
            .path()
            .join(format!("tools/fixture{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    let out = exits(start(&sandbox, &[], 300), 8);
    assert_schema("run-start", &out.json());
    assert_eq!(out.json()["run"]["state"], "start_failed");
    golden("cli_start_failed", &out);
}

#[test]
fn duplicate_requests_are_idempotent_and_omitted_ids_always_create_new_runs() {
    let sandbox = echo();
    let a = start(&sandbox, &["--request-id", "same"], 300).ok().json();
    let log = sandbox.read("events/current.log");
    let b = start(&sandbox, &["--request-id", "same"], 600).ok().json();
    assert_eq!(a["run"]["id"], b["run"]["id"]);
    assert_eq!(b["created"], false);
    assert_eq!(sandbox.read("events/current.log"), log);
    let c = start(&sandbox, &[], 900).ok().json();
    let d = start(&sandbox, &[], 1200).ok().json();
    assert_ne!(c["run"]["id"], d["run"]["id"]);
    assert_ne!(c["run"]["request_id"], d["run"]["request_id"]);
}

#[test]
fn invalid_bindings_provider_schema_and_timeouts_are_refused_before_a_run() {
    let sandbox = echo();
    for args in [
        vec!["--input", "data=input", "--input", "data=input"],
        vec!["--input", "stray=input"],
        vec!["--input", "broken"],
        vec!["--timeout", "0", "--input", "data=input"],
        vec!["--timeout", "3601", "--input", "data=input"],
    ] {
        let mut command = vec!["run", "start", "workflow", "--grant-exec"];
        command.extend(args);
        sandbox.run(&command, 300).fails(2);
    }
    for (from, to, code) in [
        ("local-process", "future-provider", 2),
        ("functor.workflow/1", "functor.workflow/2", 2),
    ] {
        let original = sandbox.read("workflow.toml");
        fs::write(
            sandbox.path().join("workflow.toml"),
            original.replace(from, to),
        )
        .unwrap();
        exits(start(&sandbox, &[], 300), code);
        fs::write(sandbox.path().join("workflow.toml"), original).unwrap();
    }
    let list = sandbox.run(&["run", "list", "-o", "json"], 900).ok().json();
    assert_eq!(list["runs"], json!([]));
    sandbox.run(&["run", "show", "input"], 900).fails(2);
    sandbox.run(&["run", "show", "missing"], 900).fails(3);
}

#[test]
fn failed_events_warn_but_do_not_change_the_run_outcome() {
    let sandbox = echo();
    let log = sandbox.path().join("events").join("current.log");
    fs::remove_file(&log).unwrap();
    fs::create_dir(&log).unwrap();
    let out = start(&sandbox, &[], 300).ok();
    assert_eq!(out.json()["run"]["state"], "succeeded");
    assert!(out.stderr.contains("warning: event not recorded"));
}

#[test]
fn mandatory_intent_storage_failure_never_reports_success_or_starts_the_child() {
    let sandbox = world(&["write", "outputs/result", "ran"], true);
    fs::create_dir_all(sandbox.path().join("content").join("evidence")).unwrap();
    fs::write(
        sandbox
            .path()
            .join("content")
            .join("evidence")
            .join("captures"),
        "blocked",
    )
    .unwrap();
    let out = start(&sandbox, &[], 300).fails(7);
    assert_schema("error", &out.error_json());
    assert!(!sandbox.read("events/current.log").contains("run.started"));
}

#[test]
fn failed_result_publication_keeps_the_run_id_and_never_emits_completion() {
    let sandbox = world(
        &["write", "../../../../content/evidence/results", "blocked"],
        false,
    );
    let out = start(&sandbox, &[], 300).fails(7);
    let error = out.error_json();
    assert_schema("error", &error);
    let id = error["run"].as_str().unwrap();
    let mut golden_error = error.clone();
    golden_error["error"]["message"] = json!("<storage diagnostic>");
    insta::assert_snapshot!(
        "result_publication_error",
        serde_json::to_string_pretty(&golden_error).unwrap()
    );
    let log = sandbox.read("events/current.log");
    assert!(log.contains("run.started"));
    assert!(!log.contains("run.completed"));
    let show = sandbox.run(&["run", "show", id, "-o", "json"], 900).ok();
    assert_schema("run-show", &show.json());
    assert_eq!(show.json()["run"]["state"], "interrupted_unknown");
    fs::remove_file(
        sandbox
            .path()
            .join("content")
            .join("evidence")
            .join("results"),
    )
    .unwrap();
    let repaired = sandbox.run(&["run", "recover", "-o", "json"], 1000).ok();
    assert_schema("run-recover", &repaired.json());
    assert_eq!(repaired.json()["runs"][0]["state"], "interrupted");
}

#[test]
fn saved_validation_is_opt_in_and_failure_preserves_the_computed_report() {
    let sandbox = echo();
    let plain = sandbox
        .run(&["validate", "input", "-o", "json"], 300)
        .ok()
        .json();
    assert!(plain.get("saved_report").is_none());
    let saved = sandbox
        .run(&["validate", "input", "--save-report", "-o", "json"], 400)
        .ok()
        .json();
    assert_schema("validate", &saved);
    let mut without_save = saved.clone();
    without_save.as_object_mut().unwrap().remove("saved_report");
    assert_eq!(plain, without_save);
    assert!(saved["saved_report"]["id"].is_string());
    insta::assert_snapshot!(
        "saved_validation_json",
        serde_json::to_string_pretty(&saved).unwrap()
    );
    let report = sandbox
        .run(
            &[
                "artifact",
                "show",
                saved["saved_report"]["id"].as_str().unwrap(),
                "-o",
                "json",
            ],
            600,
        )
        .ok()
        .json();
    assert_eq!(report["artifact"]["metadata"]["subject_revision"], 1);
    assert!(report["artifact"]["metadata"]["subject_sha256"].is_string());

    let blocked = echo();
    fs::create_dir_all(blocked.path().join("content").join("evidence")).unwrap();
    fs::write(
        blocked
            .path()
            .join("content")
            .join("evidence")
            .join("reports"),
        "blocked",
    )
    .unwrap();
    let out = exits(
        blocked.run(&["validate", "--save-report", "-o", "json"], 300),
        7,
    );
    assert_schema("validate", &out.json());
    assert_schema("error", &out.error_json());
    assert_eq!(out.json()["outcome"], "passed");
    assert!(out.json()["saved_report"]["id"].is_null());
    assert!(out.json()["saved_report"]["error"].is_string());
}

#[test]
fn saved_reports_keep_validation_outcomes_and_subjects_separate_from_persistence() {
    let sandbox = echo();
    sandbox
        .run(
            &[
                "relation",
                "add",
                "workflow",
                "related_to",
                "input",
                "--allow-custom-type",
            ],
            300,
        )
        .ok();
    let out = exits(
        sandbox.run(
            &["validate", "--strict", "--save-report", "-o", "json"],
            500,
        ),
        6,
    );
    assert_schema("validate", &out.json());
    assert_eq!(out.json()["outcome"], "warning");
    assert!(out.json()["saved_report"]["id"].is_string());
    assert_eq!(out.stderr, "");
    let report = sandbox
        .run(
            &[
                "artifact",
                "show",
                out.json()["saved_report"]["id"].as_str().unwrap(),
                "-o",
                "json",
            ],
            700,
        )
        .ok()
        .json();
    assert_eq!(report["artifact"]["metadata"]["subject"], "repository");
    let text = sandbox
        .run(&["validate", "input", "--save-report"], 800)
        .ok();
    assert!(text.stdout.contains("saved_report: art_"));
    let log = sandbox.read("events/current.log");
    assert_eq!(log.matches("\"type\":\"validation.completed\"").count(), 2);
    assert_eq!(log.matches("\"type\":\"validation.recorded\"").count(), 2);
}

#[test]
fn a_damaged_repository_still_prints_a_report_when_saving_is_refused() {
    let sandbox = echo();
    fs::write(
        sandbox.path().join(".functor").join("repository.toml"),
        "not toml =",
    )
    .unwrap();
    let out = exits(
        sandbox.run(&["validate", "--save-report", "-o", "json"], 300),
        7,
    );
    assert_schema("validate", &out.json());
    assert_eq!(out.json()["outcome"], "failed");
    assert!(out.json()["saved_report"]["error"].is_string());
}

#[tokio::test]
async fn library_written_interrupted_evidence_is_inspected_and_recovered_by_cli() {
    let sandbox = echo();
    let env = Env::new();
    let evidence = evidence(sandbox.path());
    let workflow = evidence
        .capture(
            &ArtifactRef::parse("workflow").unwrap(),
            functor_model::CaptureRole::Workflow,
        )
        .await
        .unwrap();
    let input = evidence
        .capture(
            &ArtifactRef::parse("input").unwrap(),
            functor_model::CaptureRole::Input,
        )
        .await
        .unwrap();
    let intent = functor_testkit::sample_intent(
        "library-run",
        workflow.capture.artifact,
        &[("data", input.capture.artifact)],
        env.clock().now(),
    );
    let recorded = evidence.record_intent(intent).await.unwrap();
    let id = recorded.record.run().to_string();
    let show = sandbox
        .run(&["run", "show", &id, "-o", "json"], 1000)
        .ok()
        .json();
    assert_schema("run-show", &show);
    assert_eq!(show["run"]["state"], "interrupted_unknown");
    let lease = evidence.acquire_lease(recorded.record.run()).await.unwrap();
    let running = sandbox
        .run(&["run", "show", &id, "-o", "json"], 1000)
        .ok()
        .json();
    assert_schema("run-show", &running);
    assert_eq!(running["run"]["state"], "running");
    drop(lease);
    let out = sandbox
        .run(&["run", "recover", "-o", "json"], 1100)
        .ok()
        .json();
    assert_schema("run-recover", &out);
    assert_eq!(out["runs"][0]["state"], "interrupted");
    let second = sandbox
        .run(&["run", "recover", "-o", "json"], 1200)
        .ok()
        .json();
    assert_eq!(second["runs"], json!([]));
}

#[tokio::test]
async fn a_library_cancelled_run_has_the_same_cli_schema_and_preserves_binary_streams() {
    let sandbox = echo();
    let env = Env::new();
    let evidence = evidence(sandbox.path());
    let workflow = evidence
        .capture(
            &ArtifactRef::parse("workflow").unwrap(),
            functor_model::CaptureRole::Workflow,
        )
        .await
        .unwrap();
    let input = evidence
        .capture(
            &ArtifactRef::parse("input").unwrap(),
            functor_model::CaptureRole::Input,
        )
        .await
        .unwrap();
    let intent = functor_testkit::sample_intent(
        "cancelled-run",
        workflow.capture.artifact,
        &[("data", input.capture.artifact)],
        env.clock().now(),
    );
    let run = evidence.record_intent(intent).await.unwrap().record;
    evidence
        .record_result(
            run.run(),
            functor_core::RunFinish {
                outcome: functor_model::RunOutcome::Cancelled,
                exit_code: None,
                truncated: functor_model::Truncation {
                    stdout: true,
                    stderr: false,
                },
                diagnostics: Vec::new(),
                stdout: vec![0, 255, 10],
                stderr: Vec::new(),
                outputs: Vec::new(),
            },
        )
        .await
        .unwrap();
    let out = sandbox
        .run(&["run", "show", &run.run().to_string(), "-o", "json"], 1000)
        .ok();
    assert_schema("run-show", &out.json());
    assert_eq!(out.json()["run"]["state"], "cancelled");
    assert_eq!(out.json()["run"]["streams"]["stdout"], json!([0, 255, 10]));
    golden("cli_cancelled", &out);
}

#[test]
fn watch_observes_run_and_validation_events_without_writing() {
    let sandbox = echo();
    start(&sandbox, &["--validate-outputs"], 300).ok();
    let before = sandbox.read("events/current.log");
    let events = sandbox
        .run(
            &[
                "watch",
                "--from-start",
                "--type",
                "run.",
                "--count",
                "2",
                "--timeout",
                "2",
            ],
            900,
        )
        .ok();
    assert!(events.stdout.contains("run.started"));
    assert!(events.stdout.contains("run.completed"));
    assert!(events.stdout.contains("succeeded"));
    let events = sandbox
        .run(
            &[
                "watch",
                "--from-start",
                "--type",
                "validation.recorded",
                "--count",
                "1",
                "--timeout",
                "2",
                "-o",
                "json",
            ],
            900,
        )
        .ok();
    assert_eq!(events.json()["type"], "validation.recorded");
    assert_eq!(sandbox.read("events/current.log"), before);
}

#[test]
fn schemas_reject_invalid_outcomes_and_missing_run_identity() {
    let sandbox = echo();
    let document = start(&sandbox, &[], 300).ok().json();
    let mut invalid = document.clone();
    invalid["run"]["state"] = json!("pretend_success");
    assert_schema_rejects("run-start", &invalid);
    let mut invalid = document.clone();
    invalid["run"].as_object_mut().unwrap().remove("id");
    assert_schema_rejects("run-start", &invalid);
    let mut invalid = document;
    invalid["run"]["streams"]["stdout"] = json!([256]);
    assert_schema_rejects("run-start", &invalid);
    let invalid_saved = json!({
        "schema_version": 1, "outcome": "passed", "strict": false,
        "counts": { "artifacts": 0, "relationships": 0, "errors": 0, "warnings": 0, "infos": 0 },
        "findings": [], "saved_report": { "id": null, "error": null }
    });
    assert_schema_rejects("validate", &invalid_saved);
}
