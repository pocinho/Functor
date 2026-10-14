//! Functor test support: test doubles, fixtures and the conformance suites.
//!
//! See `docs/quality/testing-strategy.md`. Used only as a dev-dependency of other crates.
//!
//! - [`FixedClock`] and [`SequentialIds`] make time and identifiers deterministic.
//! - [`RepoFixture`] builds small in-memory repositories.
//! - [`run_store_conformance`] is the behavioral contract every store implementation must pass.
//! - [`run_provider_conformance`] and [`FakeProvider`]: the contract of an execution provider
//!   (ADR-0012), with a scriptable in-process fake.
//! - [`run_evidence_conformance`] and [`MemoryEvidence`]: the contract of the evidence and report
//!   recorders (ADR-0013, ADR-0015), with an in-memory fake.

#[cfg(test)]
mod block_on;
mod conformance;
mod doubles;
mod evidence;
mod execution;
mod fixture;

#[cfg(test)]
mod reference_store;

pub use conformance::{
    Ctx, Env, Fresh, StoreUnderTest, create_get_resolve_and_list, determinism,
    duplicate_slug_is_a_conflict, expected_revision_is_enforced, info_is_stable,
    load_of_a_clean_repository_has_no_issues, no_op_update_changes_nothing,
    relationship_get_and_list_order, relationship_rules, relationships_require_existing_endpoints,
    rename_to_a_taken_slug_conflicts, run_store_conformance,
    slug_rename_releases_the_old_slug_and_keeps_relationships, snapshot_reflects_state,
    update_increments_revision_and_preserves_identity, update_of_unknown_artifact_is_not_found,
};
pub use doubles::{FixedClock, SequentialIds};
pub use evidence::{
    EvidenceUnderTest, MemoryEvidence, a_capture_records_the_exact_state,
    a_capture_survives_later_edits, an_inconsistent_finish_writes_nothing,
    capturing_an_unknown_artifact_is_not_found, damaged_captures_are_detected,
    failures_are_recorded_as_outcomes, fake_digest, intent_is_idempotent_by_request_id,
    intent_needs_existing_captures, leases_are_exclusive_and_released_on_drop,
    recovery_completes_only_orphans, reports_need_an_existing_subject,
    reports_round_trip_and_stay_immutable, run_evidence_conformance, runs_are_listed_in_id_order,
    sample_intent, sample_success, status_follows_the_records, the_result_is_the_commit_point,
};
pub use execution::{
    FAKE_PROVIDER, FakeProvider, ProviderScenario, ProviderUnderTest, cancellation_is_an_outcome,
    failures_are_outcomes_not_errors, fake_plan, output_is_bounded_and_truncation_is_reported,
    refusals_happen_before_any_side_effect, replay_is_deterministic, run_provider_conformance,
    success_produces_declared_outputs,
};
pub use fixture::{FIXTURE_START, RepoFixture};
