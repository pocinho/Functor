//! The MVP success path (and its negative variant) run through the libraries only, without the
//! CLI. It proves that every capability of the walkthrough is reachable through the service
//! traits (exit criterion 6 of `docs/planning/mvp-scope.md`), and that the libraries and the CLI read
//! and write the same repositories.

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use functor_core::{
    ArtifactRepository, EventPosition, EventPublisher, EventSource, QueryEngine,
    RelationshipRepository, RepositoryHandle, RepositoryManager, ValidationEngine,
    ValidationTarget,
};
use functor_core::{ErrorKind, InitOptions};
use functor_events::{EventLogOptions, FileEventLog};
use functor_model::{
    Artifact, ArtifactRef, ArtifactType, Direction, EventData, EventKind, EventSubject,
    NewArtifact, NewEvent, NewRelationship, Outcome, RelationshipType, Slug, Title, Traversal,
};
use functor_query::SnapshotQueryEngine;
use functor_repository::FileRepositoryManager;
use functor_store::{FileStore, StoreOptions};
use functor_testkit::Env;
use functor_validation::RuleValidationEngine;

fn slug(text: &str) -> Slug {
    Slug::parse(text).unwrap()
}

fn artifact(text: &str, title: &str) -> NewArtifact {
    NewArtifact::new(
        slug(text),
        ArtifactType::new("rfc").unwrap(),
        Title::new(title).unwrap(),
    )
}

fn depends_on() -> RelationshipType {
    RelationshipType::parse("depends_on").unwrap()
}

/// A repository built with the libraries only: `rfc-0001 depends_on rfc-0000`.
async fn library_repository(at: &Path, env: &Env) -> (RepositoryHandle, Artifact, Artifact) {
    let mut options = InitOptions::new();
    options.name = Some("Functor Demo".to_owned());
    let handle = FileRepositoryManager::new(env.clock(), env.ids())
        .init(at, options)
        .await
        .unwrap();
    let store = FileStore::with_options(&handle, env.clock(), env.ids(), StoreOptions::default());
    let vision = ArtifactRepository::create(&store, artifact("rfc-0000", "Functor Vision"))
        .await
        .unwrap();
    let model = ArtifactRepository::create(&store, artifact("rfc-0001", "Core Semantic Model"))
        .await
        .unwrap();
    RelationshipRepository::create(
        &store,
        NewRelationship::new(depends_on(), *model.id(), *vision.id()),
    )
    .await
    .unwrap();
    (handle, vision, model)
}

fn functor(dir: &Path, args: &[&str]) -> std::process::Output {
    assert_cmd::Command::cargo_bin("functor")
        .unwrap()
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

fn json(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Copies a directory tree, so that a test never writes into the working tree.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[tokio::test]
async fn a_repository_written_by_the_libraries_is_read_by_the_cli() {
    let dir = tempfile::tempdir().unwrap();
    let (_, vision, model) = library_repository(dir.path(), &Env::new()).await;

    let shown = json(&functor(
        dir.path(),
        &["artifact", "show", "rfc-0001", "--output", "json"],
    ));
    assert_eq!(shown["artifact"]["id"], model.id().to_string());

    let dependencies = json(&functor(
        dir.path(),
        &["query", "dependencies", "rfc-0001", "--output", "json"],
    ));
    assert_eq!(
        dependencies["nodes"][0]["artifact"]["id"],
        vision.id().to_string()
    );

    let validated = json(&functor(dir.path(), &["validate", "--output", "json"]));
    assert_eq!(validated["outcome"], "passed");
}

#[tokio::test]
async fn a_repository_written_by_the_cli_is_read_by_the_libraries() {
    // The reference repository is what the CLI produced in the acceptance test.
    let example =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../examples/getting-started");
    let dir = tempfile::tempdir().unwrap();
    copy_tree(&example, dir.path());

    let env = Env::new();
    let handle = FileRepositoryManager::new(env.clock(), env.ids())
        .discover(dir.path())
        .await
        .unwrap();
    let store = FileStore::with_options(&handle, env.clock(), env.ids(), StoreOptions::default());
    let slugs: Vec<String> = ArtifactRepository::list(&store)
        .await
        .unwrap()
        .iter()
        .map(|artifact| artifact.slug().to_string())
        .collect();
    assert_eq!(slugs, ["rfc-0000", "rfc-0001"]);
    assert_eq!(RelationshipRepository::list(&store).await.unwrap().len(), 1);
    let report = RuleValidationEngine::new(&store, env.clock())
        .validate(ValidationTarget::Repository)
        .await
        .unwrap();
    assert_eq!(report.outcome(), Outcome::Passed);
}

#[tokio::test]
async fn a_moved_repository_keeps_every_identity() {
    let parent = tempfile::tempdir().unwrap();
    let before = parent.path().join("before");
    let after = parent.path().join("after");
    let env = Env::new();
    let (handle, vision, model) = library_repository(&before, &env).await;
    std::fs::rename(&before, &after).unwrap();

    let moved = FileRepositoryManager::new(env.clock(), env.ids())
        .discover(&after)
        .await
        .unwrap();
    assert_eq!(moved.info.id(), handle.info.id());
    let store = FileStore::with_options(&moved, env.clock(), env.ids(), StoreOptions::default());
    let ids: Vec<_> = ArtifactRepository::list(&store)
        .await
        .unwrap()
        .iter()
        .map(|artifact| *artifact.id())
        .collect();
    let mut expected = [*vision.id(), *model.id()];
    expected.sort();
    assert_eq!(ids, expected);
}

#[tokio::test]
async fn repository_metadata_never_changes_what_the_repository_means() {
    let dir = tempfile::tempdir().unwrap();
    let env = Env::new();
    let (handle, _, _) = library_repository(dir.path(), &env).await;

    let dependencies = || {
        json(&functor(
            dir.path(),
            &["query", "dependencies", "rfc-0001", "--output", "json"],
        ))
    };
    let original = dependencies();

    let file = handle.root.join(".functor").join("repository.toml");
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(
        &file,
        text.replace("name = \"Functor Demo\"", "name = \"Another Name\""),
    )
    .unwrap();

    assert_eq!(original, dependencies());
    let validated = json(&functor(dir.path(), &["validate", "--output", "json"]));
    assert_eq!(validated["outcome"], "passed");
}

// One narrative, step by step, like the walkthrough it mirrors.
#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn the_success_path_and_the_negative_variant_run_through_the_libraries() {
    let dir = tempfile::tempdir().unwrap();
    let env = Env::new();

    // init
    let mut options = InitOptions::new();
    options.name = Some("Functor Demo".to_owned());
    let handle = FileRepositoryManager::new(env.clock(), env.ids())
        .init(dir.path(), options)
        .await
        .unwrap();
    let store = FileStore::with_options(&handle, env.clock(), env.ids(), StoreOptions::default());
    let log = FileEventLog::new(
        &handle.root,
        *handle.info.id(),
        env.clock(),
        env.ids(),
        EventLogOptions::default(),
    );

    // artifact create, relation add; every change is published
    let vision = ArtifactRepository::create(&store, artifact("rfc-0000", "Functor Vision"))
        .await
        .unwrap();
    log.publish(NewEvent::new(
        EventKind::ArtifactCreated,
        EventSubject::Artifact(*vision.id()),
        EventData::new(),
    ))
    .await
    .unwrap();
    let model = ArtifactRepository::create(&store, artifact("rfc-0001", "Core Semantic Model"))
        .await
        .unwrap();
    log.publish(NewEvent::new(
        EventKind::ArtifactCreated,
        EventSubject::Artifact(*model.id()),
        EventData::new(),
    ))
    .await
    .unwrap();
    let relationship = RelationshipRepository::create(
        &store,
        NewRelationship::new(depends_on(), *model.id(), *vision.id()),
    )
    .await
    .unwrap();
    log.publish(NewEvent::new(
        EventKind::RelationshipCreated,
        EventSubject::Relationship(*relationship.id()),
        EventData::new(),
    ))
    .await
    .unwrap();

    // query dependencies and dependents
    let engine = SnapshotQueryEngine::new(&store);
    let dependencies = engine
        .traverse(
            &Traversal::new(ArtifactRef::parse("rfc-0001").unwrap())
                .with_types([depends_on()])
                .with_max_depth(1),
        )
        .await
        .unwrap();
    let reached: Vec<&str> = dependencies
        .nodes
        .iter()
        .map(|node| node.artifact.slug().as_str())
        .collect();
    assert_eq!(reached, ["rfc-0000"]);

    let dependents = engine
        .traverse(
            &Traversal::new(ArtifactRef::parse("rfc-0000").unwrap())
                .with_types([depends_on()])
                .with_direction(Direction::Inbound)
                .with_max_depth(1),
        )
        .await
        .unwrap();
    let reached: Vec<&str> = dependents
        .nodes
        .iter()
        .map(|node| node.artifact.slug().as_str())
        .collect();
    assert_eq!(reached, ["rfc-0001"]);

    // validate
    let validation = RuleValidationEngine::new(&store, env.clock());
    let report = validation
        .validate(ValidationTarget::Repository)
        .await
        .unwrap();
    assert_eq!(report.outcome(), Outcome::Passed);
    log.publish(NewEvent::new(
        EventKind::ValidationCompleted,
        EventSubject::Repository(*handle.info.id()),
        EventData::new(),
    ))
    .await
    .unwrap();

    // watch: the four events arrive in order
    let mut subscription = log.subscribe(EventPosition::Start).await.unwrap();
    let mut kinds = Vec::new();
    for expected_seq in 1..=4 {
        let event = tokio::time::timeout(Duration::from_secs(10), subscription.next())
            .await
            .expect("the event arrives")
            .unwrap()
            .expect("the log is open");
        assert_eq!(event.seq, expected_seq);
        kinds.push(event.kind);
    }
    assert_eq!(
        kinds,
        [
            EventKind::ArtifactCreated,
            EventKind::ArtifactCreated,
            EventKind::RelationshipCreated,
            EventKind::ValidationCompleted,
        ]
    );

    // negative variant: removing an artifact file leaves a dangling relationship
    let file = handle
        .root
        .join("artifacts")
        .join(format!("{}.toml", vision.id()));
    std::fs::remove_file(&file).unwrap();
    let report = validation
        .validate(ValidationTarget::Repository)
        .await
        .unwrap();
    assert_eq!(report.outcome(), Outcome::Failed);
    assert!(
        report
            .findings()
            .iter()
            .any(|finding| finding.code.as_str() == "V0007"),
        "{:?}",
        report.findings()
    );

    // and the strict reads refuse the damaged repository
    let error = ArtifactRepository::list(&store).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Inconsistent);
}
