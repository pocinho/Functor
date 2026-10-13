//! Event emission.
//!
//! Mutating commands build the event after their state change has committed and hand it to
//! [`Recorder`], which appends it to the event log (ADR-0003, `docs/design/event-model-mvp.md`). A
//! failed append never fails the command: it prints `warning: event not recorded: <reason>` to
//! stderr and the command carries on with exit code 0.

use functor_events::FileEventLog;
use functor_model::{
    Artifact, ArtifactField, EventData, EventKind, EventSubject, NewEvent, Relationship,
    RepositoryId, ValidationReport,
};

/// Appends the events produced by commands.
#[derive(Debug, Clone)]
pub struct Recorder {
    log: FileEventLog,
}

impl Recorder {
    /// A recorder that appends to `log`.
    #[must_use]
    pub fn new(log: FileEventLog) -> Self {
        Self { log }
    }

    /// Appends an event. A failure, or a rotation or pruning problem, is reported as a warning
    /// on stderr and does not affect the command.
    pub async fn record(&self, event: NewEvent) {
        match self.log.publish_detailed(event).await {
            Ok(published) => {
                for warning in published.warnings {
                    eprintln!("warning: {warning}");
                }
            }
            Err(error) => eprintln!("warning: event not recorded: {error}"),
        }
    }
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

/// The `validation.completed` event for a finished validation run.
#[must_use]
pub fn validation_completed(
    repository: RepositoryId,
    report: &ValidationReport,
    strict: bool,
) -> NewEvent {
    let count = |n: usize| i64::try_from(n).unwrap_or(i64::MAX);
    let counts = report.counts();
    NewEvent::new(
        EventKind::ValidationCompleted,
        EventSubject::Repository(repository),
        EventData::new()
            .with("outcome", report.outcome().as_str())
            .with("errors", count(counts.errors))
            .with("warnings", count(counts.warnings))
            .with("infos", count(counts.infos))
            .with("strict", strict),
    )
}

#[cfg(test)]
mod tests {
    use functor_model::{
        ArtifactId, ArtifactPatch, ArtifactType, Finding, MetadataValue, NewArtifact,
        NewRelationship, RelationshipId, RelationshipType, RuleCode, Severity, Slug, Subject,
        Timestamp, Title,
    };

    use super::*;

    #[test]
    fn validation_completed_events_carry_the_outcome_and_counts() {
        let report = ValidationReport::new(
            vec![
                Finding::new(
                    RuleCode::new("V0007"),
                    Severity::Error,
                    Subject::Repository,
                    "x",
                ),
                Finding::new(
                    RuleCode::new("V0012"),
                    Severity::Warning,
                    Subject::Repository,
                    "y",
                ),
                Finding::new(
                    RuleCode::new("V0018"),
                    Severity::Info,
                    Subject::Repository,
                    "z",
                ),
                Finding::new(
                    RuleCode::new("V0018"),
                    Severity::Info,
                    Subject::Repository,
                    "w",
                ),
            ],
            3,
            2,
            false,
            Timestamp::parse("2026-10-12T18:00:00Z").unwrap(),
        );
        let event = validation_completed(RepositoryId::from_bits(5), &report, true);
        assert_eq!(event.kind, EventKind::ValidationCompleted);
        assert_eq!(
            event.subject,
            EventSubject::Repository(RepositoryId::from_bits(5))
        );
        assert_eq!(
            event.data.get("outcome"),
            Some(&MetadataValue::String("failed".into()))
        );
        assert_eq!(event.data.get("errors"), Some(&MetadataValue::Integer(1)));
        assert_eq!(event.data.get("warnings"), Some(&MetadataValue::Integer(1)));
        assert_eq!(event.data.get("infos"), Some(&MetadataValue::Integer(2)));
        assert_eq!(event.data.get("strict"), Some(&MetadataValue::Bool(true)));
    }

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
