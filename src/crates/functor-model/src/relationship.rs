//! Relationships between artifacts (RFC-0004).
//!
//! See `docs/design/domain-model.md` and ADR-0007.

use std::fmt;
use std::str::FromStr;

use crate::artifact::Metadata;
use crate::error::{ModelError, Result};
use crate::grammar::{check_name, is_lower, is_lower_digit_underscore};
use crate::id::{ArtifactId, RelationshipId};
use crate::timestamp::Timestamp;

const BUILT_INS: [&str; 7] = [
    "depends_on",
    "references",
    "derived_from",
    "extends",
    "supports",
    "contradicts",
    "validates",
];

/// The name of a custom relationship type: `[a-z][a-z0-9_]{0,63}`, never a built-in name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelationshipTypeName(String);

impl RelationshipTypeName {
    /// Validates a custom type name.
    ///
    /// # Errors
    /// Fails when the grammar is violated or the name is a built-in type.
    pub fn new(input: &str) -> Result<Self> {
        check_name(
            "relationship type",
            input,
            64,
            is_lower,
            is_lower_digit_underscore,
            "[a-z][a-z0-9_]{0,63}",
        )?;
        if BUILT_INS.contains(&input) {
            return Err(ModelError::new(
                "relationship type",
                format!("`{input}` is a built-in type and cannot be custom"),
            ));
        }
        Ok(Self(input.to_owned()))
    }

    /// The name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The semantics of a relationship edge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum RelationshipType {
    /// The source requires the target.
    DependsOn,
    /// The source references the target.
    References,
    /// The source originates from the target.
    DerivedFrom,
    /// The source extends or specializes the target.
    Extends,
    /// The source is evidence supporting the target claim (ADR-0014).
    Supports,
    /// The source is evidence contradicting the target claim (ADR-0014).
    Contradicts,
    /// The source, a validation report, evaluated the target (ADR-0014).
    Validates,
    /// Any other well-formed type; flagged by validation (`V0012`).
    Custom(RelationshipTypeName),
}

impl RelationshipType {
    /// Parses a type name in `snake_case` or `kebab-case`; both spellings normalize to `snake_case`.
    ///
    /// Built-in names always yield the built-in variant, never `Custom`. Names that were custom
    /// before ADR-0014 (`supports`, `contradicts`, `validates`) now parse as built-ins, so stored
    /// files need no change.
    ///
    /// # Errors
    /// Fails when the normalized name violates the custom-type grammar.
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = input.replace('-', "_");
        Ok(match normalized.as_str() {
            "depends_on" => Self::DependsOn,
            "references" => Self::References,
            "derived_from" => Self::DerivedFrom,
            "extends" => Self::Extends,
            "supports" => Self::Supports,
            "contradicts" => Self::Contradicts,
            "validates" => Self::Validates,
            other => Self::Custom(RelationshipTypeName::new(other)?),
        })
    }

    /// The `snake_case` name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::DependsOn => "depends_on",
            Self::References => "references",
            Self::DerivedFrom => "derived_from",
            Self::Extends => "extends",
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
            Self::Validates => "validates",
            Self::Custom(name) => name.as_str(),
        }
    }

    /// Whether this is one of the built-in types.
    #[must_use]
    pub fn is_built_in(&self) -> bool {
        !matches!(self, Self::Custom(_))
    }

    /// Whether the graph of this type must be acyclic (`depends_on`, `derived_from`, `extends`).
    #[must_use]
    pub fn is_acyclic(&self) -> bool {
        matches!(self, Self::DependsOn | Self::DerivedFrom | Self::Extends)
    }

    /// Whether an edge from an artifact to itself is invalid: the acyclic types and the evidence
    /// assertions `supports`, `contradicts` and `validates` (ADR-0014).
    #[must_use]
    pub fn rejects_self_reference(&self) -> bool {
        self.is_acyclic() || self.is_evidence()
    }

    /// Whether this is an evidence assertion type: `supports`, `contradicts` or `validates`.
    #[must_use]
    pub fn is_evidence(&self) -> bool {
        matches!(self, Self::Supports | Self::Contradicts | Self::Validates)
    }
}

impl fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RelationshipType {
    type Err = ModelError;

    fn from_str(input: &str) -> Result<Self> {
        Self::parse(input)
    }
}

/// A first-class, directed, identified edge `source --type--> target`.
// `relationship_type` mirrors the RFC-0004 term and the documented field name.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    id: RelationshipId,
    relationship_type: RelationshipType,
    source: ArtifactId,
    target: ArtifactId,
    created: Timestamp,
    metadata: Metadata,
}

/// The data needed to create a relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NewRelationship {
    /// The type of the edge.
    pub relationship_type: RelationshipType,
    /// The artifact holding the dependency.
    pub source: ArtifactId,
    /// The artifact pointed to.
    pub target: ArtifactId,
    /// Metadata.
    pub metadata: Metadata,
}

impl NewRelationship {
    /// Starts a new relationship.
    #[must_use]
    pub fn new(
        relationship_type: RelationshipType,
        source: ArtifactId,
        target: ArtifactId,
    ) -> Self {
        Self {
            relationship_type,
            source,
            target,
            metadata: Metadata::default(),
        }
    }

    /// Sets the metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }
}

impl Relationship {
    /// Creates a relationship, rejecting self-reference for acyclic and evidence types.
    ///
    /// Existence of the endpoints is not checked here; repositories check it on creation and
    /// validation checks it for existing data.
    ///
    /// # Errors
    /// Fails when `source == target` and the type rejects self-reference.
    pub fn create(id: RelationshipId, new: NewRelationship, now: Timestamp) -> Result<Self> {
        Self::from_parts(
            id,
            new.relationship_type,
            new.source,
            new.target,
            now,
            new.metadata,
        )
    }

    /// Reconstitutes a relationship from stored parts, applying the same self-reference rule.
    ///
    /// # Errors
    /// Fails when `source == target` and the type rejects self-reference.
    pub fn from_parts(
        id: RelationshipId,
        relationship_type: RelationshipType,
        source: ArtifactId,
        target: ArtifactId,
        created: Timestamp,
        metadata: Metadata,
    ) -> Result<Self> {
        if source == target && relationship_type.rejects_self_reference() {
            return Err(ModelError::new(
                "target",
                format!("a `{relationship_type}` relationship cannot point to its own source"),
            ));
        }
        Ok(Self {
            id,
            relationship_type,
            source,
            target,
            created,
            metadata,
        })
    }

    /// The identity.
    #[must_use]
    pub fn id(&self) -> &RelationshipId {
        &self.id
    }

    /// The edge type.
    #[must_use]
    pub fn relationship_type(&self) -> &RelationshipType {
        &self.relationship_type
    }

    /// The source artifact.
    #[must_use]
    pub fn source(&self) -> &ArtifactId {
        &self.source
    }

    /// The target artifact.
    #[must_use]
    pub fn target(&self) -> &ArtifactId {
        &self.target
    }

    /// When the relationship was created.
    #[must_use]
    pub fn created(&self) -> Timestamp {
        self.created
    }

    /// The metadata.
    #[must_use]
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_names_parse_to_built_ins() {
        for (text, expected) in [
            ("depends_on", RelationshipType::DependsOn),
            ("depends-on", RelationshipType::DependsOn),
            ("references", RelationshipType::References),
            ("derived_from", RelationshipType::DerivedFrom),
            ("derived-from", RelationshipType::DerivedFrom),
            ("extends", RelationshipType::Extends),
            ("supports", RelationshipType::Supports),
            ("contradicts", RelationshipType::Contradicts),
            ("validates", RelationshipType::Validates),
        ] {
            assert_eq!(RelationshipType::parse(text).unwrap(), expected, "{text}");
        }
    }

    #[test]
    fn display_is_snake_case_and_round_trips() {
        for t in [
            "depends_on",
            "references",
            "derived_from",
            "extends",
            "supports",
            "contradicts",
            "validates",
            "related_to",
            "x1_y",
        ] {
            let parsed = RelationshipType::parse(t).unwrap();
            assert_eq!(parsed.to_string(), t);
            assert_eq!(
                RelationshipType::parse(&parsed.to_string()).unwrap(),
                parsed
            );
        }
        assert_eq!(
            RelationshipType::parse("depends-on").unwrap().to_string(),
            "depends_on"
        );
    }

    #[test]
    fn custom_types_are_flagged_and_validated() {
        let related = RelationshipType::parse("related_to").unwrap();
        assert!(!related.is_built_in());
        assert!(!related.is_acyclic());
        assert!(!related.rejects_self_reference());
        assert!(RelationshipType::parse("a-b").is_ok());
        for bad in ["", "Depends", "1x", "a b", "a.b", &"a".repeat(65)] {
            assert!(RelationshipType::parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn built_in_names_cannot_be_constructed_as_custom() {
        for name in BUILT_INS {
            assert!(RelationshipTypeName::new(name).is_err(), "{name}");
        }
    }

    #[test]
    fn acyclic_types() {
        assert!(RelationshipType::DependsOn.is_acyclic());
        assert!(RelationshipType::DerivedFrom.is_acyclic());
        assert!(RelationshipType::Extends.is_acyclic());
        assert!(!RelationshipType::References.is_acyclic());
    }

    #[test]
    fn evidence_types_are_built_in_but_may_form_cycles() {
        for t in [
            RelationshipType::Supports,
            RelationshipType::Contradicts,
            RelationshipType::Validates,
        ] {
            assert!(t.is_built_in(), "{t}");
            assert!(t.is_evidence(), "{t}");
            assert!(!t.is_acyclic(), "{t}");
            assert!(t.rejects_self_reference(), "{t}");
        }
        assert!(!RelationshipType::References.is_evidence());
        assert!(!RelationshipType::References.rejects_self_reference());
    }

    #[test]
    fn evidence_direction_and_self_reference() {
        let report = ArtifactId::from_bits(1);
        let claim = ArtifactId::from_bits(2);
        let now = Timestamp::parse("2026-10-12T18:05:00Z").unwrap();
        for t in [
            RelationshipType::Supports,
            RelationshipType::Contradicts,
            RelationshipType::Validates,
        ] {
            let edge = Relationship::create(
                RelationshipId::from_bits(9),
                NewRelationship::new(t.clone(), report, claim),
                now,
            )
            .unwrap();
            assert_eq!((edge.source(), edge.target()), (&report, &claim), "{t}");
            let own = Relationship::create(
                RelationshipId::from_bits(10),
                NewRelationship::new(t.clone(), report, report),
                now,
            );
            assert!(own.is_err(), "{t}");
        }
    }

    #[test]
    fn relationship_direction_and_self_reference() {
        let a = ArtifactId::from_bits(1);
        let b = ArtifactId::from_bits(2);
        let now = Timestamp::parse("2026-10-12T18:05:00Z").unwrap();
        let rel = Relationship::create(
            RelationshipId::from_bits(9),
            NewRelationship::new(RelationshipType::DependsOn, a, b),
            now,
        )
        .unwrap();
        assert_eq!(rel.source(), &a);
        assert_eq!(rel.target(), &b);
        assert_eq!(rel.created(), now);

        let self_dep = Relationship::create(
            RelationshipId::from_bits(10),
            NewRelationship::new(RelationshipType::DependsOn, a, a),
            now,
        );
        assert!(self_dep.is_err());
        let self_ref = Relationship::create(
            RelationshipId::from_bits(11),
            NewRelationship::new(RelationshipType::References, a, a),
            now,
        );
        assert!(self_ref.is_ok());
    }
}
