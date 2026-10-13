# API Design Guidelines

Status: Draft
Related RFCs: RFC-0002, RFC-0010 (future capability exposure)
Related docs: [traits-and-interfaces.md](traits-and-interfaces.md), [error-handling.md](error-handling.md), [domain-model.md](domain-model.md)

## Purpose

Conventions for every public Rust API in the Functor workspace. They keep the API stable, projection-independent and consistent.

## Principles

1. **API first.** A capability exists in `functor-core` and its implementations before a projection exposes it (RFC-0002).
2. **Semantic consistency.** Public names follow the RFC vocabulary and [glossary.md](glossary.md): Artifact, Relationship, Repository, Query, Validation, Event.
3. **Projection independence.** No API assumes a terminal, a GUI or a protocol. No `println!`, no `std::env`, no `process::exit`, no ANSI output in library crates.
4. **Make invalid states unrepresentable.** Validating constructors, private fields (see [domain-model.md](domain-model.md)).
5. **Small and composable.** Many focused traits over one large one. Prefer composition over inheritance and supertrait towers.

## Traits and Async

- Core I/O traits are async with native `async fn` and are composed statically (ADR-0002). Do not write `Box<dyn Trait>` for core traits.
- Do not expose a runtime type (`tokio::...`) in any signature outside `functor-cli`, `functor-store` and `functor-events` internals.
- Pure logic stays synchronous. Do not make a function `async` unless it awaits I/O.
- Do not hold a lock across an `.await`. Document cancellation behavior: dropping a future before completion must leave the repository consistent (writes are atomic, see [storage-design.md](storage-design.md)).

## Naming

| Kind | Convention | Examples |
|------|------------|----------|
| Entity types | domain noun | `Artifact`, `Relationship`, `Finding` |
| Id types | `<Entity>Id` | `ArtifactId` |
| Input types | `New<Entity>`, `<Entity>Patch` | `NewArtifact`, `ArtifactPatch` |
| Query inputs | `<Thing>Query`, `Traversal` | `SearchQuery` |
| Service traits | role noun | `ArtifactRepository`, `QueryEngine`, `ValidationEngine`, `EventPublisher` |
| Methods | verbs: `create`, `get`, `resolve`, `list`, `update`, `lookup`, `search`, `traverse`, `validate`, `publish`, `subscribe` | |
| Fallible constructors | `new` for infallible, `parse` for text, `try_new` is avoided | |

- `get` returns `Option`, `require`-style helpers return `NotFound` errors.
- Use snake_case for text forms stored on disk and in JSON; use the Rust field names listed in the domain model.
- The CLI noun `relation` is a shorthand for Relationship. Rust identifiers always spell `Relationship`.

## Errors

- One public error enum, `FunctorError`, in `functor-core` (details in [error-handling.md](error-handling.md)). It is `#[non_exhaustive]`.
- Library crates use `thiserror`. `anyhow` is allowed only in `functor-cli`.
- Errors carry the data a projection needs (reference, path, expected and found values) so projections never parse error strings.
- Never panic on user data. `unwrap`/`expect` are limited to tests and to provably infallible cases with a comment.

## Data Types in Signatures

- Take borrowed forms for inputs (`&ArtifactId`, `&Path`, `&str`) and return owned values.
- Do not return `impl Iterator` borrowed from internal locks. Return owned `Vec`s with documented ordering. Streaming APIs (large repositories) are a deferred concern, so list methods may later gain paged or streaming variants without changing existing ones.
- Every collection result has a documented deterministic order (see [domain-model.md](domain-model.md)).
- Public structs that may gain fields are `#[non_exhaustive]` with constructors or builders.

## Evolution and Versioning

- Additive evolution: new methods get default implementations; new enum variants only on `#[non_exhaustive]` enums.
- Breaking changes need an ADR and a crate major version bump (pre-1.0: a minor bump with a changelog entry).
- Deprecate before removing: `#[deprecated(since, note)]` for at least one release.

## Capability Exposure

The MVP has no dynamic capability discovery (RFC-0010 is deferred). Each projection simply uses the traits it needs. Keep traits free of projection-specific concepts so a future capability registry can describe them.

## Documentation Requirements

Every public item documents:

- **Purpose**
- **Inputs** (including invariants)
- **Outputs** (including ordering)
- **Errors** (which `FunctorError` variants and when)
- **Guarantees** (atomicity, idempotence, determinism)

Crate-level docs link back to the RFC and doc that define the contract. `#![warn(missing_docs)]` is enabled for library crates through the workspace lints.

## Testing Support

- Each trait has a test double or an in-memory implementation (`MemoryStore`, `FixedClock`, `SequentialIds`).
- Each trait with more than one implementation has a shared conformance test suite.
- Doc examples compile (`cargo test --doc`).

## Review Checklist

- [ ] Does the API trace back to an RFC or an ADR?
- [ ] Is it usable without a terminal or a specific storage engine?
- [ ] Are errors typed and documented?
- [ ] Is output deterministic and ordered?
- [ ] Is the new surface the smallest that works?
