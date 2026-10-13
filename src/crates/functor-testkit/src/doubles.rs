//! Deterministic test doubles for the injected clock and id generator.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use functor_core::{Clock, IdGenerator};
use functor_model::{ArtifactId, EventId, RelationshipId, RepositoryId, Timestamp};

/// A clock that only moves when told to.
#[derive(Debug)]
pub struct FixedClock {
    seconds: AtomicI64,
}

impl FixedClock {
    /// Creates a clock stopped at `start`.
    #[must_use]
    pub fn new(start: Timestamp) -> Self {
        Self {
            seconds: AtomicI64::new(start.unix_seconds()),
        }
    }

    /// Moves the clock to `instant`.
    pub fn set(&self, instant: Timestamp) {
        self.seconds.store(instant.unix_seconds(), Ordering::SeqCst);
    }

    /// Moves the clock forward by `seconds` (saturating at the representable range).
    pub fn advance(&self, seconds: i64) {
        self.seconds.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        let seconds = self.seconds.load(Ordering::SeqCst);
        Timestamp::from_unix_seconds(seconds).unwrap_or(if seconds < 0 {
            Timestamp::MIN
        } else {
            Timestamp::MAX
        })
    }
}

/// An id generator that hands out 1, 2, 3, ... across all id kinds, so ids are unique, sort in
/// creation order and are identical from run to run.
#[derive(Debug)]
pub struct SequentialIds {
    next: AtomicU64,
}

impl SequentialIds {
    /// Starts counting at 1.
    #[must_use]
    pub fn new() -> Self {
        Self::starting_at(1)
    }

    /// Starts counting at `first`.
    #[must_use]
    pub fn starting_at(first: u64) -> Self {
        Self {
            next: AtomicU64::new(first),
        }
    }

    fn bits(&self) -> u128 {
        u128::from(self.next.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for SequentialIds {
    fn default() -> Self {
        Self::new()
    }
}

impl IdGenerator for SequentialIds {
    fn artifact_id(&self) -> ArtifactId {
        ArtifactId::from_bits(self.bits())
    }

    fn relationship_id(&self) -> RelationshipId {
        RelationshipId::from_bits(self.bits())
    }

    fn repository_id(&self) -> RepositoryId {
        RepositoryId::from_bits(self.bits())
    }

    fn event_id(&self) -> EventId {
        EventId::from_bits(self.bits())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(text: &str) -> Timestamp {
        Timestamp::parse(text).unwrap()
    }

    #[test]
    fn fixed_clock_only_moves_when_told() {
        let clock = FixedClock::new(t("2026-10-12T18:00:00Z"));
        assert_eq!(clock.now(), t("2026-10-12T18:00:00Z"));
        assert_eq!(clock.now(), clock.now());
        clock.advance(60);
        assert_eq!(clock.now(), t("2026-10-12T18:01:00Z"));
        clock.set(t("2027-01-01T00:00:00Z"));
        assert_eq!(clock.now(), t("2027-01-01T00:00:00Z"));
    }

    #[test]
    fn fixed_clock_saturates_instead_of_panicking() {
        let clock = FixedClock::new(Timestamp::MAX);
        clock.advance(10);
        assert_eq!(clock.now(), Timestamp::MAX);
        clock.set(Timestamp::MIN);
        clock.advance(-10);
        assert_eq!(clock.now(), Timestamp::MIN);
    }

    #[test]
    fn sequential_ids_are_unique_ordered_and_repeatable() {
        let ids = SequentialIds::new();
        let a = ids.artifact_id();
        let b = ids.artifact_id();
        let rel = ids.relationship_id();
        assert!(a < b);
        assert_eq!(a, ArtifactId::from_bits(1));
        assert_eq!(rel, RelationshipId::from_bits(3));

        let again = SequentialIds::new();
        assert_eq!(again.artifact_id(), a);
        assert_eq!(
            SequentialIds::starting_at(10).artifact_id(),
            ArtifactId::from_bits(10)
        );
    }
}
