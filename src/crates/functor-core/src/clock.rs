//! Injected time and identifier sources.

use std::time::{SystemTime, UNIX_EPOCH};

use functor_model::{ArtifactId, EventId, MonotonicUlids, RelationshipId, RepositoryId, Timestamp};

/// A source of the current time. Injected so tests are deterministic.
pub trait Clock: Send + Sync {
    /// The current instant, at second precision.
    fn now(&self) -> Timestamp;
}

/// A source of new identifiers. Injected so tests are deterministic.
pub trait IdGenerator: Send + Sync {
    /// A new artifact id.
    fn artifact_id(&self) -> ArtifactId;
    /// A new relationship id.
    fn relationship_id(&self) -> RelationshipId;
    /// A new repository id.
    fn repository_id(&self) -> RepositoryId;
    /// A new event id.
    fn event_id(&self) -> EventId;
}

/// The production clock, backed by the system time.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
        Timestamp::from_unix_seconds(seconds).unwrap_or(Timestamp::MAX)
    }
}

/// The production id generator: monotonic ULIDs, so ids sort in creation order.
#[derive(Debug, Default)]
pub struct UlidGenerator {
    source: MonotonicUlids,
}

impl UlidGenerator {
    /// Creates a generator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl IdGenerator for UlidGenerator {
    fn artifact_id(&self) -> ArtifactId {
        ArtifactId::from_bits(self.source.next_bits())
    }

    fn relationship_id(&self) -> RelationshipId {
        RelationshipId::from_bits(self.source.next_bits())
    }

    fn repository_id(&self) -> RepositoryId {
        RepositoryId::from_bits(self.source.next_bits())
    }

    fn event_id(&self) -> EventId {
        EventId::from_bits(self.source.next_bits())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_is_after_2025_and_does_not_go_backwards() {
        let clock = SystemClock;
        let first = clock.now();
        let second = clock.now();
        assert!(first.unix_seconds() > 1_735_689_600);
        assert!(second >= first);
    }

    #[test]
    fn ulid_generator_ids_increase_across_kinds() {
        let ids = UlidGenerator::new();
        let a = ids.artifact_id();
        let b = ids.artifact_id();
        assert!(a < b);
        assert!(ids.relationship_id().to_string().starts_with("rel_"));
        assert!(ids.repository_id().to_string().starts_with("repo_"));
        assert!(ids.event_id().to_string().starts_with("evt_"));
    }

    #[test]
    fn generators_are_usable_as_trait_objects() {
        let clock: Box<dyn Clock> = Box::new(SystemClock);
        let ids: Box<dyn IdGenerator> = Box::new(UlidGenerator::new());
        let _ = (clock.now(), ids.artifact_id());
    }
}
