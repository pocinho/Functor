# ADR-0014: Evidence relationship vocabulary

## Status

Accepted. Amends [ADR-0007](ADR-0007-relationship-types.md).

## Context

[ADR-0007](ADR-0007-relationship-types.md) kept `supports`, `contradicts` and `validates` as custom types (warning `V0012`). Phase 1 needs defined direction and semantics for evidence assertions and for provenance, without breaking stored repositories.

## Decision

1. **New built-in types:** `supports`, `contradicts` and `validates`. Spelling stays snake_case. Existing stored files already use exactly these strings, so no file changes and no migration: they now parse as built-ins instead of `Custom`. `relation add` no longer needs `--allow-custom-type` for them (the flag is still accepted). `V0012` no longer fires for them; every other custom name still does.
2. **Direction:** `source --supports--> target` reads "source is evidence supporting the claim target"; `contradicts` is the same with opposite effect; `validates`: a validation report (source) validates its subject (target). Reverse lookups remain query concerns.
3. **Admissible endpoints:** any two existing artifacts. Human-authored evidence is valid, so there is no type restriction on the source. An execution failure creates no `contradicts` edge by itself; assertions are made deliberately.
4. **Rules:** self-reference of `supports`, `contradicts` and `validates` is rejected (model constructor, `V0009` on load). Cycles are allowed (not acyclic types), like `references`. Duplicates keep `V0015`. A new warning `V0023`: one source both `supports` and `contradicts` the same target.
5. **Provenance uses existing types, no new vocabulary:**

   | Edge | Meaning |
   |------|---------|
   | `capture --derived_from--> live artifact` | the capture was taken from that artifact |
   | `run --depends_on--> capture` | the run consumed that exact capture (workflow and inputs) |
   | `run_result --derived_from--> run` | terminal record of that run |
   | `output --derived_from--> run_result` | the output was produced by that run |
   | `validation_report --validates--> subject` | report evaluated the subject (see [ADR-0015](ADR-0015-validation-report-persistence-and-events.md)) |

   `derived_from` and `depends_on` stay acyclic, and these edges form a DAG because evidence is append-only: a later run consuming an earlier output depends on a **new** capture.
6. **Traversal:** conclusion or report to outputs, run, workflow and original inputs follows outgoing `validates`, `derived_from`, `depends_on`; the reverse from an input follows incoming edges. Both use the existing query traversal; no new query API is required.
7. **Marking promotion for the matrix:** C-0004-2 changes from Subset to Implemented only when `supports`, `contradicts` and `validates` have tests (RFC-0004 core types are then all built-in).

## Consequences

- `V0012` stops reporting three names. This is a deliberate edit of an informational rule's scope, pinned by updated tests (frozen-contracts policy). The `V0009` description is extended to name the new self-reference types.
- ADR-0007 test `model:custom_types_are_flagged_and_validated` must switch to a different custom name; this is a required P1.1 obligation.
- Old binaries read these as custom types and warn `V0012`; they still load and traverse them.

## RFC Impact

Implements the rest of RFC-0004 core types (C-0004-2) and the RFC-0007 "supports/contradicts" evidence semantics. `validates` is also an RFC-0001 relationship name.
