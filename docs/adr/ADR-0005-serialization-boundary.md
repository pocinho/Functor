# ADR-0005: Serialization lives in functor-store

## Status

Accepted

## Context

`crate-dependencies.md` said `functor-model` must not depend on serialization frameworks, yet artifacts are stored as TOML and CLI output has a JSON mode. The docs did not say where `serde` lives.

## Decision

- `functor-model` has no `serde` dependency. It holds pure domain types with validating constructors.
- `functor-store` owns **storage DTOs** (`ArtifactFile`, `RelationshipFile`, `RepositoryFile`) that derive `serde` and convert to and from model types. All TOML reading and writing happens there.
- `functor-events` owns the event wire DTO (JSON) in the same way.
- `functor-cli` owns **output DTOs** for `--output json`, defined against the schemas in [../cli-spec.md](../cli-spec.md). Model types are never serialized directly to users.

## Consequences

- Storage and output formats can change independently from the domain model.
- Some mapping code is duplicated. This is accepted because it keeps the model dependency-light and makes the on-disk and CLI contracts explicit and versionable.
- Every DTO conversion needs round-trip tests (see [../testing-strategy.md](../testing-strategy.md)).

## RFC Impact

Supports RFC-0001 and RFC-0008: semantics are independent of any representation.
