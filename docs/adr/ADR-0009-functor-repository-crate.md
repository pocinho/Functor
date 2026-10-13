# ADR-0009: functor-repository is part of the MVP

## Status

Accepted

## Context

`workspace-layout.md` deferred a `functor-repository` crate and put repository lifecycle (`init`, `open`, `discover`) in `functor-store`. The project owner decided to extract it now so that `functor-store` is purely entity persistence.

## Decision

- New crate `functor-repository`, depending on `functor-core` and `functor-model`. It owns:
  - the repository layout constants and path helpers ([../repository-layout.md](../design/repository-layout.md));
  - the `repository.toml` file and its DTO, and the `format_version` check;
  - `init`, `open` and `discover`, implementing `RepositoryManager`;
  - the repository write lock on `.functor/lock`.
- `functor-store` depends on `functor-repository` for layout paths, the `format_version`-checked open and the write lock. It owns only artifact and relationship persistence (`FileStore`) and `MemoryStore`.
- `functor-repository` is a **foundation service**: the only service other services may depend on. `functor-store` and `functor-events` depend on it (events needs the log path and the write lock to assign sequence numbers). All other service-to-service dependencies stay forbidden.
- The rule is enforced by the dependency-edge check ([../crate-dependencies.md](../architecture/crate-dependencies.md)).

## Consequences

- Seven library crates plus the testkit instead of six.
- `MemoryStore` implements the artifact, relationship and reader traits but not `RepositoryManager`; manager behavior is tested in `functor-repository`.
- Versioning (RFC-0016) and federation (RFC-0009) work will extend this crate rather than create it.

## RFC Impact

None. It improves the mapping of RFC-0015 (Repository Model) to a single crate.
