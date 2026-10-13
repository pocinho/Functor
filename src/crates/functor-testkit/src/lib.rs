//! Functor test support: test doubles, fixtures and the store conformance suite.
//!
//! See `docs/testing-strategy.md`. Used only as a dev-dependency of other crates.
//!
//! - [`FixedClock`] and [`SequentialIds`] make time and identifiers deterministic.
//! - [`RepoFixture`] builds small in-memory repositories.
//! - [`run_store_conformance`] is the behavioral contract every store implementation must pass.

mod conformance;
mod doubles;
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
pub use fixture::{FIXTURE_START, RepoFixture};
