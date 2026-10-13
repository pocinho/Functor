//! Functor events: event log, rotation, publishing and watching.
//!
//! See `docs/design/event-model-mvp.md`. [`FileEventLog`] appends events under the repository write lock
//! (gap-free sequence numbers, size-based rotation) and implements the core `EventPublisher` and
//! `EventSource` traits; [`Follower`] is the live view `functor watch` uses, which also reports
//! warnings the plain `EventSubscription` trait cannot carry. [`encode_event`] and
//! [`decode_event`] are the wire format.

mod follow;
mod log;
mod wire;

pub use follow::{FollowOptions, Follower, Update};
pub use log::{
    EventLogOptions, FileEventLog, Limits, MAX_ARCHIVES, MAX_LOG_BYTES, Published, archive_name,
};
pub use wire::{ENVELOPE_VERSION, LineProblem, decode_event, encode_event, peek_seq};
