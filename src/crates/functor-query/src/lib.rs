//! Functor query engine: lookup, search and graph traversal over repository snapshots.
//!
//! See `docs/query-model-mvp.md`. The semantics live in pure, synchronous functions over a
//! [`RepositorySnapshot`](functor_model::RepositorySnapshot) ([`search`], [`traverse`],
//! [`relationships_of`]); [`SnapshotQueryEngine`] adapts them to the asynchronous
//! [`QueryEngine`](functor_core::QueryEngine) trait by loading one snapshot per call.

mod engine;
mod search;
mod traverse;

pub use engine::SnapshotQueryEngine;
pub use search::search;
pub use traverse::{relationships_of, traverse};
