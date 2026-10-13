# Crate Dependencies

Status: Draft
Related RFCs: RFC-0002, RFC-0011
Related ADRs: ADR-0002, ADR-0005
Related docs: [workspace-layout.md](workspace-layout.md), [dependency-policy.md](dependency-policy.md)

## Purpose

Defines which crates may depend on which. The goals are strict architectural boundaries, API-first design, minimal coupling and a semantic model isolated from implementation details.

## Layers

Dependencies flow **downward** only.

```text
Projections        functor-cli  (composition root)
      ↓
Services           functor-store  functor-query  functor-validation  functor-events
      ↓
Core API           functor-core
      ↓
Semantic Model     functor-model
```

`functor-testkit` supports all layers and is used only as a `dev-dependency`. It depends on `functor-core` and `functor-model`.

Services are **siblings**: none depends on another. In particular, `functor-query` and `functor-validation` depend on the `RepositoryReader` *trait* in `functor-core`, not on `functor-store`. The earlier linear chain (store, then query, then validation, then CLI) was incorrect and is removed.

## Allowed Dependencies (MVP)

| Crate | May depend on (workspace) | Notable external crates |
|-------|---------------------------|-------------------------|
| `functor-model` | none | `ulid`, `thiserror` |
| `functor-core` | `functor-model` | `thiserror` |
| `functor-store` | `functor-core`, `functor-model` | `serde`, `toml`, `tokio` (fs, sync), `fd-lock` |
| `functor-query` | `functor-core`, `functor-model` | none |
| `functor-validation` | `functor-core`, `functor-model` | none |
| `functor-events` | `functor-core`, `functor-model` | `serde`, `serde_json`, `tokio`, `notify` |
| `functor-testkit` | `functor-core`, `functor-model` | `tempfile`, `proptest` |
| `functor-cli` | all of the above | `clap`, `anyhow`, `tracing`, `tracing-subscriber`, `serde`, `serde_json`, `tokio` |

Dev-dependencies (any crate): `functor-testkit`, `proptest`, `insta`, `tempfile`; CLI tests also `assert_cmd` and `predicates`. The approved list and rules for adding crates are in [dependency-policy.md](dependency-policy.md).

## Model Purity and Serialization

`functor-model` must not depend on:

```text
storage, runtime (tokio), CLI, networking, serde or any other serialization framework
```

Serialization belongs to the crate that owns the representation (ADR-0005): TOML DTOs in `functor-store`, event JSON in `functor-events`, CLI output JSON in `functor-cli`. This resolves the earlier contradiction between "the model must be serialization free" and "artifacts are TOML".

## Composition Root

`functor-cli` is the **only** place where concrete implementations are chosen and wired:

```text
FileStore            -> RepositoryManager, RepositoryReader, ArtifactRepository, RelationshipRepository
FileEventLog         -> EventPublisher, EventSource
SnapshotQueryEngine<FileStore>
RuleValidationEngine<FileStore>
SystemClock, UlidGenerator
```

Services receive their collaborators as generic parameters (ADR-0002). No service constructs another service. Command handlers are generic over the traits they use, so tests run the same handlers against `MemoryStore`.

## Forbidden Dependencies

```text
functor-model        -> any other functor crate
functor-core         -> store, query, validation, events, cli
store | query | validation | events -> each other
store | query | validation | events -> cli
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

When the deferred crates in [workspace-layout.md](workspace-layout.md) arrive, they follow the same rules: new projections (`functor-mcp`, `functor-web`) sit at the CLI's layer and depend on services through `functor-core`; `functor-runtime` and `functor-sync` are services; none may be depended on by `functor-model` or `functor-core`.
