# Traits and Interfaces

Status: Accepted
Related RFCs: RFC-0002 (primary), RFC-0006, RFC-0007, RFC-0008, RFC-0015, RFC-0017
Owner crate: `functor-core`
Related ADRs: ADR-0005, ADR-0009, ADR-0010

## Purpose

Defines the Rust traits that form the Functor core API for the CLI MVP. Every projection, including the CLI, reaches the platform only through these traits (RFC-0002). Domain types are in [domain-model.md](../design/domain-model.md); error types in [error-handling.md](error-handling.md).

## Design Rules

- Traits live in `functor-core`, which depends only on `functor-model` (and `async-trait`).
- I/O-facing traits are `async` and annotated with `#[async_trait]` ([ADR-0010](../adr/ADR-0010-async-trait-and-dyn.md)); the attribute is omitted from the listings below for brevity. They are **dyn-compatible**: `Box<dyn Trait>` and `Arc<dyn Trait>` are allowed. Static generics remain the recommended default inside services.
- To stay object-safe, traits have **no associated types, no generic methods and no `impl Trait` returns**. Implementations are `Send + Sync`, and returned futures are `Send`.
- A compile-time test asserts every core trait can be used as `dyn Trait`.
- Traits are small and focused; services that need several capabilities take several type parameters or a single composed struct.
- All fallible operations return `Result<T, FunctorError>`, aliased as `functor_core::Result<T>`.
- "Not found" is expressed as `Ok(None)` by `get`-style lookups and as `Err(FunctorError::NotFound)` by operations that require existence (`update`, relationship creation with a missing endpoint).
- Mutating operations take `&self`. Implementations use interior synchronization and the repository lock (see [storage-design.md](../design/storage-design.md)).
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
    async fn init(&self, at: &Path, options: InitOptions) -> Result<RepositoryHandle>;
    async fn open(&self, root: &Path) -> Result<RepositoryHandle>;
    async fn discover(&self, start: &Path) -> Result<RepositoryHandle>;   // walks up to find `.functor/`
}

pub struct InitOptions { pub name: Option<String>, pub description: Option<String>, pub nested: bool }

/// Concrete, storage-neutral handle: where the repository is and what it is.
pub struct RepositoryHandle { pub root: PathBuf, pub info: RepositoryInfo }
```

- `init` creates `.functor/repository.toml` and the directories listed in [repository-layout.md](../design/repository-layout.md). It fails with `Conflict` if the target already contains `.functor/` (or an ancestor does, unless `nested`). See [repository-lifecycle.md](../design/repository-lifecycle.md).
- `discover` returns `NotARepository` if no ancestor contains `.functor/`.
- The implementation, `FileRepositoryManager`, lives in `functor-repository` ([ADR-0009](../adr/ADR-0009-functor-repository-crate.md)), together with the write lock. `MemoryStore` does not implement this trait.

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

`LoadIssueKind` (non-exhaustive) lists the problem kinds and their validation rules in [storage-design.md](../design/storage-design.md). Two kinds are non-fatal for a strict load (`UnexpectedFile`, `UnknownField`).

`load` exists so `validate` can report problems with a damaged repository instead of failing at the first one (RFC-0007 "Unknown" and "Failed" outcomes). Everything else uses `snapshot`, which turns any fatal `LoadIssue` into an `Inconsistent` error pointing the user to `functor validate`. `snapshot` has a **default implementation** (`load().into_snapshot()`, with `SnapshotLoad::into_snapshot` in `functor-core`) so stores only have to implement `info` and `load`; they may override it for efficiency but must keep the same semantics.

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
    ) -> Result<Patched>;
}
```

- `create` generates the id, checks slug uniqueness (`Conflict` on duplicate) and sets `revision = 1` and `created = updated = now`.
- `update` applies the patch and returns `Patched { artifact, changed }` (see [domain-model.md](../design/domain-model.md)). A real change increments the revision and updates `updated`. A patch that changes nothing writes nothing and returns an empty `changed` list, so callers can skip the event and say `no changes`. If `expected` is `Some` and differs from the stored revision, it returns `Conflict` without writing. A slug already used by another artifact is `Conflict`; an unknown id is `NotFound`.
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

Aligned with the RFC-0006 operations (Lookup, Search, Traverse). `execute` has a default implementation that dispatches to the other three. Filter and Project are expressed through `SearchQuery` fields and output selection in the projection. See [query-model-mvp.md](../design/query-model-mvp.md). Implementations build a `RepositorySnapshot` once per call, then run pure synchronous functions over it.

## Validation

```rust
pub enum ValidationTarget { Repository, Artifact(ArtifactRef) }

pub trait ValidationEngine {
    async fn validate(&self, target: ValidationTarget) -> Result<ValidationReport>;
}
```

- A completed validation is reported to the caller; **persisting** the report and publishing `validation.completed` is done by the caller orchestration (the CLI) so that the engine stays free of side effects.
- Rules are plain synchronous objects: `trait Rule { fn code(&self) -> RuleCode; fn check(&self, load: &SnapshotLoad) -> Vec<Finding>; }`. They live in `functor-validation`. See [validation-rules.md](../design/validation-rules.md).

## Events

```rust
pub trait EventPublisher {
    async fn publish(&self, event: NewEvent) -> Result<Event>;   // assigns id, seq and timestamp
}

pub trait EventSource {
    async fn subscribe(&self, from: EventPosition) -> Result<Box<dyn EventSubscription>>;
}

pub trait EventSubscription: Send {
    /// Returns the next event, waiting if none is available. `Ok(None)` means the source closed.
    async fn next(&mut self) -> Result<Option<Event>>;
}

pub enum EventPosition { Start, End, After(u64) }   // sequence number
```

`functor-events` implements these over the event log, using `tokio` for waiting. `functor-core` never names a runtime. The MVP has no push-style subscriber callbacks. The subscription is a boxed trait object so that `EventSource` has no associated type and stays object-safe ([ADR-0010](../adr/ADR-0010-async-trait-and-dyn.md)).

## Not in the MVP

`CapabilityProvider`, `Extension`, artifact and relationship deletion, relationship update, sync and federation traits. They come with later RFC work and ADRs. The earlier draft of this document listed them; they are intentionally removed so the MVP surface stays small. Execution arrives in Phase 1 (next section).

## Execution and Evidence (Phase 1)

Contracts from [ADR-0012](../adr/ADR-0012-execution-provider-and-safety.md) to [ADR-0015](../adr/ADR-0015-validation-report-persistence-and-events.md); values in `functor-model`, traits in `functor-core`, all dyn-compatible.

```rust
pub trait ExecutionProvider: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    async fn execute(&self, plan: ExecutionPlan, cancel: CancelToken) -> Result<ProviderRun>;
}

pub trait EvidenceRecorder: Send + Sync {
    async fn capture(&self, source: &ArtifactRef, role: CaptureRole) -> Result<CapturedBytes>;
    async fn record_intent(&self, intent: RunIntent) -> Result<IntentRecord>;     // idempotent by request id
    async fn acquire_lease(&self, run: &ArtifactId) -> Result<Box<dyn RunLease>>;  // dropped = released
    async fn record_result(&self, run: &ArtifactId, finish: RunFinish) -> Result<RunRecord>; // commit point
    async fn run(&self, run: &ArtifactRef) -> Result<Option<RunRecord>>;
    async fn runs(&self) -> Result<Vec<RunRecord>>;
    async fn status(&self, run: &ArtifactId) -> Result<RunStatus>;
    async fn streams(&self, run: &ArtifactId) -> Result<Option<RunStreams>>;
    async fn read_capture(&self, capture: &ArtifactId) -> Result<Vec<u8>>;
    async fn recover(&self) -> Result<Vec<RunRecord>>;
}

pub trait ReportRecorder: Send + Sync {
    async fn save_report(&self, report: &ValidationReport, subject: ReportSubject, strict: bool) -> Result<ArtifactId>;
    async fn load_report(&self, id: &ArtifactId) -> Result<Option<(ValidationReport, ReportSubject)>>;
}
```

- **Provider.** Receives a prepared `ExecutionPlan` (captured workflow and inputs, effective limits, an `ExecutionGrant`) and returns a `ProviderRun`. It never opens the repository, persists, validates or publishes events. A child failure, timeout, cancellation or missing program is an outcome (`ProviderRun::outcome`), never an `Err`; `Err` is a refusal (`Unsupported`, `PermissionDenied`, `InvalidInput`, all produced by `ExecutionPlan::check` before any side effect) or an infrastructure fault. `ProviderRun::check` verifies a report honours the contract before the orchestrator publishes anything.
- **Cancellation.** `CancelToken` is a shared flag (`std` only; core names no runtime). Providers poll it and must stop the whole process tree.
- **Recorder.** One short critical section per method; the write lock is never held across a child. A run is complete if and only if its `run_result` exists; `record_result` refuses a second result (`Conflict`) and an inconsistent `RunFinish` (`InvalidInput`, nothing written). `RunStatus` is derived (`Running` only while a lease is held). `recover` completes orphans as `interrupted` and never reruns, publishes or deletes.
- **Reports.** `ReportRecorder` is separate from `ValidationEngine`, which stays free of side effects.
- **Outcome versus error.** A run that finishes badly is `Ok(RunRecord)` with its outcome; infrastructure and policy failures are `FunctorError`.
- The orchestration that wires capture, intent, execution, publication, relationships, validation and events is P1.4; there is no `ExecutionEngine` trait.

## Implementations

| Trait | MVP implementation | Crate |
|-------|--------------------|-------|
| `RepositoryManager` | `FileRepositoryManager` | `functor-repository` |
| `RepositoryReader`, `ArtifactRepository`, `RelationshipRepository` | `FileStore` | `functor-store` |
| same | `MemoryStore` (tests and examples) | `functor-store` |
| `QueryEngine` | `SnapshotQueryEngine<'a, R>` (borrows any `R: RepositoryReader + ?Sized`; one fresh snapshot per call) | `functor-query` |
| `ValidationEngine` | `RuleValidationEngine<'a, R>` (borrows any `R: RepositoryReader + ?Sized`, takes the injected `Clock`; one lenient load per call) | `functor-validation` |
| `EventPublisher`, `EventSource` | `FileEventLog` | `functor-events` |
| `Clock`, `IdGenerator` | `SystemClock`, `UlidGenerator` in `functor-core`; `FixedClock`, `SequentialIds` in `functor-testkit` | `functor-core`, `functor-testkit` |
| `ExecutionProvider` | `LocalProcessProvider` (`local-process`, P1.3); `FakeProvider` (testkit) | `functor-runtime`, `functor-testkit` |
| `EvidenceRecorder`, `ReportRecorder` | `FileEvidence`, `InMemoryEvidence` (both `EvidenceStore`, P1.2); `MemoryEvidence` is the simpler testkit fake | `functor-store`, `functor-testkit` |

`MemoryStore` and `FileStore` must pass the same conformance test suite (see [testing-strategy.md](../quality/testing-strategy.md)); that suite is what makes the storage-independence promise of RFC-0008 enforceable. `RepositoryManager` is tested in `functor-repository`.

## Stability

Traits are conservative: adding a method requires a default implementation or a major version of the crate. Parameter and result types are `#[non_exhaustive]` where they may grow. See [api-design-guidelines.md](api-design-guidelines.md).
