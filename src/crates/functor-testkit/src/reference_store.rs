//! A minimal reference store used to test the conformance suite itself.
//!
//! The suite must pass for a correct store and fail for each deliberately broken variant;
//! otherwise it would be vacuous. `MemoryStore` in `functor-store` (M3) is the real implementation
//! and runs the same suite.

use std::collections::BTreeMap;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

use async_trait::async_trait;
use functor_core::{
    ArtifactRepository, Clock, FunctorError, IdGenerator, RelationshipRepository, RepositoryReader,
    Result, SnapshotLoad,
};
use functor_model::{
    Artifact, ArtifactId, ArtifactPatch, ArtifactRef, EntityKind, EvidenceKind, FormatVersion,
    NewArtifact, NewRelationship, Patched, Relationship, RelationshipId, RepositoryInfo,
    RepositoryName, Revision,
};

use crate::conformance::{Env, Fresh, run_store_conformance};

/// Deliberate defects, one per behavior the suite must protect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bug {
    AllowDuplicateSlugs,
    IgnoreExpectedRevision,
    SkipEndpointCheck,
    AllowDuplicateRelationships,
    ListInReverse,
    BumpRevisionOnNoOp,
    AllowEvidenceUpdates,
}

#[derive(Default)]
struct State {
    artifacts: BTreeMap<ArtifactId, Artifact>,
    relationships: BTreeMap<RelationshipId, Relationship>,
}

struct ReferenceStore {
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    info: RepositoryInfo,
    state: Mutex<State>,
    bug: Option<Bug>,
}

impl ReferenceStore {
    fn new(env: &Env, bug: Option<Bug>) -> Self {
        let ids = env.ids();
        let info = RepositoryInfo::new(
            ids.repository_id(),
            RepositoryName::new("Reference").unwrap(),
            None,
            FormatVersion::CURRENT,
            env.now(),
        );
        Self {
            clock: env.clock(),
            ids,
            info,
            state: Mutex::default(),
            bug,
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn has(&self, bug: Bug) -> bool {
        self.bug == Some(bug)
    }
}

#[async_trait]
impl RepositoryReader for ReferenceStore {
    async fn info(&self) -> Result<RepositoryInfo> {
        Ok(self.info.clone())
    }

    async fn load(&self) -> Result<SnapshotLoad> {
        let state = self.state();
        Ok(SnapshotLoad {
            info: Ok(self.info.clone()),
            artifacts: state.artifacts.values().cloned().collect(),
            relationships: state.relationships.values().cloned().collect(),
            issues: Vec::new(),
        })
    }
}

#[async_trait]
impl ArtifactRepository for ReferenceStore {
    async fn create(&self, new: NewArtifact) -> Result<Artifact> {
        let mut state = self.state();
        let taken = state.artifacts.values().any(|a| a.slug() == &new.slug);
        if taken && !self.has(Bug::AllowDuplicateSlugs) {
            return Err(FunctorError::conflict(
                EntityKind::Artifact,
                format!("slug `{}` is taken", new.slug),
            ));
        }
        let artifact = Artifact::create(self.ids.artifact_id(), new, self.clock.now());
        state.artifacts.insert(*artifact.id(), artifact.clone());
        Ok(artifact)
    }

    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>> {
        Ok(self.state().artifacts.get(id).cloned())
    }

    async fn resolve(&self, reference: &ArtifactRef) -> Result<Option<Artifact>> {
        let state = self.state();
        Ok(match reference {
            ArtifactRef::Id(id) => state.artifacts.get(id).cloned(),
            ArtifactRef::Slug(slug) => state.artifacts.values().find(|a| a.slug() == slug).cloned(),
        })
    }

    async fn list(&self) -> Result<Vec<Artifact>> {
        let mut all: Vec<Artifact> = self.state().artifacts.values().cloned().collect();
        if self.has(Bug::ListInReverse) {
            all.reverse();
        }
        Ok(all)
    }

    async fn update(
        &self,
        id: &ArtifactId,
        patch: ArtifactPatch,
        expected: Option<Revision>,
    ) -> Result<Patched> {
        let mut state = self.state();
        let current = state
            .artifacts
            .get(id)
            .cloned()
            .ok_or_else(|| FunctorError::not_found(EntityKind::Artifact, id.to_string()))?;
        if EvidenceKind::of(&current).is_some() && !self.has(Bug::AllowEvidenceUpdates) {
            return Err(FunctorError::conflict(
                EntityKind::Artifact,
                "evidence is immutable",
            ));
        }
        if let Some(expected) = expected
            && expected != current.revision()
            && !self.has(Bug::IgnoreExpectedRevision)
        {
            return Err(FunctorError::conflict(
                EntityKind::Artifact,
                format!("expected revision {expected}, found {}", current.revision()),
            ));
        }
        if let Some(slug) = &patch.slug
            && state
                .artifacts
                .values()
                .any(|a| a.id() != id && a.slug() == slug)
            && !self.has(Bug::AllowDuplicateSlugs)
        {
            return Err(FunctorError::conflict(
                EntityKind::Artifact,
                format!("slug `{slug}` is taken"),
            ));
        }
        let mut patched = current.apply_patch(patch, self.clock.now())?;
        if patched.changed.is_empty() && self.has(Bug::BumpRevisionOnNoOp) {
            patched = current.apply_patch(
                ArtifactPatch::new().with_description(Some("bumped".into())),
                self.clock.now(),
            )?;
        }
        if !patched.changed.is_empty() {
            state.artifacts.insert(*id, patched.artifact.clone());
        }
        Ok(patched)
    }
}

#[async_trait]
impl RelationshipRepository for ReferenceStore {
    async fn create(&self, new: NewRelationship) -> Result<Relationship> {
        let mut state = self.state();
        if !self.has(Bug::SkipEndpointCheck) {
            for endpoint in [new.source, new.target] {
                if !state.artifacts.contains_key(&endpoint) {
                    return Err(FunctorError::not_found(
                        EntityKind::Artifact,
                        endpoint.to_string(),
                    ));
                }
            }
        }
        let relationship = Relationship::create(self.ids.relationship_id(), new, self.clock.now())?;
        let duplicate = state.relationships.values().any(|r| {
            r.source() == relationship.source()
                && r.target() == relationship.target()
                && r.relationship_type() == relationship.relationship_type()
        });
        if duplicate && !self.has(Bug::AllowDuplicateRelationships) {
            return Err(FunctorError::conflict(
                EntityKind::Relationship,
                "duplicate (source, type, target)",
            ));
        }
        state
            .relationships
            .insert(*relationship.id(), relationship.clone());
        Ok(relationship)
    }

    async fn get(&self, id: &RelationshipId) -> Result<Option<Relationship>> {
        Ok(self.state().relationships.get(id).cloned())
    }

    async fn list(&self) -> Result<Vec<Relationship>> {
        Ok(self.state().relationships.values().cloned().collect())
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
}

fn suite(bug: Option<Bug>) {
    block_on(run_store_conformance(|env: &Env| {
        Fresh::new(ReferenceStore::new(env, bug))
    }));
}

#[test]
fn a_correct_store_passes_the_suite() {
    suite(None);
}

#[test]
fn the_suite_rejects_every_deliberately_broken_store() {
    for bug in [
        Bug::AllowDuplicateSlugs,
        Bug::IgnoreExpectedRevision,
        Bug::SkipEndpointCheck,
        Bug::AllowDuplicateRelationships,
        Bug::ListInReverse,
        Bug::BumpRevisionOnNoOp,
        Bug::AllowEvidenceUpdates,
    ] {
        let outcome = catch_unwind(AssertUnwindSafe(|| suite(Some(bug))));
        assert!(outcome.is_err(), "the suite did not detect {bug:?}");
    }
}

#[test]
fn the_reference_info_comes_from_the_injected_clock() {
    let env = Env::new();
    let store = ReferenceStore::new(&env, None);
    assert_eq!(store.info.created(), env.now());
}
