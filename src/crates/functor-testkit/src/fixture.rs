//! Builders for small in-memory repositories used by tests and examples.

// Fixture builders are test support: a bad slug or unknown reference is a bug in the test, so
// they panic with a clear message instead of returning errors.
#![allow(clippy::expect_used, clippy::missing_panics_doc)]

use functor_core::{Clock, IdGenerator};
use functor_model::{
    Artifact, ArtifactType, FormatVersion, NewArtifact, NewRelationship, Relationship,
    RelationshipType, RepositoryInfo, RepositoryName, RepositorySnapshot, Slug, Timestamp, Title,
};

use crate::doubles::{FixedClock, SequentialIds};

/// The instant fixtures start at.
pub const FIXTURE_START: &str = "2026-10-12T18:00:00Z";

/// Builds a small repository: `RepoFixture::new().artifact("a").artifact("b").relation("b",
/// "depends_on", "a").build()`.
///
/// Ids and timestamps are deterministic. Each artifact and relationship is created one second
/// after the previous one.
#[derive(Debug)]
pub struct RepoFixture {
    clock: FixedClock,
    ids: SequentialIds,
    info: RepositoryInfo,
    artifacts: Vec<Artifact>,
    relationships: Vec<Relationship>,
}

impl Default for RepoFixture {
    fn default() -> Self {
        Self::new()
    }
}

impl RepoFixture {
    /// Starts an empty fixture repository named `Fixture`.
    #[must_use]
    pub fn new() -> Self {
        let start = Timestamp::parse(FIXTURE_START).expect("valid fixture start");
        let clock = FixedClock::new(start);
        let ids = SequentialIds::new();
        let info = RepositoryInfo::new(
            ids.repository_id(),
            RepositoryName::new("Fixture").expect("valid name"),
            None,
            FormatVersion::CURRENT,
            start,
        );
        Self {
            clock,
            ids,
            info,
            artifacts: Vec::new(),
            relationships: Vec::new(),
        }
    }

    /// Adds a `document` artifact whose title equals its slug.
    #[must_use]
    pub fn artifact(self, slug: &str) -> Self {
        self.artifact_of_type(slug, ArtifactType::DOCUMENT)
    }

    /// Adds an artifact of the given type whose title equals its slug.
    #[must_use]
    pub fn artifact_of_type(mut self, slug: &str, artifact_type: &str) -> Self {
        self.clock.advance(1);
        let new = NewArtifact::new(
            Slug::parse(slug).expect("valid fixture slug"),
            ArtifactType::new(artifact_type).expect("valid fixture type"),
            Title::new(slug).expect("valid fixture title"),
        );
        self.artifacts.push(Artifact::create(
            self.ids.artifact_id(),
            new,
            self.clock.now(),
        ));
        self
    }

    /// Adds a relationship between two existing artifacts named by slug.
    ///
    /// # Panics
    /// Panics when a slug is unknown or the relationship type is malformed or self-referencing.
    #[must_use]
    pub fn relation(mut self, source: &str, relationship_type: &str, target: &str) -> Self {
        let find = |slug: &str| {
            *self
                .artifacts
                .iter()
                .find(|a| a.slug().as_str() == slug)
                .unwrap_or_else(|| panic!("fixture has no artifact `{slug}`"))
                .id()
        };
        let (source_id, target_id) = (find(source), find(target));
        self.clock.advance(1);
        let new = NewRelationship::new(
            RelationshipType::parse(relationship_type).expect("valid relationship type"),
            source_id,
            target_id,
        );
        self.relationships.push(
            Relationship::create(self.ids.relationship_id(), new, self.clock.now())
                .expect("valid fixture relationship"),
        );
        self
    }

    /// The artifacts built so far.
    #[must_use]
    pub fn artifacts(&self) -> &[Artifact] {
        &self.artifacts
    }

    /// The relationships built so far.
    #[must_use]
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    /// The repository information.
    #[must_use]
    pub fn info(&self) -> &RepositoryInfo {
        &self.info
    }

    /// Looks up the id of an artifact by slug.
    #[must_use]
    pub fn id_of(&self, slug: &str) -> Option<functor_model::ArtifactId> {
        self.artifacts
            .iter()
            .find(|a| a.slug().as_str() == slug)
            .map(|a| *a.id())
    }

    /// Builds the strict snapshot.
    #[must_use]
    pub fn build(&self) -> RepositorySnapshot {
        RepositorySnapshot::new(
            self.info.clone(),
            self.artifacts.clone(),
            self.relationships.clone(),
        )
        .expect("fixtures are well-formed by construction")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_connected_snapshot() {
        let fixture = RepoFixture::new()
            .artifact("rfc-0000")
            .artifact_of_type("rfc-0001", "rfc")
            .relation("rfc-0001", "depends-on", "rfc-0000");
        let snapshot = fixture.build();
        assert_eq!(snapshot.artifacts().len(), 2);
        assert_eq!(snapshot.relationships().len(), 1);
        let dependent = fixture.id_of("rfc-0001").unwrap();
        assert_eq!(snapshot.outgoing(&dependent).count(), 1);
        assert_eq!(fixture.id_of("missing"), None);
    }

    #[test]
    fn fixtures_are_deterministic() {
        let build = || {
            RepoFixture::new()
                .artifact("a")
                .artifact("b")
                .relation("a", "references", "b")
        };
        let (one, two) = (build(), build());
        assert_eq!(one.artifacts(), two.artifacts());
        assert_eq!(one.relationships(), two.relationships());
        assert_eq!(one.info(), two.info());
    }

    #[test]
    #[should_panic(expected = "fixture has no artifact `nope`")]
    fn unknown_slug_panics_clearly() {
        let _ = RepoFixture::new()
            .artifact("a")
            .relation("a", "references", "nope");
    }
}
