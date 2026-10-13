# Crate Dependencies

Status: Draft
Related RFCs: RFC-0002, RFC-0011
Related ADRs: ADR-0005, ADR-0009, ADR-0010
Related docs: [workspace-layout.md](workspace-layout.md), [dependency-policy.md](dependency-policy.md)

## Purpose

Defines which crates may depend on which. The goals are strict architectural boundaries, API-first design, minimal coupling and a semantic model isolated from implementation details.

## Layers

Dependencies flow **downward** only.

```text
Projections        functor-cli  (composition root)
      ↓
Services           functor-store  functor-query  functor-validation  functor-events
      ↓            (store and events also use the foundation service below)
Foundation service functor-repository
      ↓
Core API           functor-core
      ↓
Semantic Model     functor-model
```

`functor-testkit` supports all layers and is used only as a `dev-dependency`. It depends on `functor-core` and `functor-model`.

Services are **siblings**: none depends on another, with one exception. `functor-repository` is a **foundation service** ([ADR-0009](adr/ADR-0009-functor-repository-crate.md)): it owns the repository layout, lifecycle and write lock, and `functor-store` and `functor-events` may depend on it. No other service-to-service edge is allowed. `functor-query` and `functor-validation` depend on the `RepositoryReader` *trait* in `functor-core`, not on `functor-store`. The earlier linear chain (store, then query, then validation, then CLI) was incorrect and is removed.

## Allowed Dependencies (MVP)

| Crate | May depend on (workspace) | Notable external crates |
|-------|---------------------------|-------------------------|
| `functor-model` | none | `ulid`, `thiserror` |
| `functor-core` | `functor-model` | `thiserror`, `async-trait` |
| `functor-repository` | `functor-core`, `functor-model` | `toml_edit`, `serde`, `tokio` (fs, sync), `fd-lock`, `async-trait` |
| `functor-store` | `functor-core`, `functor-model`, `functor-repository` | `toml_edit`, `serde`, `tokio` (fs, sync), `async-trait` |
| `functor-query` | `functor-core`, `functor-model` | `async-trait` |
| `functor-validation` | `functor-core`, `functor-model` | `async-trait` |
| `functor-events` | `functor-core`, `functor-model`, `functor-repository` | `serde`, `serde_json`, `tokio`, `notify`, `async-trait` |
| `functor-testkit` | `functor-core`, `functor-model` | `tempfile`, `proptest` |
| `functor-cli` | all of the above | `clap`, `anyhow`, `tracing`, `tracing-subscriber`, `serde`, `serde_json`, `tokio` |

Dev-dependencies (any crate): `functor-testkit`, `proptest`, `insta`, `tempfile`; CLI tests also `assert_cmd` and `predicates`. The approved list and rules for adding crates are in [dependency-policy.md](dependency-policy.md).

## Model Purity and Serialization

`functor-model` must not depend on:

```text
storage, runtime (tokio), CLI, networking, serde or any other serialization framework
```

Serialization belongs to the crate that owns the representation (ADR-0005): TOML DTOs in `functor-store` (entities) and `functor-repository` (`repository.toml`), event JSON in `functor-events`, CLI output JSON in `functor-cli`. This resolves the earlier contradiction between "the model must be serialization free" and "artifacts are TOML".

## Composition Root

`functor-cli` is the **only** place where concrete implementations are chosen and wired:

```text
FileRepositoryManager (functor-repository) -> RepositoryManager
FileStore                                  -> RepositoryReader, ArtifactRepository, RelationshipRepository
FileEventLog                               -> EventPublisher, EventSource
SnapshotQueryEngine<FileStore>
RuleValidationEngine<FileStore>
SystemClock, UlidGenerator
```

Core traits are dyn-compatible ([ADR-0010](adr/ADR-0010-async-trait-and-dyn.md)), so services receive their collaborators either as generic parameters (recommended inside services) or as `Arc<dyn Trait>` (allowed, notably at the composition root). No service constructs another service. Command handlers depend on the traits only, so tests run the same handlers against `MemoryStore`.

## Forbidden Dependencies

```text
functor-model        -> any other functor crate
functor-core         -> repository, store, query, validation, events, cli
functor-repository   -> store, query, validation, events, cli
store | query | validation | events -> each other
   (exception: store -> repository and events -> repository)
store | query | validation | events | repository -> cli
any library crate    -> functor-cli
functor-testkit      -> as a normal (non-dev) dependency of anything
any projection       -> another projection
```

A library crate must also not depend on `anyhow`, `clap`, `tracing-subscriber` or terminal crates; those belong to the CLI.

## Enforcement

- `cargo deny check bans` configured with the table above (`deny.toml` `[bans]` wrappers and skip lists) to block disallowed external crates per layer where `cargo-deny` can express it.
- A CI check (`cargo metadata` based, a small script or an `xtask`) that asserts the workspace dependency edges equal the allowed set. Any new edge fails CI until this document and the check are updated together.
- Code review checklist item in [development-workflow.md](development-workflow.md).

## Architectural Principle

The semantic model is the center of gravity. Everything else is replaceable:

- Remove a projection (CLI): the platform remains intact.
- Replace the store (file, memory, later git or database): the platform remains intact; the conformance suite proves behavioral equivalence.
- Change a protocol or output format: the model and core API do not change.

## Future Crates

When the deferred crates in [workspace-layout.md](workspace-layout.md) arrive, they follow the same rules: new projections (`functor-mcp`, `functor-web`) sit at the CLI's layer and depend on services through `functor-core`; `functor-runtime` and `functor-sync` are services; none may be depended on by `functor-model` or `functor-core`. `functor-repository` remains the only foundation service unless a new ADR creates another.
