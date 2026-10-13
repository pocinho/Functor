//! Deterministic time and ids for golden tests. Compiled only with the `test-support` feature,
//! so release builds do not recognize these environment variables.
//!
//! - `FUNCTOR_TEST_FIXED_TIME=2026-10-12T18:00:00Z` stops the clock at that instant.
//! - `FUNCTOR_TEST_ID_SEED=100` makes ids count 100, 101, 102, ... within one process.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use functor_core::{Clock, FunctorError, IdGenerator, Result, SystemClock, UlidGenerator};
use functor_model::{ArtifactId, EventId, RelationshipId, RepositoryId, Timestamp};

use crate::context::Services;

struct FixedTime(Timestamp);

impl Clock for FixedTime {
    fn now(&self) -> Timestamp {
        self.0
    }
}

struct CountingIds(AtomicU64);

impl CountingIds {
    fn bits(&self) -> u128 {
        u128::from(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

impl IdGenerator for CountingIds {
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

/// The deterministic services, if either test variable is set.
pub fn services() -> Result<Option<Services>> {
    let time = std::env::var("FUNCTOR_TEST_FIXED_TIME").ok();
    let seed = std::env::var("FUNCTOR_TEST_ID_SEED").ok();
    if time.is_none() && seed.is_none() {
        return Ok(None);
    }
    let clock: Arc<dyn Clock> = match time {
        Some(text) => Arc::new(FixedTime(Timestamp::parse(&text)?)),
        None => Arc::new(SystemClock),
    };
    let ids: Arc<dyn IdGenerator> = match seed {
        Some(text) => {
            let first = text.parse::<u64>().map_err(|_| {
                FunctorError::invalid_input("FUNCTOR_TEST_ID_SEED", "must be a whole number")
            })?;
            Arc::new(CountingIds(AtomicU64::new(first)))
        }
        None => Arc::new(UlidGenerator::new()),
    };
    Ok(Some(Services { clock, ids }))
}
