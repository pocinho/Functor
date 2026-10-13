//! Functor core API: service traits, errors, clock and id generation.
//!
//! See `docs/architecture/traits-and-interfaces.md` and `docs/architecture/error-handling.md`.
//!
//! This crate defines **contracts only**: service traits (dyn-compatible through `async-trait`,
//! ADR-0010), the error taxonomy, and the injected [`Clock`] and [`IdGenerator`]. It names no
//! runtime and does no I/O.

mod clock;
mod error;
mod repository;
mod services;

pub use clock::{Clock, IdGenerator, SystemClock, UlidGenerator};
pub use error::{ErrorKind, FunctorError, Result};
pub use repository::{
    InitOptions, LoadIssue, LoadIssueKind, RepositoryHandle, RepositoryManager, RepositoryReader,
    SnapshotLoad,
};
pub use services::{
    ArtifactRepository, EventPosition, EventPublisher, EventSource, EventSubscription, QueryEngine,
    RelationshipRepository, ValidationEngine, ValidationTarget,
};
