//! `SnapshotQueryEngine` over the real stores: both must answer identically.

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::fs;

use functor_core::{
    ArtifactRepository, ErrorKind, InitOptions, QueryEngine, RelationshipRepository,
    RepositoryReader,
};
use functor_model::{
    ArtifactId, ArtifactRef, ArtifactType, Direction, MetadataKey, NewArtifact, NewRelationship,
    Query, QueryResult, RelationshipType, SearchQuery, Slug, Tag, Tags, Title, Traversal,
    TraversalResult,
};
use functor_query::SnapshotQueryEngine;
use functor_repository::FileRepositoryManager;
use functor_store::{FileStore, MemoryStore, StoreOptions};
use functor_testkit::Env;
use tempfile::TempDir;

fn artifact(slug: &str, artifact_type: &str) -> NewArtifact {
    NewArtifact::new(
        Slug::parse(slug).unwrap(),
        ArtifactType::new(artifact_type).unwrap(),
        Title::new(slug).unwrap(),
    )
}

/// `rfc-0001` depends on `rfc-0000`, `rfc-0002` on `rfc-0001`, and `notes` references `rfc-0000`.
async fn seed<S: ArtifactRepository + RelationshipRepository>(store: &S) {
    let rfc0 = ArtifactRepository::create(
        store,
        artifact("rfc-0000", "rfc")
            .with_tags(Tags::new(vec![Tag::new("core").unwrap()]))
            .with_metadata(
                [(MetadataKey::new("status").unwrap(), "draft".into())]
                    .into_iter()
                    .collect(),
            ),
    )
    .await
    .unwrap();
    let rfc1 = ArtifactRepository::create(store, artifact("rfc-0001", "rfc"))
        .await
        .unwrap();
    let rfc2 = ArtifactRepository::create(store, artifact("rfc-0002", "rfc"))
        .await
        .unwrap();
    let notes = ArtifactRepository::create(
        store,
        artifact("notes", "document").with_description("about the vision"),
    )
    .await
    .unwrap();
    for (source, relationship_type, target) in [
        (&rfc1, RelationshipType::DependsOn, &rfc0),
        (&rfc2, RelationshipType::DependsOn, &rfc1),
        (&notes, RelationshipType::References, &rfc0),
    ] {
        RelationshipRepository::create(
            store,
            NewRelationship::new(relationship_type, *source.id(), *target.id()),
        )
        .await
        .unwrap();
    }
}

fn describe(result: &TraversalResult) -> Vec<String> {
    let mut slugs: BTreeMap<ArtifactId, String> = result
        .nodes
        .iter()
        .map(|n| (*n.artifact.id(), n.artifact.slug().to_string()))
        .collect();
    slugs.insert(*result.start.id(), result.start.slug().to_string());
    let mut lines = vec![format!("start {}", result.start.slug())];
    for node in &result.nodes {
        lines.push(format!(
            "{}@{} via {}>{}:{}",
            node.artifact.slug(),
            node.depth,
            slugs[node.via.source()],
            slugs[node.via.target()],
            node.via.relationship_type()
        ));
    }
    lines.push(format!("edges {}", result.edges.len()));
    lines
}

/// Everything the engine can answer about the seeded repository, as text.
async fn transcript(engine: &dyn QueryEngine) -> Vec<String> {
    let mut lines = Vec::new();
    let reference = |text: &str| ArtifactRef::parse(text).unwrap();

    let found = engine.lookup(&reference("rfc-0001")).await.unwrap();
    lines.push(format!(
        "lookup rfc-0001 -> {:?}",
        found.map(|a| a.slug().to_string())
    ));
    let missing = engine.lookup(&reference("nope")).await.unwrap();
    lines.push(format!(
        "lookup nope -> {:?}",
        missing.map(|a| a.slug().to_string())
    ));

    let searches = [
        ("all", SearchQuery::new()),
        (
            "rfcs",
            SearchQuery::new().with_type(ArtifactType::new("rfc").unwrap()),
        ),
        (
            "tag core",
            SearchQuery::new().with_tag(Tag::new("core").unwrap()),
        ),
        ("text VISION", SearchQuery::new().with_text("VISION")),
        (
            "status=draft",
            SearchQuery::new().with_metadata(MetadataKey::new("status").unwrap(), "draft".into()),
        ),
    ];
    for (label, query) in searches {
        let slugs: Vec<String> = engine
            .search(&query)
            .await
            .unwrap()
            .iter()
            .map(|a| a.slug().to_string())
            .collect();
        lines.push(format!("search {label} -> {}", slugs.join(",")));
    }

    for (start, direction, depth) in [
        ("rfc-0002", Direction::Outbound, None),
        ("rfc-0000", Direction::Inbound, None),
        ("rfc-0000", Direction::Inbound, Some(1)),
        ("rfc-0001", Direction::Both, None),
    ] {
        let mut traversal = Traversal::new(reference(start))
            .with_types([RelationshipType::DependsOn])
            .with_direction(direction);
        traversal.max_depth = depth;
        let result = engine.traverse(&traversal).await.unwrap();
        lines.push(format!("traverse {start} {direction:?} {depth:?}"));
        lines.extend(describe(&result).into_iter().map(|l| format!("  {l}")));
    }
    let missing = engine
        .traverse(&Traversal::new(reference("nope")))
        .await
        .unwrap_err();
    lines.push(format!("traverse nope -> {}", missing.kind().as_str()));
    lines
}

fn file_store(env: &Env) -> (TempDir, FileStore) {
    let dir = tempfile::tempdir().unwrap();
    let manager = FileRepositoryManager::new(env.clock(), env.ids());
    let handle = manager
        .init_blocking(dir.path(), &InitOptions::new().with_name("Test"))
        .unwrap();
    let store = FileStore::with_options(&handle, env.clock(), env.ids(), StoreOptions::default());
    (dir, store)
}

#[tokio::test]
async fn memory_and_file_stores_answer_identically() {
    let env = Env::new();
    let memory = MemoryStore::new(env.clock(), env.ids());
    seed(&memory).await;
    let from_memory = transcript(&SnapshotQueryEngine::new(&memory)).await;

    let env = Env::new();
    let (_dir, file) = file_store(&env);
    seed(&file).await;
    let from_file = transcript(&SnapshotQueryEngine::new(&file)).await;

    assert_eq!(from_memory, from_file);
    assert_eq!(
        from_memory,
        [
            "lookup rfc-0001 -> Some(\"rfc-0001\")",
            "lookup nope -> None",
            "search all -> notes,rfc-0000,rfc-0001,rfc-0002",
            "search rfcs -> rfc-0000,rfc-0001,rfc-0002",
            "search tag core -> rfc-0000",
            "search text VISION -> notes",
            "search status=draft -> rfc-0000",
            "traverse rfc-0002 Outbound None",
            "  start rfc-0002",
            "  rfc-0001@1 via rfc-0002>rfc-0001:depends_on",
            "  rfc-0000@2 via rfc-0001>rfc-0000:depends_on",
            "  edges 2",
            "traverse rfc-0000 Inbound None",
            "  start rfc-0000",
            "  rfc-0001@1 via rfc-0001>rfc-0000:depends_on",
            "  rfc-0002@2 via rfc-0002>rfc-0001:depends_on",
            "  edges 2",
            "traverse rfc-0000 Inbound Some(1)",
            "  start rfc-0000",
            "  rfc-0001@1 via rfc-0001>rfc-0000:depends_on",
            "  edges 1",
            "traverse rfc-0001 Both None",
            "  start rfc-0001",
            "  rfc-0000@1 via rfc-0001>rfc-0000:depends_on",
            "  rfc-0002@1 via rfc-0002>rfc-0001:depends_on",
            "  edges 2",
            "traverse nope -> not_found",
        ]
    );
}

#[tokio::test]
async fn the_engine_works_through_trait_objects_and_execute() {
    let env = Env::new();
    let store = MemoryStore::new(env.clock(), env.ids());
    seed(&store).await;

    let reader: &dyn RepositoryReader = &store;
    let engine = SnapshotQueryEngine::new(reader);
    let dynamic: &dyn QueryEngine = &engine;

    let QueryResult::Lookup(Some(found)) = dynamic
        .execute(Query::Lookup(ArtifactRef::parse("notes").unwrap()))
        .await
        .unwrap()
    else {
        panic!("expected a lookup result");
    };
    assert_eq!(found.slug().as_str(), "notes");

    let QueryResult::Search(all) = dynamic
        .execute(Query::Search(SearchQuery::new()))
        .await
        .unwrap()
    else {
        panic!("expected a search result");
    };
    assert_eq!(all.len(), 4);

    let QueryResult::Traverse(result) = dynamic
        .execute(Query::Traverse(Traversal::new(
            ArtifactRef::parse("rfc-0002").unwrap(),
        )))
        .await
        .unwrap()
    else {
        panic!("expected a traversal result");
    };
    assert_eq!(result.nodes.len(), 2);
}

#[tokio::test]
async fn relationships_come_with_the_artifact_in_id_order() {
    let env = Env::new();
    let store = MemoryStore::new(env.clock(), env.ids());
    seed(&store).await;
    let engine = SnapshotQueryEngine::new(&store);
    let reference = ArtifactRef::parse("rfc-0000").unwrap();

    let (artifact, both) = engine
        .relationships(&reference, &[], Direction::Both)
        .await
        .unwrap();
    assert_eq!(artifact.slug().as_str(), "rfc-0000");
    assert_eq!(both.len(), 2);
    assert!(both[0].id() < both[1].id());

    let (_, depends) = engine
        .relationships(
            &reference,
            &[RelationshipType::DependsOn],
            Direction::Inbound,
        )
        .await
        .unwrap();
    assert_eq!(depends.len(), 1);

    let error = engine
        .relationships(&ArtifactRef::parse("nope").unwrap(), &[], Direction::Both)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[tokio::test]
async fn every_engine_call_reads_exactly_one_snapshot() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use functor_core::{Result, SnapshotLoad};
    use functor_model::RepositoryInfo;

    struct Counting<'a> {
        inner: &'a MemoryStore,
        loads: AtomicUsize,
    }

    #[async_trait]
    impl RepositoryReader for Counting<'_> {
        async fn info(&self) -> Result<RepositoryInfo> {
            self.inner.info().await
        }

        async fn load(&self) -> Result<SnapshotLoad> {
            self.loads.fetch_add(1, Ordering::SeqCst);
            self.inner.load().await
        }
    }

    let env = Env::new();
    let store = MemoryStore::new(env.clock(), env.ids());
    seed(&store).await;
    let counting = Counting {
        inner: &store,
        loads: AtomicUsize::new(0),
    };
    let engine = SnapshotQueryEngine::new(&counting);
    let reference = ArtifactRef::parse("rfc-0002").unwrap();
    let loads = || counting.loads.load(Ordering::SeqCst);

    engine.lookup(&reference).await.unwrap();
    assert_eq!(loads(), 1);
    engine.search(&SearchQuery::new()).await.unwrap();
    assert_eq!(loads(), 2);
    engine
        .traverse(&Traversal::new(reference.clone()))
        .await
        .unwrap();
    assert_eq!(loads(), 3);
    engine
        .relationships(&reference, &[], Direction::Both)
        .await
        .unwrap();
    assert_eq!(loads(), 4);
}

#[tokio::test]
async fn an_inconsistent_repository_fails_every_query_with_inconsistent() {
    let env = Env::new();
    let (dir, store) = file_store(&env);
    seed(&store).await;
    let victim = *ArtifactRepository::resolve(&store, &ArtifactRef::parse("rfc-0000").unwrap())
        .await
        .unwrap()
        .unwrap()
        .id();
    fs::remove_file(dir.path().join(format!("artifacts/{victim}.toml"))).unwrap();

    let engine = SnapshotQueryEngine::new(&store);
    let reference = ArtifactRef::parse("notes").unwrap();
    assert_eq!(
        engine.lookup(&reference).await.unwrap_err().kind(),
        ErrorKind::Inconsistent
    );
    assert_eq!(
        engine.search(&SearchQuery::new()).await.unwrap_err().kind(),
        ErrorKind::Inconsistent
    );
    assert_eq!(
        engine
            .traverse(&Traversal::new(reference.clone()))
            .await
            .unwrap_err()
            .kind(),
        ErrorKind::Inconsistent
    );
    assert_eq!(
        engine
            .relationships(&reference, &[], Direction::Both)
            .await
            .unwrap_err()
            .kind(),
        ErrorKind::Inconsistent
    );
}
