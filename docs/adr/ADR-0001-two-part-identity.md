# ADR-0001: Two-part identity: opaque id plus slug

## Status

Accepted

## Context

RFC-0005 requires identifiers to be immutable, independent of storage and **opaque**: consumers must not infer meaning from them. The MVP docs and success path use human-readable names such as `rfc-0000`, and a CLI that demands typing random ids is unusable.

## Decision

Every artifact has two identifiers:

- **Id**: opaque, generated, immutable. Format `art_<ULID>` (relationships use `rel_`, repositories `repo_`, events `evt_`). This is the identity in the RFC-0005 sense. Relationships, events and canonical references always use ids.
- **Slug**: a unique, human-friendly alias within a repository (for example `rfc-0000`). It is a lookup aid, not identity. It may be renamed. The CLI accepts either an id or a slug wherever an artifact is expected.

Details are in [../identity-and-addressing.md](../design/identity-and-addressing.md).

## Consequences

- Renaming a slug never breaks a relationship.
- Files are named by id (see ADR-0006), so renames do not move files.
- Every CLI argument that names an artifact needs a resolver (id first, then slug).
- `functor artifact create rfc-0000` means "create an artifact with slug `rfc-0000`"; the generated id is reported in the output.

## RFC Impact

Satisfies RFC-0005 (opaque, immutable, storage independent). The canonical reference `functor:artifact:<id>` is adopted with the full `art_<ULID>` string as the identifier.
