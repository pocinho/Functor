//! Event emission seam.
//!
//! Events arrive in M8 (`functor-events`, ADR-0003). Until then mutating commands already build
//! the event they will publish and hand it to [`Recorder`], which discards it. Replacing the
//! recorder is the only change M8 needs in the command handlers.

use functor_model::{
    Artifact, ArtifactField, EventData, EventKind, EventSubject, NewEvent, Relationship,
};

/// Receives the events produced by commands.
#[derive(Debug, Default)]
pub struct Recorder;

impl Recorder {
    /// Records an event. Currently a no-op.
    #[allow(clippy::unused_self)]
    pub fn record(&self, _event: NewEvent) {}
}

/// The `artifact.created` event for a new artifact.
#[must_use]
pub fn artifact_created(artifact: &Artifact) -> NewEvent {
    NewEvent::new(
        EventKind::ArtifactCreated,
        EventSubject::Artifact(*artifact.id()),
        EventData::new()
            .with("slug", artifact.slug().as_str())
            .with("type", artifact.artifact_type().as_str())
            .with(
                "revision",
                i64::try_from(artifact.revision().get()).unwrap_or(i64::MAX),
            ),
    )
}

/// The `artifact.updated` event for a changed artifact.
#[must_use]
pub fn artifact_updated(artifact: &Artifact, changed: &[ArtifactField]) -> NewEvent {
    NewEvent::new(
        EventKind::ArtifactUpdated,
        EventSubject::Artifact(*artifact.id()),
        EventData::new()
            .with("slug", artifact.slug().as_str())
            .with(
                "revision",
                i64::try_from(artifact.revision().get()).unwrap_or(i64::MAX),
            )
            .with(
                "changed",
                changed.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ),
    )
}

/// The `relationship.created` event for a new relationship.
#[must_use]
pub fn relationship_created(relationship: &Relationship) -> NewEvent {
    NewEvent::new(
        EventKind::RelationshipCreated,
        EventSubject::Relationship(*relationship.id()),
        EventData::new()
            .with(
                "relationship_type",
                relationship.relationship_type().as_str(),
            )
            .with("source", relationship.source().to_string().as_str())
            .with("target", relationship.target().to_string().as_str()),
    )
}

#[cfg(test)]
mod tests {
    use functor_model::{
        ArtifactId, ArtifactPatch, ArtifactType, MetadataValue, NewArtifact, NewRelationship,
        RelationshipId, RelationshipType, Slug, Timestamp, Title,
    };

    use super::*;

    #[test]
    fn relationship_created_events_carry_type_and_endpoints() {
        let relationship = Relationship::create(
            RelationshipId::from_bits(7),
            NewRelationship::new(
                RelationshipType::DependsOn,
                ArtifactId::from_bits(1),
                ArtifactId::from_bits(2),
            ),
            Timestamp::parse("2026-10-12T18:00:00Z").unwrap(),
        )
        .unwrap();
        let event = relationship_created(&relationship);
        assert_eq!(event.kind, EventKind::RelationshipCreated);
        assert_eq!(
            event.subject,
            EventSubject::Relationship(RelationshipId::from_bits(7))
        );
        assert_eq!(
            event.data.get("relationship_type"),
            Some(&MetadataValue::String("depends_on".into()))
        );
        assert_eq!(
            event.data.get("source"),
            Some(&MetadataValue::String(ArtifactId::from_bits(1).to_string()))
        );
        assert_eq!(
            event.data.get("target"),
            Some(&MetadataValue::String(ArtifactId::from_bits(2).to_string()))
        );
    }

    fn artifact() -> Artifact {
        Artifact::create(
            ArtifactId::from_bits(1),
            NewArtifact::new(
                Slug::parse("rfc-0000").unwrap(),
                ArtifactType::new("rfc").unwrap(),
                Title::new("T").unwrap(),
            ),
            Timestamp::parse("2026-10-12T18:00:00Z").unwrap(),
        )
    }

    #[test]
    fn created_events_carry_slug_type_and_revision() {
        let event = artifact_created(&artifact());
        assert_eq!(event.kind, EventKind::ArtifactCreated);
        assert_eq!(
            event.subject,
            EventSubject::Artifact(ArtifactId::from_bits(1))
        );
        assert_eq!(
            event.data.get("slug"),
            Some(&MetadataValue::String("rfc-0000".into()))
        );
        assert_eq!(
            event.data.get("type"),
            Some(&MetadataValue::String("rfc".into()))
        );
        assert_eq!(event.data.get("revision"), Some(&MetadataValue::Integer(1)));
    }

    #[test]
    fn updated_events_list_the_changed_fields() {
        let original = artifact();
        let patched = original
            .apply_patch(
                ArtifactPatch::new().with_title(Title::new("New").unwrap()),
                Timestamp::parse("2026-10-13T00:00:00Z").unwrap(),
            )
            .unwrap();
        let event = artifact_updated(&patched.artifact, &patched.changed);
        assert_eq!(event.kind, EventKind::ArtifactUpdated);
        assert_eq!(event.data.get("revision"), Some(&MetadataValue::Integer(2)));
        assert_eq!(
            event.data.get("changed"),
            Some(&MetadataValue::StringList(vec!["title".into()]))
        );
    }
}
