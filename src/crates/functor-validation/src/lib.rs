//! Functor validation: rule catalog and validation engine.
//!
//! See `docs/design/validation-rules.md`. The rules are pure functions over a lenient
//! [`SnapshotLoad`](functor_core::SnapshotLoad); [`evaluate`] runs them and assembles the report,
//! and [`RuleValidationEngine`] adapts that to the asynchronous
//! [`ValidationEngine`](functor_core::ValidationEngine) trait.

mod catalog;
mod engine;
mod evaluate;
mod graph;
mod rules;

pub use catalog::{CATALOG, RuleInfo, rule_info};
pub use engine::RuleValidationEngine;
pub use evaluate::evaluate;
