//! The M1 exit scenario: artifacts and relationships created entirely in memory.

use functor_model::{
    Artifact, ArtifactId, ArtifactPatch, ArtifactRef, ArtifactType, Direction, FormatVersion,
    MonotonicUlids, NewArtifact, NewRelationship, Relationship, RelationshipId, RelationshipType,
    RepositoryId, RepositoryInfo, RepositoryName, RepositorySnapshot, SearchQuery, Slug, Timestamp,
    Title, Traversal,
};

fn ids() -> MonotonicUlids {
    MonotonicUlids::new()
}

#[test]
fn success_path_in_memory() {
    let ids = ids();
    let now = Timestamp::parse("2026-10-12T18:00:00Z").unwrap();

    let make = |slug: &str, title: &str| {
        Artifact::create(
            ArtifactId::from_bits(ids.next_bits()),
            NewArtifact::new(
                Slug::parse(slug).unwrap(),
                ArtifactType::new("rfc").unwrap(),
                Title::new(title).unwrap(),
            ),
            now,
        )
    };
    let vision = make("rfc-0000", "Functor Vision");
    let model = make("rfc-0001", "Core Semantic Model");

    let relationship = Relationship::create(
        RelationshipId::from_bits(ids.next_bits()),
        NewRelationship::new(
            RelationshipType::parse("depends-on").unwrap(),
            *model.id(),
            *vision.id(),
        ),
        now,
    )
    .unwrap();

    let info = RepositoryInfo::new(
        RepositoryId::from_bits(ids.next_bits()),
        RepositoryName::new("Demo").unwrap(),
        None,
        FormatVersion::CURRENT,
        now,
    );
    let snapshot = RepositorySnapshot::new(
        info,
        vec![model.clone(), vision.clone()],
        vec![relationship.clone()],
    )
    .unwrap();

    // Slugs and ids resolve to the same artifacts.
    let by_slug = snapshot
        .resolve(&ArtifactRef::parse("rfc-0001").unwrap())
        .unwrap();
    let by_id = snapshot.resolve(&ArtifactRef::Id(*model.id())).unwrap();
    assert_eq!(by_slug, by_id);

    // `rfc-0001 depends_on rfc-0000`: rfc-0000 is a dependency of rfc-0001 and rfc-0001 a
    // dependent of rfc-0000.
    let outgoing: Vec<_> = snapshot.outgoing(model.id()).collect();
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].target(), vision.id());
    let incoming: Vec<_> = snapshot.incoming(vision.id()).collect();
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].source(), model.id());

    // Query value types can be assembled for the later engine.
    let _search = SearchQuery::new().with_text("vision");
    let _traversal = Traversal::new(ArtifactRef::parse("rfc-0001").unwrap())
        .with_types([RelationshipType::DependsOn])
        .with_direction(Direction::Outbound)
        .with_max_depth(1);

    // Updating produces a new revision without touching identity.
    let renamed = model
        .apply_patch(
            ArtifactPatch::new().with_slug(Slug::parse("core-model").unwrap()),
            Timestamp::parse("2026-10-13T00:00:00Z").unwrap(),
        )
        .unwrap();
    assert_eq!(renamed.artifact.id(), model.id());
    assert_eq!(renamed.artifact.revision().get(), 2);

    // The relationship still points at the same id after the rename.
    let renamed_snapshot = RepositorySnapshot::new(
        snapshot.info().clone(),
        vec![renamed.artifact.clone(), vision],
        vec![relationship],
    )
    .unwrap();
    assert_eq!(renamed_snapshot.outgoing(model.id()).count(), 1);
    assert!(
        renamed_snapshot
            .resolve(&ArtifactRef::parse("rfc-0001").unwrap())
            .is_none()
    );
    assert!(
        renamed_snapshot
            .resolve(&ArtifactRef::parse("core-model").unwrap())
            .is_some()
    );
}
