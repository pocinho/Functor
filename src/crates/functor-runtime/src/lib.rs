//! Functor execution runtime: the bounded local-process provider.
//!
//! [`LocalProcessProvider`] implements `functor_core::ExecutionProvider` for workflows whose
//! provider is `local-process` (ADR-0012): one child process per run from a structured program
//! and arguments (no shell), a cleared environment with a closed allowlist, bounded stdout,
//! stderr and outputs, a wall-clock timeout, cooperative cancellation and termination of the
//! whole process tree.
//!
//! It is **not a security sandbox**: the child runs with the caller's privileges and may touch
//! anything the caller can, including the network. The provider takes injected collaborators only
//! (a repository root); it never opens a store, validates or publishes. Durable publication is the
//! orchestrator's job (P1.4).

mod orchestrator;
mod program;
mod provider;
mod supervise;
mod workflow;

pub use orchestrator::{
    OutputChecks, OutputValidation, RecoveryReport, RunFailure, RunOrchestrator, RunReport,
    SavedReport,
};
pub use provider::LocalProcessProvider;
pub use workflow::parse_workflow;
