# ADR-0006: One file per artifact and per relationship

## Status

Accepted

## Context

`repository-layout.md` stored all relationships in one `relationships/relationships.toml` and named artifact files after human names. A single relationships file creates merge conflicts, rewrites the whole file on each change, and gives relationships no natural identity (RFC-0004 wants stable relationship ids). Naming files after slugs would make slug renames move files and would introduce case-insensitive filesystem collisions on Windows and macOS.

## Decision

- Each artifact is stored in `artifacts/<artifact-id>.toml`.
- Each relationship is stored in `relationships/<relationship-id>.toml`.
- Files are named by id. The slug is a field inside the artifact file.
- Ids use only characters safe on all supported filesystems (ASCII letters, digits and underscore), so no path escaping is required.

## Consequences

- Directory listing is by id and sorts chronologically (ULID), which gives a natural creation order.
- Humans browsing the directory see ids, not names; `functor artifact list` is the discovery tool.
- Many small files; acceptable for the MVP target size (see [../nonfunctional-requirements.md](../nonfunctional-requirements.md)).
- Git diffs and merges are per entity.

## RFC Impact

Supports RFC-0005 (identity independent of filename and location) and RFC-0004 (stable relationship ids).
