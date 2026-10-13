# Artifact Format

Status: Draft
Related RFCs: RFC-0003 (primary), RFC-0005, RFC-0008
Related ADRs: ADR-0001, ADR-0005, ADR-0006
Owner crate: `functor-store` (DTOs); semantics in [domain-model.md](domain-model.md)

## Purpose

Defines the canonical TOML representation of an artifact in a repository for the CLI MVP. The format is human-readable, version-control friendly, deterministic and extensible.

## Design Principles

1. Identity is stable and opaque; the slug is a mutable alias (ADR-0001).
2. One artifact per file, named by artifact id (ADR-0006).
3. Metadata is explicit and extensible; it never changes semantics.
4. Relationships are **not** stored in artifacts; they are separate entities.
5. Serialization is deterministic: the same artifact always produces byte-identical output.
6. Projections must not alter semantics.

## File Location

`artifacts/<artifact-id>.toml`, for example `artifacts/art_01J9Z3K8M2Q7X4V6B5N1R0T8WD.toml`. The id inside the file **must** equal the file stem (rule `V0017`).

## Minimum Artifact

```toml
id = "art_01J9Z3K8M2Q7X4V6B5N1R0T8WD"
slug = "rfc-0000"
type = "rfc"
title = "Functor Vision"
revision = 1
created = 2026-10-12T18:00:00Z
updated = 2026-10-12T18:00:00Z
```

## Complete Example

```toml
id = "art_01J9Z3K8M2Q7X4V6B5N1R0T8WD"
slug = "rfc-0000"
type = "rfc"
title = "Functor Vision"
revision = 3
description = "Platform vision and goals"
created = 2026-10-12T18:00:00Z
updated = 2026-10-14T09:30:00Z
tags = ["architecture", "core", "rfc"]

[metadata]
author = "Paulo"
status = "draft"

[content]
format = "markdown"
path = "content/rfc-0000.md"
```

## Fields

| Field | Type | Required | Rules |
|-------|------|----------|-------|
| `id` | string | yes | `art_<ULID>`; immutable |
| `slug` | string | yes | grammar in [identity-and-addressing.md](identity-and-addressing.md); unique in the repository |
| `type` | string | yes | `[a-z][a-z0-9_]{0,31}`. RFC-0003 "Type"; an earlier draft called this `kind`. `kind` is **not** accepted |
| `title` | string | yes | 1 to 200 characters, trimmed, no control characters |
| `revision` | integer | yes | >= 1. RFC-0003 "Version" in its MVP form: starts at 1 and increments on each update |
| `created` | datetime | yes | TOML offset date-time, UTC, second precision, written with `Z` |
| `updated` | datetime | yes | same format; `updated >= created` |
| `description` | string | no | free text; omitted when absent (not written as empty) |
| `tags` | array of strings | no | sorted, unique, grammar `[a-z0-9][a-z0-9_-]{0,39}`; omitted when empty |
| `metadata` | table | no | keys follow the tag grammar; values are string, integer, boolean or array of strings |
| `content` | table | no | see below |

Required fields are a superset of the earlier minimum (`id`, `kind`, `title`) because RFC-0003 requires Identity, Type, Metadata, Version and Content to be *provided by the model*: slug, revision and timestamps are always present so every consumer gets a complete record. `metadata` and `content` default to empty and `None`.

## Metadata

Free-form but typed (string, integer, boolean, array of strings). Implementations may add keys without breaking compatibility. Reserved keys are not defined in the MVP.

## Content

Content is optional. Two forms:

Inline:

```toml
[content]
format = "markdown"
text = """
# Notes
...
"""
```

Referenced:

```toml
[content]
format = "markdown"
path = "content/rfc-0000.md"
```

Rules:

- Exactly one of `text` and `path`. Both or neither (with a `[content]` table present) is a format error.
- `format` is required when `content` is present. Well-known values: `markdown`, `text`, `toml`, `json`. Unknown values are allowed; consumers decide how to treat them (RFC-0003).
- `path` is **relative to the repository root**, uses `/` separators, may not be absolute, may not contain `..` segments and may not traverse a symbolic link outside the repository. Violations are validation errors (`V0010`) and are refused at write time.
- Referenced files are plain files under the repository (conventionally `content/`). The CLI never executes content (see [nonfunctional-requirements.md](nonfunctional-requirements.md)).
- Content size limit for parsing: 1 MiB inline, enforced by the loader.

## Artifact Types

Suggested well-known types: `document`, `rfc`, `note`, `specification`, `dataset`, `workflow`, `source`, `report`. The set is open; unknown types are valid.

## Canonical Serialization

Writers must emit this exact form so that diffs are minimal and runs are reproducible:

- UTF-8, no byte order mark, LF line endings (the repository's `.gitattributes` also normalizes line endings), exactly one trailing newline.
- Key order: `id`, `slug`, `type`, `title`, `revision`, `description`, `created`, `updated`, `tags`, then tables `[metadata]` (keys sorted ascending) and `[content]` (`format`, then `text` or `path`).
- Strings use basic quoted strings; text containing newlines uses multi-line basic strings.
- Arrays are written on one line when they fit in 80 columns, otherwise one element per line.
- Optional fields that are absent are omitted.
- Timestamps are `YYYY-MM-DDThh:mm:ssZ`.

Unknown top-level keys and tables found when reading are **preserved** (written after the known keys, sorted by key) so editing with an older tool does not lose data written by a newer one. Comments and original formatting are not preserved on rewrite. See [storage-design.md](storage-design.md).

## Compatibility

- The format is governed by `format_version` in `.functor/repository.toml` (currently `1`).
- Readers must tolerate unknown fields (info finding `V0018`) but must refuse a repository whose `format_version` is greater than they support (`UnsupportedFormat`).
- Compatible changes (new optional fields) do not bump the version. Changes to required fields, meanings or the key order contract do.

## Validation Rules for Artifacts

An artifact file is valid when the rules `V0001`, `V0002`, `V0003`, `V0004`, `V0005`, `V0010` and `V0017` in [validation-rules.md](validation-rules.md) all pass: it parses, has all required fields with valid values, its id matches the filename, its id and slug are unique and its content path is safe and exists.

## Future Work

Embedded content formats, binary artifact references, schema validation per type, cross-repository references (RFC-0009), artifact signing (RFC-0013) and real versions and history (RFC-0016).
