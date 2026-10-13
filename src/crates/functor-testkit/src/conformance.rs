//! The store conformance suite.
//!
//! Every implementation of the repository state traits (`RepositoryReader`, `ArtifactRepository`
//! and `RelationshipRepository`) must pass [`run_store_conformance`]. This is what makes the
//! storage independence of RFC-0008 enforceable: `MemoryStore` and `FileStore` run the same
//! scenarios. See `docs/quality/testing-strategy.md`.
//!
//! The suite is runtime-agnostic: it returns futures, and the calling test drives them (for
//! example with `#[tokio::test]`).

// The scenarios are assertions: they panic on failure by design.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::missing_panics_doc)]

use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

use functor_core::{
    ArtifactRepository, Clock, ErrorKind, IdGenerator, RelationshipRepository, RepositoryReader,
    Result, SnapshotLoad,
};
use functor_model::{
    Artifact, ArtifactField, ArtifactId, ArtifactPatch, ArtifactRef, ArtifactType, NewArtifact,
    NewRelationship, Patched, Relationship, RelationshipId, RelationshipType, RepositoryInfo,
    RepositorySnapshot, Revision, Slug, Timestamp, Title,
};

use crate::doubles::{FixedClock, SequentialIds};

/// The traits a store must implement to be tested.
pub trait StoreUnderTest: RepositoryReader + ArtifactRepository + RelationshipRepository {}

impl<T> StoreUnderTest for T where T: RepositoryReader + ArtifactRepository + RelationshipRepository {}

/// The injected collaborators a store under test must use.
///
/// Stores must take their time from [`Env::clock`] and their ids from [`Env::ids`], so the suite
/// can assert exact timestamps and compare runs.
#[derive(Debug, Clone)]
pub struct Env {
    fixed_clock: Arc<FixedClock>,
    sequential_ids: Arc<SequentialIds>,
}

impl Env {
    /// Creates a fresh environment with the clock at `2026-10-12T18:00:00Z`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fixed_clock: Arc::new(FixedClock::new(start())),
            sequential_ids: Arc::new(SequentialIds::new()),
        }
    }

    /// The clock the store must use.
    #[must_use]
    pub fn clock(&self) -> Arc<dyn Clock> {
        Arc::clone(&self.fixed_clock) as Arc<dyn Clock>
    }

    /// The id generator the store must use.
    #[must_use]
    pub fn ids(&self) -> Arc<dyn IdGenerator> {
        Arc::clone(&self.sequential_ids) as Arc<dyn IdGenerator>
    }

    /// Moves the shared clock forward.
    pub fn advance(&self, seconds: i64) {
        self.fixed_clock.advance(seconds);
    }

    /// The current time according to the shared clock.
    #[must_use]
    pub fn now(&self) -> Timestamp {
        self.fixed_clock.now()
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

/// A store plus anything that must stay alive while it is used (for example a temporary
/// directory).
pub struct Fresh<S> {
    /// The empty store.
    pub store: S,
    /// Kept alive until the scenario ends.
    pub guard: Option<Box<dyn Any + Send>>,
}

impl<S> Fresh<S> {
    /// Wraps a store with no guard.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self { store, guard: None }
    }

    /// Wraps a store and a guard to keep alive.
    #[must_use]
    pub fn with_guard(store: S, guard: impl Any + Send) -> Self {
        Self {
            store,
            guard: Some(Box::new(guard)),
        }
    }
}

/// A view of a store with one unambiguous method per operation.
///
/// `create`, `get` and `list` exist on both repository traits, so scenarios call these instead.
#[derive(Debug)]
pub struct Ctx<'a, S>(pub &'a S);

impl<S: StoreUnderTest> Ctx<'_, S> {
    async fn create_artifact(&self, new: NewArtifact) -> Result<Artifact> {
        ArtifactRepository::create(self.0, new).await
    }

    async fn get_artifact(&self, id: &ArtifactId) -> Result<Option<Artifact>> {
        ArtifactRepository::get(self.0, id).await
    }

    async fn resolve(&self, reference: &ArtifactRef) -> Result<Option<Artifact>> {
        ArtifactRepository::resolve(self.0, reference).await
    }

    async fn list_artifacts(&self) -> Result<Vec<Artifact>> {
        ArtifactRepository::list(self.0).await
    }

    async fn update_artifact(
        &self,
        id: &ArtifactId,
        patch: ArtifactPatch,
        expected: Option<Revision>,
    ) -> Result<Patched> {
        ArtifactRepository::update(self.0, id, patch, expected).await
    }

    async fn create_relationship(&self, new: NewRelationship) -> Result<Relationship> {
        RelationshipRepository::create(self.0, new).await
    }

    async fn get_relationship(&self, id: &RelationshipId) -> Result<Option<Relationship>> {
        RelationshipRepository::get(self.0, id).await
    }

    async fn list_relationships(&self) -> Result<Vec<Relationship>> {
        RelationshipRepository::list(self.0).await
    }

    async fn snapshot(&self) -> Result<RepositorySnapshot> {
        RepositoryReader::snapshot(self.0).await
    }

    async fn load(&self) -> Result<SnapshotLoad> {
        RepositoryReader::load(self.0).await
    }

    async fn info(&self) -> Result<RepositoryInfo> {
        RepositoryReader::info(self.0).await
    }

    async fn depends_on(&self, source: &Artifact, target: &Artifact) -> Relationship {
        self.create_relationship(NewRelationship::new(
            RelationshipType::DependsOn,
            *source.id(),
            *target.id(),
        ))
        .await
        .expect("create relationship")
    }
}

fn start() -> Timestamp {
    Timestamp::parse("2026-10-12T18:00:00Z").unwrap()
}

fn slug(text: &str) -> Slug {
    Slug::parse(text).unwrap()
}

fn new_artifact(text: &str) -> NewArtifact {
    NewArtifact::new(
        slug(text),
        ArtifactType::document(),
        Title::new(text).unwrap(),
    )
}

fn by_slug(text: &str) -> ArtifactRef {
    ArtifactRef::Slug(slug(text))
}

fn retitle(title: &str) -> ArtifactPatch {
    ArtifactPatch::new().with_title(Title::new(title).unwrap())
}

fn expect_err<T: Debug>(result: Result<T>, kind: ErrorKind, context: &str) {
    match result {
        Ok(value) => panic!("{context}: expected a {kind:?} error, got Ok({value:?})"),
        Err(error) => assert_eq!(error.kind(), kind, "{context}: got `{error}`"),
    }
}

/// Runs every scenario, each against a fresh empty store built by `factory`.
///
/// `factory` receives the [`Env`] the store must use for time and ids.
pub async fn run_store_conformance<S, F>(factory: F)
where
    S: StoreUnderTest,
    F: Fn(&Env) -> Fresh<S>,
{
    macro_rules! scenario {
        ($name:ident) => {{
            let env = Env::new();
            let fresh = factory(&env);
            $name(&Ctx(&fresh.store), &env).await;
            drop(fresh);
        }};
    }

    scenario!(create_get_resolve_and_list);
    scenario!(duplicate_slug_is_a_conflict);
    scenario!(update_increments_revision_and_preserves_identity);
    scenario!(no_op_update_changes_nothing);
    scenario!(slug_rename_releases_the_old_slug_and_keeps_relationships);
    scenario!(rename_to_a_taken_slug_conflicts);
    scenario!(expected_revision_is_enforced);
    scenario!(update_of_unknown_artifact_is_not_found);
    scenario!(relationships_require_existing_endpoints);
    scenario!(relationship_rules);
    scenario!(relationship_get_and_list_order);
    scenario!(snapshot_reflects_state);
    scenario!(load_of_a_clean_repository_has_no_issues);
    scenario!(info_is_stable);
    determinism(&factory).await;
}

/// Created artifacts start at revision 1 with the injected time and can be found by id and slug.
pub async fn create_get_resolve_and_list<S: StoreUnderTest>(store: &Ctx<'_, S>, env: &Env) {
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    env.advance(5);
    let b = store.create_artifact(new_artifact("beta")).await.unwrap();

    assert_eq!(
        a.revision(),
        Revision::FIRST,
        "new artifacts start at revision 1"
    );
    assert_eq!(a.created(), start());
    assert_eq!(a.updated(), start(), "created == updated on creation");
    assert_eq!(b.created(), env.now());
    assert_ne!(a.id(), b.id());
    assert_eq!(a.slug().as_str(), "alpha");

    assert_eq!(store.get_artifact(a.id()).await.unwrap().as_ref(), Some(&a));
    assert_eq!(
        store
            .get_artifact(&ArtifactId::from_bits(9999))
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        store
            .resolve(&ArtifactRef::Id(*a.id()))
            .await
            .unwrap()
            .as_ref(),
        Some(&a)
    );
    assert_eq!(
        store.resolve(&by_slug("alpha")).await.unwrap().as_ref(),
        Some(&a)
    );
    assert_eq!(
        store.resolve(&by_slug("beta")).await.unwrap().as_ref(),
        Some(&b)
    );
    assert_eq!(store.resolve(&by_slug("nope")).await.unwrap(), None);

    let listed = store.list_artifacts().await.unwrap();
    assert_eq!(
        listed,
        [a, b],
        "list is ordered by id, which is creation order"
    );
}

/// Slugs are unique within a repository.
pub async fn duplicate_slug_is_a_conflict<S: StoreUnderTest>(store: &Ctx<'_, S>, _env: &Env) {
    store.create_artifact(new_artifact("alpha")).await.unwrap();
    expect_err(
        store.create_artifact(new_artifact("alpha")).await,
        ErrorKind::Conflict,
        "second artifact with the same slug",
    );
    assert_eq!(
        store.list_artifacts().await.unwrap().len(),
        1,
        "the failed create wrote nothing"
    );
}

/// An update bumps the revision and `updated`, reports the changed fields and keeps identity.
pub async fn update_increments_revision_and_preserves_identity<S: StoreUnderTest>(
    store: &Ctx<'_, S>,
    env: &Env,
) {
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    env.advance(60);
    let patched = store
        .update_artifact(a.id(), retitle("Alpha Two"), None)
        .await
        .unwrap();

    assert_eq!(patched.changed, [ArtifactField::Title]);
    let updated = &patched.artifact;
    assert_eq!(updated.revision().get(), 2);
    assert_eq!(updated.id(), a.id());
    assert_eq!(updated.created(), a.created());
    assert_eq!(updated.updated(), env.now());
    assert_eq!(updated.artifact_type(), a.artifact_type());
    assert_eq!(updated.title().as_str(), "Alpha Two");
    assert_eq!(
        store.get_artifact(a.id()).await.unwrap().as_ref(),
        Some(updated)
    );
    assert_eq!(store.list_artifacts().await.unwrap().len(), 1);
}

/// A patch that changes nothing writes nothing: no new revision, no new timestamp.
pub async fn no_op_update_changes_nothing<S: StoreUnderTest>(store: &Ctx<'_, S>, env: &Env) {
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    env.advance(60);
    let same = ArtifactPatch::new()
        .with_title(a.title().clone())
        .with_slug(a.slug().clone());
    let patched = store.update_artifact(a.id(), same, None).await.unwrap();
    assert_eq!(patched.changed, []);
    assert_eq!(patched.artifact, a);
    assert_eq!(store.get_artifact(a.id()).await.unwrap().as_ref(), Some(&a));
}

/// Renaming a slug frees the old one, and relationships still point at the same identity.
pub async fn slug_rename_releases_the_old_slug_and_keeps_relationships<S: StoreUnderTest>(
    store: &Ctx<'_, S>,
    env: &Env,
) {
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    let b = store.create_artifact(new_artifact("beta")).await.unwrap();
    let rel = store.depends_on(&a, &b).await;
    env.advance(1);

    let renamed = store
        .update_artifact(a.id(), ArtifactPatch::new().with_slug(slug("gamma")), None)
        .await
        .unwrap();
    assert_eq!(renamed.changed, [ArtifactField::Slug]);

    assert_eq!(store.resolve(&by_slug("alpha")).await.unwrap(), None);
    let resolved = store.resolve(&by_slug("gamma")).await.unwrap().unwrap();
    assert_eq!(resolved.id(), a.id());
    assert_eq!(store.list_artifacts().await.unwrap().len(), 2);
    assert_eq!(
        store.list_relationships().await.unwrap(),
        [rel],
        "relationships are untouched by a slug rename"
    );

    let reused = store.create_artifact(new_artifact("alpha")).await.unwrap();
    assert_ne!(
        reused.id(),
        a.id(),
        "the old slug is free to reuse for a new artifact"
    );
}

/// A rename cannot take another artifact's slug.
pub async fn rename_to_a_taken_slug_conflicts<S: StoreUnderTest>(store: &Ctx<'_, S>, _env: &Env) {
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    store.create_artifact(new_artifact("beta")).await.unwrap();
    expect_err(
        store
            .update_artifact(a.id(), ArtifactPatch::new().with_slug(slug("beta")), None)
            .await,
        ErrorKind::Conflict,
        "renaming to a taken slug",
    );
    assert_eq!(
        store.get_artifact(a.id()).await.unwrap().as_ref(),
        Some(&a),
        "nothing was written"
    );
}

/// Optimistic concurrency: a stale expected revision conflicts without writing.
pub async fn expected_revision_is_enforced<S: StoreUnderTest>(store: &Ctx<'_, S>, env: &Env) {
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    env.advance(1);

    let stale = Some(Revision::new(2).unwrap());
    expect_err(
        store
            .update_artifact(a.id(), retitle("Changed"), stale)
            .await,
        ErrorKind::Conflict,
        "stale expected revision",
    );
    assert_eq!(
        store.get_artifact(a.id()).await.unwrap().as_ref(),
        Some(&a),
        "nothing was written"
    );

    let current = Some(Revision::FIRST);
    let patched = store
        .update_artifact(a.id(), retitle("Changed"), current)
        .await
        .unwrap();
    assert_eq!(patched.artifact.revision().get(), 2);
}

/// Updating an artifact that does not exist is `NotFound`.
pub async fn update_of_unknown_artifact_is_not_found<S: StoreUnderTest>(
    store: &Ctx<'_, S>,
    _env: &Env,
) {
    expect_err(
        store
            .update_artifact(&ArtifactId::from_bits(4242), retitle("x"), None)
            .await,
        ErrorKind::NotFound,
        "update of an unknown artifact",
    );
}

/// A relationship needs both endpoints to exist.
pub async fn relationships_require_existing_endpoints<S: StoreUnderTest>(
    store: &Ctx<'_, S>,
    _env: &Env,
) {
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    let ghost = ArtifactId::from_bits(4242);
    expect_err(
        store
            .create_relationship(NewRelationship::new(
                RelationshipType::DependsOn,
                *a.id(),
                ghost,
            ))
            .await,
        ErrorKind::NotFound,
        "missing target",
    );
    expect_err(
        store
            .create_relationship(NewRelationship::new(
                RelationshipType::DependsOn,
                ghost,
                *a.id(),
            ))
            .await,
        ErrorKind::NotFound,
        "missing source",
    );
    assert_eq!(store.list_relationships().await.unwrap(), []);
}

/// Self-reference, duplicates, direction and types.
pub async fn relationship_rules<S: StoreUnderTest>(store: &Ctx<'_, S>, _env: &Env) {
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    let b = store.create_artifact(new_artifact("beta")).await.unwrap();
    let make = |kind: RelationshipType, from: &Artifact, to: &Artifact| {
        NewRelationship::new(kind, *from.id(), *to.id())
    };

    expect_err(
        store
            .create_relationship(make(RelationshipType::DependsOn, &a, &a))
            .await,
        ErrorKind::InvalidInput,
        "an acyclic type pointing at itself",
    );
    store
        .create_relationship(make(RelationshipType::References, &a, &a))
        .await
        .expect("`references` may point at itself");

    store.depends_on(&a, &b).await;
    expect_err(
        store
            .create_relationship(make(RelationshipType::DependsOn, &a, &b))
            .await,
        ErrorKind::Conflict,
        "exact duplicate (source, type, target)",
    );
    store
        .create_relationship(make(RelationshipType::References, &a, &b))
        .await
        .expect("same endpoints, different type");
    store.depends_on(&b, &a).await;
    assert_eq!(
        store.list_relationships().await.unwrap().len(),
        4,
        "reverse direction is a different relationship; cycles are allowed at insert time"
    );
}

/// Relationships are listed by id (creation order), carry the injected time and can be fetched.
pub async fn relationship_get_and_list_order<S: StoreUnderTest>(store: &Ctx<'_, S>, env: &Env) {
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    let b = store.create_artifact(new_artifact("beta")).await.unwrap();
    let c = store.create_artifact(new_artifact("gamma")).await.unwrap();
    env.advance(30);
    let first = store.depends_on(&a, &b).await;
    let second = store.depends_on(&b, &c).await;
    let third = store.depends_on(&a, &c).await;

    assert_eq!(first.created(), env.now());
    assert!(first.id() < second.id() && second.id() < third.id());
    assert_eq!(
        store.list_relationships().await.unwrap(),
        [first.clone(), second, third]
    );
    assert_eq!(
        store.get_relationship(first.id()).await.unwrap(),
        Some(first)
    );
    let unknown = RelationshipId::from_bits(4242);
    assert_eq!(store.get_relationship(&unknown).await.unwrap(), None);
}

/// The strict snapshot agrees with the individual operations and is sorted by id.
pub async fn snapshot_reflects_state<S: StoreUnderTest>(store: &Ctx<'_, S>, _env: &Env) {
    let b = store.create_artifact(new_artifact("beta")).await.unwrap();
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    let rel = store.depends_on(&a, &b).await;

    let snapshot = store.snapshot().await.unwrap();
    assert_eq!(snapshot.artifacts(), [b.clone(), a.clone()]);
    assert_eq!(snapshot.relationships(), [rel]);
    assert_eq!(snapshot.resolve(&by_slug("alpha")), Some(&a));
    assert_eq!(snapshot.resolve(&ArtifactRef::Id(*b.id())), Some(&b));
    assert_eq!(snapshot.outgoing(a.id()).count(), 1);
    assert_eq!(snapshot.incoming(b.id()).count(), 1);
    assert_eq!(snapshot.info(), &store.info().await.unwrap());
}

/// A lenient load of healthy data reports no issues and the same entities.
pub async fn load_of_a_clean_repository_has_no_issues<S: StoreUnderTest>(
    store: &Ctx<'_, S>,
    _env: &Env,
) {
    let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
    let b = store.create_artifact(new_artifact("beta")).await.unwrap();
    store.depends_on(&a, &b).await;

    let load = store.load().await.unwrap();
    assert!(load.info.is_ok());
    assert!(!load.has_fatal_issues());
    assert_eq!(load.issues, [], "a clean repository has no issues");
    assert_eq!(load.artifacts.len(), 2);
    assert_eq!(load.relationships.len(), 1);
}

/// Repository information does not change by reading or by writing entities.
pub async fn info_is_stable<S: StoreUnderTest>(store: &Ctx<'_, S>, _env: &Env) {
    let first = store.info().await.unwrap();
    store.create_artifact(new_artifact("alpha")).await.unwrap();
    let second = store.info().await.unwrap();
    assert_eq!(first, second);
}

/// Identical operation sequences, clocks and id sources give identical state.
pub async fn determinism<S, F>(factory: &F)
where
    S: StoreUnderTest,
    F: Fn(&Env) -> Fresh<S>,
{
    async fn run<S: StoreUnderTest>(store: &Ctx<'_, S>, env: &Env) -> Vec<Artifact> {
        let a = store.create_artifact(new_artifact("alpha")).await.unwrap();
        env.advance(10);
        let b = store.create_artifact(new_artifact("beta")).await.unwrap();
        store.depends_on(&a, &b).await;
        env.advance(10);
        store
            .update_artifact(a.id(), retitle("A"), None)
            .await
            .unwrap();
        store.list_artifacts().await.unwrap()
    }

    let (env_one, env_two) = (Env::new(), Env::new());
    let (one, two) = (factory(&env_one), factory(&env_two));
    let (ctx_one, ctx_two) = (Ctx(&one.store), Ctx(&two.store));
    let first = run(&ctx_one, &env_one).await;
    let second = run(&ctx_two, &env_two).await;
    assert_eq!(first, second, "artifacts differ between identical runs");
    assert_eq!(
        ctx_one.list_relationships().await.unwrap(),
        ctx_two.list_relationships().await.unwrap(),
        "relationships differ between identical runs"
    );
}
