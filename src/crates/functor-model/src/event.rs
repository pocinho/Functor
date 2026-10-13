//! Event value types (RFC-0017 subset). Persistence lives in `functor-events`.
//!
//! See `docs/design/event-model-mvp.md`.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use crate::artifact::MetadataValue;
use crate::error::{ModelError, Result};
use crate::id::{ArtifactId, EventId, RelationshipId, RepositoryId};
use crate::timestamp::Timestamp;

/// The event types of the MVP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum EventKind {
    /// `artifact.created`
    ArtifactCreated,
    /// `artifact.updated`
    ArtifactUpdated,
    /// `relationship.created`
    RelationshipCreated,
    /// `validation.completed`
    ValidationCompleted,
}

impl EventKind {
    /// The dotted, hierarchical name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArtifactCreated => "artifact.created",
            Self::ArtifactUpdated => "artifact.updated",
            Self::RelationshipCreated => "relationship.created",
            Self::ValidationCompleted => "validation.completed",
        }
    }

    /// Parses a dotted name.
    ///
    /// # Errors
    /// Fails for names that are not MVP event types.
    pub fn parse(input: &str) -> Result<Self> {
        [
            Self::ArtifactCreated,
            Self::ArtifactUpdated,
            Self::RelationshipCreated,
            Self::ValidationCompleted,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == input)
        .ok_or_else(|| ModelError::new("event type", format!("unknown event type `{input}`")))
    }

    /// Whether this type's name equals `prefix` or starts with it (for example `artifact.`).
    #[must_use]
    pub fn matches_prefix(self, prefix: &str) -> bool {
        self.as_str().starts_with(prefix)
    }
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EventKind {
    type Err = ModelError;

    fn from_str(input: &str) -> Result<Self> {
        Self::parse(input)
    }
}

/// The entity an event is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum EventSubject {
    /// An artifact.
    Artifact(ArtifactId),
    /// A relationship.
    Relationship(RelationshipId),
    /// A repository.
    Repository(RepositoryId),
}

impl EventSubject {
    /// The canonical reference, for example `functor:artifact:art_...`.
    #[must_use]
    pub fn canonical(&self) -> String {
        match self {
            Self::Artifact(id) => id.canonical(),
            Self::Relationship(id) => id.canonical(),
            Self::Repository(id) => id.canonical(),
        }
    }

    /// Parses a canonical reference of any supported entity kind.
    ///
    /// # Errors
    /// Fails when the text is not a canonical reference to an artifact, relationship or repository.
    pub fn from_canonical(input: &str) -> Result<Self> {
        let invalid = || ModelError::new("event subject", "must be a canonical reference");
        let rest = input.strip_prefix("functor:").ok_or_else(invalid)?;
        let (kind, _) = rest.split_once(':').ok_or_else(invalid)?;
        match kind {
            "artifact" => ArtifactId::from_canonical(input).map(Self::Artifact),
            "relationship" => RelationshipId::from_canonical(input).map(Self::Relationship),
            "repository" => RepositoryId::from_canonical(input).map(Self::Repository),
            _ => Err(invalid()),
        }
    }
}

/// Per-type event details, with keys in ascending order so output is deterministic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventData(BTreeMap<String, MetadataValue>);

impl EventData {
    /// Creates empty data.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a field.
    #[must_use]
    pub fn with(mut self, key: &str, value: impl Into<MetadataValue>) -> Self {
        self.0.insert(key.to_owned(), value.into());
        self
    }

    /// Looks up a field.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&MetadataValue> {
        self.0.get(key)
    }

    /// Iterates fields in ascending key order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &MetadataValue)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Whether there are no fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// An event before the publisher assigns its id, sequence number and timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NewEvent {
    /// The event type.
    pub kind: EventKind,
    /// What the event is about.
    pub subject: EventSubject,
    /// Per-type details.
    pub data: EventData,
}

impl NewEvent {
    /// Creates an event description.
    #[must_use]
    pub fn new(kind: EventKind, subject: EventSubject, data: EventData) -> Self {
        Self {
            kind,
            subject,
            data,
        }
    }
}

/// An immutable, informational record of a state change (RFC-0017). Never authoritative.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Event {
    /// The event identity.
    pub id: EventId,
    /// The sequence number within the repository, starting at 1.
    pub seq: u64,
    /// When the event happened.
    pub at: Timestamp,
    /// The event type.
    pub kind: EventKind,
    /// The repository that produced the event.
    pub origin: RepositoryId,
    /// What the event is about.
    pub subject: EventSubject,
    /// Per-type details.
    pub data: EventData,
}

impl Event {
    /// Completes a [`NewEvent`] with its assigned identity, sequence number, time and origin.
    ///
    /// # Errors
    /// Fails when `seq` is zero.
    pub fn new(
        id: EventId,
        seq: u64,
        at: Timestamp,
        origin: RepositoryId,
        new: NewEvent,
    ) -> Result<Self> {
        if seq == 0 {
            return Err(ModelError::new("event sequence", "must be at least 1"));
        }
        Ok(Self {
            id,
            seq,
            at,
            kind: new.kind,
            origin,
            subject: new.subject,
            data: new.data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_names_round_trip() {
        for kind in [
            EventKind::ArtifactCreated,
            EventKind::ArtifactUpdated,
            EventKind::RelationshipCreated,
            EventKind::ValidationCompleted,
        ] {
            assert_eq!(EventKind::parse(kind.as_str()).unwrap(), kind);
        }
        assert!(EventKind::parse("artifact.deleted").is_err());
        assert!(EventKind::parse("").is_err());
    }

    #[test]
    fn prefix_matching() {
        assert!(EventKind::ArtifactCreated.matches_prefix("artifact."));
        assert!(EventKind::ArtifactCreated.matches_prefix("artifact.created"));
        assert!(!EventKind::ArtifactCreated.matches_prefix("relationship."));
    }

    #[test]
    fn subject_canonical_round_trips() {
        for subject in [
            EventSubject::Artifact(ArtifactId::from_bits(1)),
            EventSubject::Relationship(RelationshipId::from_bits(2)),
            EventSubject::Repository(RepositoryId::from_bits(3)),
        ] {
            assert_eq!(
                EventSubject::from_canonical(&subject.canonical()).unwrap(),
                subject
            );
        }
        assert!(EventSubject::from_canonical("functor:event:evt_x").is_err());
        assert!(EventSubject::from_canonical("artifact:x").is_err());
    }

    #[test]
    fn data_is_ordered_by_key() {
        let data = EventData::new()
            .with("source", "s")
            .with("revision", 1_i64)
            .with("slug", "x");
        let keys: Vec<&str> = data.iter().map(|(k, _)| k).collect();
        assert_eq!(keys, ["revision", "slug", "source"]);
        assert_eq!(data.get("revision"), Some(&MetadataValue::Integer(1)));
        assert!(!data.is_empty());
    }

    #[test]
    fn sequence_starts_at_one() {
        let new = || {
            NewEvent::new(
                EventKind::ValidationCompleted,
                EventSubject::Repository(RepositoryId::from_bits(1)),
                EventData::new(),
            )
        };
        let at = Timestamp::parse("2026-10-12T18:00:00Z").unwrap();
        let make = |seq| {
            Event::new(
                EventId::from_bits(1),
                seq,
                at,
                RepositoryId::from_bits(1),
                new(),
            )
        };
        assert!(make(0).is_err());
        assert_eq!(make(1).unwrap().seq, 1);
    }
}
