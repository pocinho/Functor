# Traits and Interfaces

Status: Draft
Related RFCs: RFC-0002 (primary), RFC-0006, RFC-0007, RFC-0008, RFC-0015, RFC-0017
Owner crate: `functor-core`
Related ADRs: ADR-0002, ADR-0005

## Purpose

Defines the Rust traits that form the Functor core API for the CLI MVP. Every projection, including the CLI, reaches the platform only through these traits (RFC-0002). Domain types are in [domain-model.md](domain-model.md); error types in [error-handling.md](error-handling.md).

## Design Rules

- Traits live in `functor-core`, which depends only on `functor-model`.
- I/O-facing traits are `async` using native `async fn` in traits. Services are composed with **generics**, not `dyn` (ADR-0002).
- Traits are small and focused; services that need several capabilities take several type parameters or a single composed struct.
- All fallible operations return `Result<T, FunctorError>`, aliased as `functor_core::Result<T>`.
- "Not found" is expressed as `Ok(None)` by `get`-style lookups and as `Err(FunctorError::NotFound)` by operations that require existence (`update`, relationship creation with a missing endpoint).
- Mutating operations take `&self`. Implementations use interior synchronization and the repository lock (see [storage-design.md](storage-design.md)).
- Methods never print and never read from the environment. Time and ids are injected.

## Cross-Cutting Services

```rust
pub trait Clock { fn now(&self) -> Timestamp; }
pub trait IdGenerator {
    fn artifact_id(&self) -> ArtifactId;
    fn relationship_id(&self) -> RelationshipId;
    fn repository_id(&self) -> RepositoryId;
    fn event_id(&self) -> EventId;
}
```

Production implementations: `SystemClock`, `UlidGenerator` (in `functor-core`). Test implementations: `FixedClock`, `SequentialIds` (in `functor-testkit`).

## Repository Lifecycle

```rust
pub trait RepositoryManager {
    type Repo;                                           // the opened repository handle
    async fn init(&self, at: &Path, options: InitOptions) -> Result<Self::Repo>;
    async fn open(&self, root: &Path) -> Result<Self::Repo>;
    async fn discover(&self, start: &Path) -> Result<Self::Repo>;   // walks up to find `.functor/`
}

pub struct InitOptions { pub name: Option<String>, pub description: Option<String> }
```

- `init` creates `.functor/repository.toml` and the directories listed in [repository-layout.md](repository-layout.md). It fails with `Conflict` if the target already contains `.functor/`. See [repository-lifecycle.md](repository-lifecycle.md).
- `discover` returns `NotARepository` if no ancestor contains `.functor/`.
- In the MVP the implementation lives in `functor-store`. A separate `functor-repository` crate is deferred.

## Reading Repository State

```rust
pub trait RepositoryReader {
    async fn info(&self) -> Result<RepositoryInfo>;
    /// Strict load. Fails on any malformed or inconsistent entity.
    async fn snapshot(&self) -> Result<RepositorySnapshot>;
    /// Lenient load used by validation. Never fails because of bad entity data.
    async fn load(&self) -> Result<SnapshotLoad>;
}

pub struct SnapshotLoad {
    pub info: Result<RepositoryInfo, LoadIssue>,
    pub artifacts: Vec<Artifact>,
    pub relationships: Vec<Relationship>,
    pub issues: Vec<LoadIssue>,      // malformed files, duplicate ids, filename/id mismatch, ...
}

pub struct LoadIssue { pub kind: LoadIssueKind, pub path: RepoPath, pub detail: String }
```

`load` exists so `validate` can report problems with a damaged repository instead of failing at the first one (RFC-0007 "Unknown" and "Failed" outcomes). Everything else uses `snapshot`, which turns any `LoadIssue` into an error pointing the user to `functor validate`.

## Artifacts

```rust
pub trait ArtifactRepository {
    async fn create(&self, new: NewArtifact) -> Result<Artifact>;
    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>>;
    async fn resolve(&self, reference: &ArtifactRef) -> Result<Option<Artifact>>;
    async fn list(&self) -> Result<Vec<Artifact>>;
    async fn update(
        &self,
        id: &ArtifactId,
        patch: ArtifactPatch,
        expected: Option<Revision>,        // optimistic concurrency
    ) -> Result<Artifact>;
}
```

- `create` generates the id, checks slug uniqueness (`Conflict` on duplicate) and sets `revision = 1` and `created = updated = now`.
- `update` applies the patch, increments the revision and updates `updated`. If `expected` is `Some` and differs from the stored revision, it returns `Conflict` without writing.
- No `delete` in the MVP. Deleting would orphan relationships and requires a deletion policy (RFC-0016 territory).

## Relationships

```rust
pub trait RelationshipRepository {
    async fn create(&self, new: NewRelationship) -> Result<Relationship>;
    async fn get(&self, id: &RelationshipId) -> Result<Option<Relationship>>;
    async fn list(&self) -> Result<Vec<Relationship>>;
}
```

- `create` verifies both endpoints exist (`NotFound`), rejects self-reference for acyclic types, rejects an exact duplicate `(source, type, target)` (`Conflict`), and does **not** reject cycles across the graph; validation reports those (`V0011`). Rejecting cycles at insert time was considered and rejected because merged or hand-edited repositories can contain them anyway and must stay loadable.
- No update or remove in the MVP.

## Queries

```rust
pub trait QueryEngine {
    async fn lookup(&self, reference: &ArtifactRef) -> Result<Option<Artifact>>;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<Artifact>>;
    async fn traverse(&self, traversal: &Traversal) -> Result<TraversalResult>;
    async fn execute(&self, query: Query) -> Result<QueryResult>;   // dispatches to the three above
}
```

Aligned with the RFC-0006 operations (Lookup, Search, Traverse). Filter and Project are expressed through `SearchQuery` fields and output selection in the projection. See [query-model-mvp.md](query-model-mvp.md). Implementations build a `RepositorySnapshot` once per call, then run pure synchronous functions over it.

## Validation

```rust
pub enum ValidationTarget { Repository, Artifact(ArtifactRef) }

pub trait ValidationEngine {
    async fn validate(&self, target: ValidationTarget) -> Result<ValidationReport>;
}
```

- A completed validation is reported to the caller; **persisting** the report and publishing `validation.completed` is done by the caller orchestration (the CLI) so that the engine stays free of side effects.
- Rules are plain synchronous objects: `trait Rule { fn code(&self) -> RuleCode; fn check(&self, load: &SnapshotLoad) -> Vec<Finding>; }`. They live in `functor-validation`. See [validation-rules.md](validation-rules.md).

## Events

```rust
pub trait EventPublisher {
    async fn publish(&self, event: NewEvent) -> Result<Event>;   // assigns id, seq and timestamp
}

pub trait EventSource {
    type Subscription: EventSubscription;
    async fn subscribe(&self, from: EventPosition) -> Result<Self::Subscription>;
}

pub trait EventSubscription {
    /// Returns the next event, waiting if none is available. `Ok(None)` means the source closed.
    async fn next(&mut self) -> Result<Option<Event>>;
}

pub enum EventPosition { Start, End, After(u64) }   // sequence number
```

`functor-events` implements these over the event log, using `tokio` for waiting (ADR-0002). `functor-core` never names a runtime. The MVP has no push-style subscriber callbacks.

## Not in the MVP

`ExecutionEngine`, `CapabilityProvider`, `Extension`, artifact and relationship deletion, relationship update, sync and federation traits. They come with later RFC work and ADRs. The earlier draft of this document listed them; they are intentionally removed so the MVP surface stays small.

## Implementations

| Trait | MVP implementation | Crate |
|-------|--------------------|-------|
| `RepositoryManager`, `RepositoryReader`, `ArtifactRepository`, `RelationshipRepository` | `FileStore` | `functor-store` |
| same | `MemoryStore` (tests and examples) | `functor-store` |
| `QueryEngine` | `SnapshotQueryEngine<R: RepositoryReader>` | `functor-query` |
| `ValidationEngine` | `RuleValidationEngine<R: RepositoryReader>` | `functor-validation` |
| `EventPublisher`, `EventSource` | `FileEventLog` | `functor-events` |
| `Clock`, `IdGenerator` | `SystemClock`, `UlidGenerator` in `functor-core`; `FixedClock`, `SequentialIds` in `functor-testkit` | `functor-core`, `functor-testkit` |

`MemoryStore` and `FileStore` must pass the same conformance test suite (see [testing-strategy.md](testing-strategy.md)); that suite is what makes the storage-independence promise of RFC-0008 enforceable.

## Stability

Traits are conservative: adding a method requires a default implementation or a major version of the crate. Parameter and result types are `#[non_exhaustive]` where they may grow. See [api-design-guidelines.md](api-design-guidelines.md).
