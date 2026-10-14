//! Tests of the run orchestrator with a fake provider and in-memory evidence: the integrated
//! workflow, every failure path, events, output validation and the evidence rules.

// Tests assert; a failing unwrap is the failure.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::items_after_statements
)]

mod support;

use std::sync::atomic::Ordering;

use functor_core::{
    ArtifactRepository, CancelToken, ErrorKind, EvidenceRecorder, ExecutionGrant,
    RelationshipRepository, RepositoryReader, ValidationEngine, ValidationTarget,
};
use functor_model::{
    ArtifactId, ArtifactRef, EventKind, EvidenceKind, ExecutionLimits, MetadataValue,
    NewRelationship, Outcome, RelationshipType, RequestId, RunOutcome, RunStatus,
};
use functor_runtime::OutputChecks;
use functor_validation::RuleValidationEngine;
use support::{Fast, StubEngine, WORKFLOW, request};

fn kinds(events: &[functor_model::NewEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.kind.as_str()).collect()
}

fn text(event: &functor_model::NewEvent, key: &str) -> String {
    match event.data.get(key) {
        Some(MetadataValue::String(text)) => text.clone(),
        other => panic!("`{key}` is not a string: {other:?}"),
    }
}

#[tokio::test]
async fn a_successful_run_leaves_traceable_evidence_and_two_events() {
    let fast = Fast::new().await;
    let report = fast.run(&request("req-1"), fast.grant()).await.unwrap();
    assert!(report.created);
    assert!(report.warnings.is_empty());
    let (result_id, result) = report.record.result().unwrap().clone();
    assert_eq!(result.outcome(), RunOutcome::Succeeded);
    assert_eq!(result.outputs().len(), 1);

    // The output is an ordinary artifact of the declared type.
    let output = ArtifactRepository::get(fast.store(), &result.outputs()[0])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(output.artifact_type().as_str(), "dataset");

    // Exactly two events, in order, from the one owner; nothing says `artifact.created`.
    let events = fast.events.taken();
    assert_eq!(kinds(&events), ["run.started", "run.completed"]);
    let run = *report.record.run();
    assert_eq!(
        events[0].subject,
        functor_model::EventSubject::Artifact(run)
    );
    assert_eq!(text(&events[0], "provider"), "fake");
    assert_eq!(text(&events[0], "request_id"), "req-1");
    assert_eq!(text(&events[0], "workflow"), fast.workflow.to_string());
    assert_eq!(text(&events[1], "outcome"), "succeeded");
    assert_eq!(text(&events[1], "result"), result_id.to_string());
    assert_eq!(
        events[1].data.get("outputs"),
        Some(&MetadataValue::StringList(vec![output.id().to_string()]))
    );
    assert_eq!(
        events[1].data.get("truncated"),
        Some(&MetadataValue::Bool(false))
    );

    // Staging of a successful run is removed; provider and recorder agree on the run.
    assert_eq!(fast.provider.discards.load(Ordering::SeqCst), 1);
    assert_eq!(fast.provider.inner.started(), 1);
    assert_eq!(
        fast.evidence.status(&run).await.unwrap(),
        RunStatus::Completed(RunOutcome::Succeeded)
    );
}

#[tokio::test]
async fn evidence_is_navigable_from_a_conclusion_to_the_inputs_and_back() {
    let fast = Fast::new().await;
    let report = fast.run(&request("req-1"), fast.grant()).await.unwrap();
    let output = report.record.result().unwrap().1.outputs()[0];
    let run = *report.record.run();

    let snapshot = RepositoryReader::snapshot(fast.store()).await.unwrap();
    let reach = |start: ArtifactId, forward: bool| {
        let mut seen = std::collections::HashSet::new();
        let mut todo = vec![start];
        while let Some(id) = todo.pop() {
            if seen.insert(id) {
                if forward {
                    todo.extend(snapshot.outgoing(&id).map(|r| *r.target()));
                } else {
                    todo.extend(snapshot.incoming(&id).map(|r| *r.source()));
                }
            }
        }
        seen
    };
    let upstream = reach(output, true);
    for expected in [run, fast.workflow, fast.input] {
        assert!(
            upstream.contains(&expected),
            "{expected} is upstream of the output"
        );
    }
    let downstream = reach(fast.input, false);
    assert!(downstream.contains(&output) && downstream.contains(&run));

    // The query engine sees the same graph, live.
    let engine = functor_query::SnapshotQueryEngine::new(fast.store());
    let (_, around_run) = engine
        .relationships(
            &ArtifactRef::Id(run),
            &[RelationshipType::DependsOn],
            functor_model::Direction::Outbound,
        )
        .await
        .unwrap();
    assert_eq!(around_run.len(), 2, "the run depends on its two captures");
}

#[tokio::test]
async fn every_child_outcome_is_recorded_not_raised() {
    for (mode, expected, kept) in [
        ("exit-nonzero", RunOutcome::Failed, true),
        ("no-output", RunOutcome::Failed, true),
        ("timeout", RunOutcome::TimedOut, true),
        ("missing-program", RunOutcome::StartFailed, true),
    ] {
        let fast = Fast::with_workflow(&WORKFLOW.replace("succeed", mode)).await;
        let report = fast.run(&request("req-1"), fast.grant()).await.unwrap();
        let result = &report.record.result().unwrap().1;
        assert_eq!(result.outcome(), expected, "{mode}");
        assert_eq!(result.outputs().len(), 0, "{mode}: nothing is published");
        let events = fast.events.taken();
        assert_eq!(kinds(&events), ["run.started", "run.completed"], "{mode}");
        assert_eq!(text(&events[1], "outcome"), expected.as_str(), "{mode}");
        assert_eq!(
            fast.provider.discards.load(Ordering::SeqCst),
            usize::from(!kept),
            "{mode}: staging of a failed run is kept for diagnosis"
        );
    }
}

#[tokio::test]
async fn a_cancelled_token_is_recorded_as_a_cancelled_run() {
    let fast = Fast::new().await;
    let token = CancelToken::new();
    token.cancel();
    let report = fast
        .orchestrator
        .execute(&request("req-1"), fast.grant(), token, None)
        .await
        .unwrap();
    assert_eq!(
        report.record.result().unwrap().1.outcome(),
        RunOutcome::Cancelled
    );
}

#[tokio::test]
async fn refusals_happen_before_any_run_exists() {
    // No child, no run, no event, whatever the reason.
    for (name, workflow, expected) in [
        (
            "grant without the allowed variable",
            WORKFLOW.to_owned(),
            ErrorKind::PermissionDenied,
        ),
        (
            "another provider",
            WORKFLOW.replace("provider = \"fake\"", "provider = \"remote\""),
            ErrorKind::Unsupported,
        ),
        (
            "newer schema",
            WORKFLOW.replace("workflow/1", "workflow/2"),
            ErrorKind::Unsupported,
        ),
        (
            "network none",
            format!("network = \"none\"\n{WORKFLOW}"),
            ErrorKind::Unsupported,
        ),
        (
            "not toml",
            "this is = = not toml".to_owned(),
            ErrorKind::InvalidInput,
        ),
        (
            "unknown key",
            format!("surprise = 1\n{WORKFLOW}"),
            ErrorKind::InvalidInput,
        ),
    ] {
        let fast = Fast::with_workflow(&workflow).await;
        let grant = if expected == ErrorKind::PermissionDenied {
            ExecutionGrant::new("fake-tool", ExecutionLimits::DEFAULT)
        } else {
            fast.grant()
        };
        let failure = fast.run(&request("req-1"), grant).await.unwrap_err();
        assert_eq!(failure.kind(), expected, "{name}: {failure}");
        assert_eq!(failure.run, None, "{name}: no run exists");
        assert_eq!(fast.evidence.runs().await.unwrap().len(), 0, "{name}");
        assert_eq!(fast.provider.inner.started(), 0, "{name}");
        assert_eq!(kinds(&fast.events.taken()), Vec::<&str>::new(), "{name}");
    }
}

#[tokio::test]
async fn bad_bindings_and_limits_above_the_grant_are_refused_before_any_input_is_captured() {
    let fast = Fast::new().await;
    async fn captures(fast: &Fast) -> usize {
        ArtifactRepository::list(fast.store())
            .await
            .unwrap()
            .iter()
            .filter(|a| EvidenceKind::of(a) == Some(EvidenceKind::Capture))
            .count()
    }
    // Unbound required input, unknown input, duplicate binding (rejected by the request itself).
    let unbound = functor_model::RunRequest::new(
        ArtifactRef::Id(fast.workflow),
        vec![],
        RequestId::new("req-1").unwrap(),
    )
    .unwrap();
    let failure = fast.run(&unbound, fast.grant()).await.unwrap_err();
    assert_eq!(failure.kind(), ErrorKind::InvalidInput);
    let unknown = functor_model::RunRequest::new(
        ArtifactRef::Id(fast.workflow),
        vec![functor_model::InputBinding {
            name: "stray".to_owned(),
            artifact: ArtifactRef::Id(fast.input),
        }],
        RequestId::new("req-2").unwrap(),
    )
    .unwrap();
    assert_eq!(
        fast.run(&unknown, fast.grant()).await.unwrap_err().kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        captures(&fast).await,
        2,
        "only the two workflow captures exist"
    );

    // A request asking for more than the grant allows.
    let greedy = request("req-3").with_limits(ExecutionLimits::MAX);
    let small =
        ExecutionGrant::new("fake-tool", ExecutionLimits::new(10, 1024).unwrap()).with_env("LANG");
    let greedy_workflow = Fast::with_workflow(
        &WORKFLOW.replace("[inputs]", "[limits]\ntimeout_seconds = 60\n\n[inputs]"),
    )
    .await;
    assert_eq!(
        greedy_workflow
            .run(&greedy, small)
            .await
            .unwrap_err()
            .kind(),
        ErrorKind::PermissionDenied
    );
    assert_eq!(greedy_workflow.evidence.runs().await.unwrap().len(), 0);
}

#[tokio::test]
async fn a_repeated_request_id_returns_the_existing_run_and_starts_nothing() {
    let fast = Fast::new().await;
    let first = fast.run(&request("same"), fast.grant()).await.unwrap();
    let events_after_first = fast.events.taken().len();
    let again = fast.run(&request("same"), fast.grant()).await.unwrap();
    assert!(!again.created);
    assert_eq!(again.record.run(), first.record.run());
    assert_eq!(again.record, first.record, "the stored result is returned");
    assert_eq!(fast.provider.inner.started(), 1, "the child ran once");
    assert_eq!(
        fast.events.taken().len(),
        events_after_first,
        "no new events"
    );
    assert_eq!(fast.evidence.runs().await.unwrap().len(), 1);

    // A different request id is a different run.
    let other = fast.run(&request("other"), fast.grant()).await.unwrap();
    assert_ne!(other.record.run(), first.record.run());
    assert_eq!(fast.provider.inner.started(), 2);
}

#[tokio::test]
async fn a_failed_event_append_is_a_warning_and_the_evidence_is_complete() {
    let fast = Fast::new().await;
    fast.events.fail.store(true, Ordering::SeqCst);
    let report = fast.run(&request("req-1"), fast.grant()).await.unwrap();
    assert_eq!(report.warnings.len(), 2, "{:?}", report.warnings);
    assert!(
        report
            .warnings
            .iter()
            .all(|w| w.starts_with("event not recorded"))
    );
    assert_eq!(
        report.record.result().unwrap().1.outcome(),
        RunOutcome::Succeeded
    );
    // Evidence inspection never needed the events.
    assert_eq!(fast.evidence.runs().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_failed_result_write_is_an_error_without_a_completion_event_and_recovers() {
    let fast = Fast::new().await;
    fast.recorder.fail_result.store(true, Ordering::SeqCst);
    let failure = fast.run(&request("req-1"), fast.grant()).await.unwrap_err();
    assert_eq!(failure.kind(), ErrorKind::Storage);
    let run = failure.run.expect("the intent was recorded");
    assert_eq!(
        kinds(&fast.events.taken()),
        ["run.started"],
        "no success notification for a write that failed"
    );
    assert_eq!(
        fast.evidence.status(&run).await.unwrap(),
        RunStatus::InterruptedUnknown,
        "the run is incomplete, not completed"
    );

    // Recovery completes it as interrupted and announces that; the child is not rerun.
    let recovered = fast.orchestrator.recover().await.unwrap();
    assert_eq!(recovered.runs.len(), 1);
    assert_eq!(fast.provider.inner.started(), 1);
    let events = fast.events.taken();
    assert_eq!(kinds(&events), ["run.started", "run.completed"]);
    assert_eq!(text(&events[1], "outcome"), "interrupted");
    assert_eq!(fast.orchestrator.recover().await.unwrap().runs.len(), 0);
}

#[tokio::test]
async fn a_failed_intent_write_means_no_child_starts_and_no_started_event() {
    let fast = Fast::new().await;
    fast.recorder.fail_intent.store(true, Ordering::SeqCst);
    let failure = fast.run(&request("req-1"), fast.grant()).await.unwrap_err();
    assert_eq!(failure.kind(), ErrorKind::Storage);
    assert_eq!(failure.run, None);
    assert_eq!(fast.provider.inner.started(), 0);
    assert_eq!(kinds(&fast.events.taken()), Vec::<&str>::new());
}

#[tokio::test]
async fn a_provider_fault_after_the_intent_is_recorded_as_a_start_failure_and_returned() {
    let fast = Fast::new().await;
    fast.provider.fail_execute.store(true, Ordering::SeqCst);
    let failure = fast.run(&request("req-1"), fast.grant()).await.unwrap_err();
    assert_eq!(failure.kind(), ErrorKind::Storage);
    let run = failure.run.unwrap();
    assert_eq!(
        fast.evidence.status(&run).await.unwrap(),
        RunStatus::Completed(RunOutcome::StartFailed),
        "the run is not left looking interrupted"
    );
    let events = fast.events.taken();
    assert_eq!(kinds(&events), ["run.started", "run.completed"]);
    assert_eq!(text(&events[1], "outcome"), "start_failed");
}

#[tokio::test]
async fn output_validation_persists_a_report_per_output_without_changing_the_run() {
    for (outcome, expected) in [
        (Outcome::Passed, Outcome::Passed),
        (Outcome::Warning, Outcome::Warning),
        (Outcome::Failed, Outcome::Failed),
        (Outcome::Unknown, Outcome::Unknown),
    ] {
        let fast = Fast::new().await;
        let engine = StubEngine::new(outcome);
        let checks = OutputChecks {
            engine: &engine,
            artifacts: fast.store(),
            rules: "V0001-V0023",
        };
        let report = fast
            .orchestrator
            .execute(
                &request("req-1").with_output_validation(),
                fast.grant(),
                CancelToken::new(),
                Some(checks),
            )
            .await
            .unwrap();
        assert_eq!(
            report.record.result().unwrap().1.outcome(),
            RunOutcome::Succeeded,
            "a {outcome} validation never changes the run"
        );
        assert_eq!(report.validations.len(), 1);
        let validation = &report.validations[0];
        assert_eq!(validation.report.outcome(), expected);
        let saved = *validation.saved.as_ref().unwrap();

        // Only the output was validated, never the evidence around it.
        assert_eq!(engine.targets(), [ArtifactRef::Id(validation.output)]);
        let (stored, subject) = fast.report(&saved).await;
        assert_eq!(stored.outcome(), expected);
        assert!(matches!(
            subject,
            functor_core::ReportSubject::Artifact { id, digest: Some(_), .. } if id == validation.output
        ));
        let events = fast.events.taken();
        assert_eq!(
            kinds(&events),
            ["run.started", "run.completed", "validation.recorded"]
        );
        assert_eq!(text(&events[2], "outcome"), expected.as_str());
        assert_eq!(text(&events[2], "subject"), validation.output.to_string());
    }
}

#[tokio::test]
async fn reports_are_never_validated_in_turn_and_validate_alone_stays_read_only() {
    let fast = Fast::new().await;
    let engine = StubEngine::new(Outcome::Passed);
    let checks = OutputChecks {
        engine: &engine,
        artifacts: fast.store(),
        rules: "V0001-V0023",
    };
    for request_id in ["req-1", "req-2"] {
        fast.orchestrator
            .execute(
                &request(request_id).with_output_validation(),
                fast.grant(),
                CancelToken::new(),
                Some(checks),
            )
            .await
            .unwrap();
    }
    let all = ArtifactRepository::list(fast.store()).await.unwrap();
    let reports: Vec<ArtifactId> = all
        .iter()
        .filter(|a| EvidenceKind::of(a) == Some(EvidenceKind::ValidationReport))
        .map(|a| *a.id())
        .collect();
    assert_eq!(
        reports.len(),
        2,
        "one report per output, none about reports"
    );
    let outputs: Vec<ArtifactRef> = engine.targets();
    assert_eq!(outputs.len(), 2);
    for target in outputs {
        let ArtifactRef::Id(id) = target else {
            panic!()
        };
        assert!(
            !reports.contains(&id),
            "a report was validated as a subject"
        );
    }

    // An ordinary validation of the repository does not write anything.
    let before = (
        ArtifactRepository::list(fast.store()).await.unwrap(),
        RelationshipRepository::list(fast.store()).await.unwrap(),
    );
    let real = RuleValidationEngine::new(fast.store(), fast.clock());
    let report = real.validate(ValidationTarget::Repository).await.unwrap();
    assert_eq!(report.outcome(), Outcome::Passed, "{:?}", report.findings());
    let after = (
        ArtifactRepository::list(fast.store()).await.unwrap(),
        RelationshipRepository::list(fast.store()).await.unwrap(),
    );
    assert_eq!(before, after, "validate is read-only");
}

#[tokio::test]
async fn a_report_that_cannot_be_saved_is_still_returned_and_the_run_stands() {
    let fast = Fast::new().await;
    fast.recorder.fail_reports.store(true, Ordering::SeqCst);
    let engine = StubEngine::new(Outcome::Failed);
    let checks = OutputChecks {
        engine: &engine,
        artifacts: fast.store(),
        rules: "V0001-V0023",
    };
    let report = fast
        .orchestrator
        .execute(
            &request("req-1").with_output_validation(),
            fast.grant(),
            CancelToken::new(),
            Some(checks),
        )
        .await
        .unwrap();
    assert_eq!(
        report.record.result().unwrap().1.outcome(),
        RunOutcome::Succeeded
    );
    let validation = &report.validations[0];
    assert_eq!(validation.report.outcome(), Outcome::Failed);
    assert_eq!(
        validation.saved.as_ref().unwrap_err().kind(),
        ErrorKind::Storage
    );
    assert_eq!(
        kinds(&fast.events.taken()),
        ["run.started", "run.completed"],
        "no validation.recorded for a report that was not saved"
    );
}

#[tokio::test]
async fn asking_for_output_validation_without_an_engine_is_refused_up_front() {
    let fast = Fast::new().await;
    let before = ArtifactRepository::list(fast.store()).await.unwrap().len();
    let failure = fast
        .run(&request("req-1").with_output_validation(), fast.grant())
        .await
        .unwrap_err();
    assert_eq!(failure.kind(), ErrorKind::InvalidInput);
    assert_eq!(
        ArtifactRepository::list(fast.store()).await.unwrap().len(),
        before,
        "nothing was written"
    );
}

#[tokio::test]
async fn a_caller_computed_report_is_recorded_with_its_event() {
    let fast = Fast::new().await;
    let real = RuleValidationEngine::new(fast.store(), fast.clock());
    let report = real
        .validate(ValidationTarget::Artifact(ArtifactRef::Id(fast.input)))
        .await
        .unwrap();
    let artifact = ArtifactRepository::get(fast.store(), &fast.input)
        .await
        .unwrap()
        .unwrap();
    let saved = fast
        .orchestrator
        .record_validation(
            &report,
            functor_core::ReportSubject::Artifact {
                id: fast.input,
                revision: artifact.revision(),
                digest: None,
            },
            true,
            "V0001-V0023",
        )
        .await
        .unwrap();
    let events = fast.events.taken();
    assert_eq!(kinds(&events), ["validation.recorded"]);
    assert_eq!(
        events[0].subject,
        functor_model::EventSubject::Artifact(saved.id)
    );
    assert_eq!(events[0].kind, EventKind::ValidationRecorded);
    let (back, _) = fast.report(&saved.id).await;
    assert_eq!(back, report, "the report is stored exactly as computed");

    // A subject that does not exist is refused and nothing is announced.
    let error = fast
        .orchestrator
        .record_validation(
            &report,
            functor_core::ReportSubject::Artifact {
                id: ArtifactId::from_bits(9999),
                revision: artifact.revision(),
                digest: None,
            },
            false,
            "V0001-V0023",
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
    assert_eq!(fast.events.taken().len(), 1);
}

#[tokio::test]
async fn evidence_assertions_have_a_direction_and_the_rules_see_conflicts_and_gaps() {
    let fast = Fast::new().await;
    let report = fast.run(&request("req-1"), fast.grant()).await.unwrap();
    let output = report.record.result().unwrap().1.outputs()[0];
    let claim = fast.source("claim", "text", "the claim").await;
    let rival = fast.source("rival", "text", "another source").await;

    let engine = RuleValidationEngine::new(fast.store(), fast.clock());
    let codes = |report: &functor_model::ValidationReport| -> Vec<&'static str> {
        report.findings().iter().map(|f| f.code.as_str()).collect()
    };
    let healthy = engine.validate(ValidationTarget::Repository).await.unwrap();
    assert_eq!(
        codes(&healthy),
        Vec::<&str>::new(),
        "a healthy run has no findings"
    );

    // The output supports the claim; a rival source contradicts it. Direction is source to target.
    for (source, kind, target) in [
        (output, RelationshipType::Supports, claim),
        (rival, RelationshipType::Contradicts, claim),
    ] {
        RelationshipRepository::create(fast.store(), NewRelationship::new(kind, source, target))
            .await
            .unwrap();
    }
    let snapshot = RepositoryReader::snapshot(fast.store()).await.unwrap();
    assert_eq!(
        snapshot.outgoing(&output).count(),
        2,
        "derived_from plus supports"
    );
    assert_eq!(snapshot.incoming(&claim).count(), 2);
    assert_eq!(
        codes(&engine.validate(ValidationTarget::Repository).await.unwrap()),
        Vec::<&str>::new(),
        "different sources may disagree"
    );

    // The same source asserting both is flagged.
    RelationshipRepository::create(
        fast.store(),
        NewRelationship::new(RelationshipType::Contradicts, output, claim),
    )
    .await
    .unwrap();
    assert_eq!(
        codes(&engine.validate(ValidationTarget::Repository).await.unwrap()),
        ["V0023"]
    );
}

#[tokio::test]
async fn interrupted_and_orphaned_work_is_visible_to_validation() {
    let fast = Fast::new().await;
    fast.recorder.fail_result.store(true, Ordering::SeqCst);
    fast.run(&request("req-1"), fast.grant()).await.unwrap_err();
    let engine = RuleValidationEngine::new(fast.store(), fast.clock());
    let report = engine.validate(ValidationTarget::Repository).await.unwrap();
    let codes: Vec<&str> = report.findings().iter().map(|f| f.code.as_str()).collect();
    assert!(codes.contains(&"V0020"), "a run with no result: {codes:?}");
    assert_eq!(report.outcome(), Outcome::Warning);

    fast.orchestrator.recover().await.unwrap();
    let report = engine.validate(ValidationTarget::Repository).await.unwrap();
    assert!(
        report.findings().iter().all(|f| f.code.as_str() != "V0020"),
        "recovery completed the run"
    );
}

#[tokio::test]
async fn provenance_records_names_never_values() {
    let fast = Fast::new().await;
    let report = fast.run(&request("req-1"), fast.grant()).await.unwrap();
    let intent = report.record.intent();
    assert_eq!(intent.env_names, ["LANG"]);
    assert_eq!(intent.provider, "fake");
    assert_eq!(intent.args, ["succeed"]);
    assert_eq!(
        intent.platform,
        format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
    );
}
