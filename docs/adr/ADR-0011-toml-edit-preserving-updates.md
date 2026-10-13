# ADR-0011: toml_edit; updates preserve comments and formatting

## Status

Accepted

## Context

The first version of [../artifact-format.md](../artifact-format.md) said comments and formatting are lost when an entity file is rewritten. That is hostile to people who hand-edit TOML in a git repository. The project owner decided updates must preserve comments, ordering and formatting.

## Decision

- **New** artifact, relationship and repository files are written in the canonical form defined in [../artifact-format.md](../artifact-format.md).
- **Updates edit the existing document in place** using `toml_edit::DocumentMut`: only the changed values, plus `revision` and `updated`, are modified. Comments, key order, whitespace, unknown keys and unknown tables are preserved by construction.
- Reading uses `toml_edit` (its serde support or direct document access) to build the storage DTOs. The `toml` crate is dropped from the approved dependencies.
- An update that changes nothing writes nothing.
- The atomic write protocol is unchanged: the edited text is written to a temporary file and renamed ([../storage-design.md](../storage-design.md)).

## Consequences

- Byte-identical output is guaranteed for files created by the tool and updated with the same operations. Hand-edited files keep the author's formatting by design, so the property test "serialize, parse, serialize is stable" applies to **new files**; updates are covered by preservation tests (comments, key order, unknown keys, whitespace).
- Editing code is more involved than struct serialization: setting a key that does not exist must insert it at the canonical position; removing a key (`--clear-description`) must remove its leading comment lines only if they were attached directly to it.
- Value types written through `toml_edit` must keep canonical formatting (timestamps `YYYY-MM-DDThh:mm:ssZ`, one-line arrays that fit in 80 columns).

## RFC Impact

None. Supports RFC-0008 portability and human-readable storage.
