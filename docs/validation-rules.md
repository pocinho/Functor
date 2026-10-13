# Validation Rules (MVP)

Status: Draft
Related RFCs: RFC-0007 (primary), RFC-0003, RFC-0004, RFC-0005
Owner crate: `functor-validation`
Related docs: [error-handling.md](error-handling.md), [storage-design.md](storage-design.md), [cli-spec.md](cli-spec.md)

## Purpose

Defines the rule catalog, severities, outcome derivation, report schema and exit-code policy of `functor validate`. Rule codes are a **stable public contract**: once released, a code is never reused for a different check.

## Model

- A **rule** is a pure function from a `SnapshotLoad` (the lenient load result) to zero or more `Finding`s.
- `validate` runs all rules and aggregates the findings into a `ValidationReport`.
- Validation never modifies the repository and never executes artifact content.
- Execution (RFC-0007) is not part of the MVP; only the validation half is implemented.

## Outcomes (RFC-0007)

| Outcome | Meaning |
|---------|---------|
| `Passed` | No findings at warning or error level, and every rule ran |
| `Warning` | No errors, at least one warning |
| `Failed` | At least one error |
| `Unknown` | No errors, but at least one rule could not run to completion (for example, unreadable files) |

Precedence: `Failed` > `Unknown` > `Warning` > `Passed`. `Info` findings never change the outcome.

## Severities

`Error`: repository data is invalid or inconsistent. `Warning`: valid but suspect or non-portable. `Info`: informational.

## Rule Catalog

| Code | Severity | Description |
|------|----------|-------------|
| `V0001` | Error | Entity file is malformed: invalid TOML, missing or invalid required field, wrong type, too large |
| `V0002` | Error | Missing identifier: an artifact or relationship file has no `id` |
| `V0003` | Error | Duplicate artifact id (two files claim the same id) |
| `V0004` | Error | Duplicate artifact slug |
| `V0005` | Error | Invalid identifier or slug format, or a relationship endpoint that is not a well-formed artifact id |
| `V0006` | Error | Relationship source does not exist (dangling source) |
| `V0007` | Error | Relationship target does not exist |
| `V0008` | Error | Duplicate relationship id |
| `V0009` | Error | Self-referencing relationship of an acyclic type (`depends_on`, `derived_from`, `extends`) |
| `V0010` | Error | Content path is unsafe (absolute, `..`, escapes the repository) or the referenced file does not exist |
| `V0011` | Error | Cycle in an acyclic relationship type (`depends_on`, `derived_from`, `extends`) |
| `V0012` | Warning | Custom (non built-in) relationship type in use |
| `V0013` | Error | Unsupported `format_version` |
| `V0014` | Error | Repository structure violation: missing `.functor/repository.toml`, `artifacts/` or `relationships/` |
| `V0015` | Warning | Duplicate relationship: same `(source, type, target)` appears more than once |
| `V0016` | Warning | Unexpected file in `artifacts/` or `relationships/` (not `*.toml`, not `.gitkeep`), or a stray `.tmp-` file |
| `V0017` | Error | File name does not match the entity id inside it |
| `V0018` | Info | Unknown field in an entity or repository file (kept for forward compatibility) |
| `V0019` | Info | A file or directory could not be read, so rules that needed it could not run to completion; the outcome becomes `Unknown` unless an error exists |

Mapping from the MVP scope list in [mvp-scope.md](mvp-scope.md):

| Scope item | Rules |
|------------|-------|
| Missing identifiers | `V0001`, `V0002` |
| Duplicate identifiers | `V0003`, `V0004`, `V0008` |
| Missing relationship targets | `V0007` (and `V0006` for sources) |
| Invalid references | `V0005`, `V0010` |
| Repository structure violations | `V0013`, `V0014`, `V0016`, `V0017` |
| Cycle detection (RFC-0004 example) | `V0009`, `V0011` |

## Finding Shape

```rust
pub struct Finding {
    pub code: RuleCode,          // "V0007"
    pub severity: Severity,
    pub subject: Subject,        // Artifact(id) | Relationship(id) | Repository | File(path)
    pub message: String,         // one sentence, no trailing period
    pub related: Vec<Subject>,   // e.g. the other duplicate, or the nodes of a cycle
}
```

Messages for the same code are stable and parametrized by subject only. Order of findings: code ascending, subject (kind, then id or path), message.

Conventions (M7):

- A finding raised from a file the loader rejected or flagged (`V0001` to `V0005`, `V0008` to `V0010`, `V0013`, `V0014`, `V0016` to `V0019`) has the **file** as its subject, with the loader's explanation as the message (a multi-line parser message is reduced to its first and last line), and the other file of a duplicate in `related`.
- Graph rules have entity subjects: `V0006`, `V0007` and `V0012` the relationship; `V0015` the relationship with the lowest id of the group, with the other relationships in `related`; `V0011` the artifact with the lowest id of the cycle, with every participant (sorted by id, including that one) in `related` and their slugs in the message.

## Cycle Detection (`V0011`)

Run Tarjan's strongly connected components on the subgraph of acyclic types (each type separately). Each SCC with more than one node produces **one** finding listing the participating artifacts (sorted by id) in `related`. Self-loops are covered by `V0009`.

## Dependency Between Rules

Rules that need the graph (`V0006`, `V0007`, `V0011`) run on the well-formed subset of entities. A malformed file is reported by `V0001` and excluded; relationships pointing to an excluded artifact then also raise `V0007`, which is intended (the user sees both the cause and the effect).

If a rule cannot complete (for example, a directory cannot be listed), the engine does not fail: the loader reports the unreadable path, rule `V0019` turns that into an info finding naming it, and the outcome becomes `Unknown` unless an error exists. `V0019` is the "unknown marker" of the model; it is informational so that it never hides a real error.

## Report

```rust
pub struct ValidationReport {
    pub outcome: Outcome,
    pub findings: Vec<Finding>,
    pub counts: Counts,          // artifacts, relationships, errors, warnings, infos
    pub completed: Timestamp,
}
```

`text` and `json` renderings are specified in [cli-spec.md](cli-spec.md). After a run, the CLI appends a `validation.completed` event with the outcome and counts (see [event-model-mvp.md](event-model-mvp.md)). Reports are not persisted in the MVP.

## Exit Code Policy

| Outcome | Exit code | With `--strict` |
|---------|-----------|-----------------|
| `Passed` | 0 | 0 |
| `Warning` | 0 | 6 |
| `Unknown` | 6 | 6 |
| `Failed` | 6 | 6 |

`Unknown` fails by default because the repository could not be proven valid. `--strict` additionally fails on warnings, intended for CI.

## Scoped Validation

`functor validate [<artifact>]` restricts **reported** findings to those about the artifact: its own findings, the findings of relationships touching it, and findings about the files that define them (whatever the files are called). A cycle or a group of duplicate relationships is reported for every artifact that takes part. Rules still run on the whole snapshot, because uniqueness and reachability need it. Repository-level findings (`V0013`, `V0014`, `V0019` and anything about `.functor/repository.toml`) are always included. The outcome and the error, warning and info counts describe the reported findings; the artifact and relationship counts describe the whole repository. An artifact that did not load cleanly cannot be scoped to (`NotFound`).

## Extending Rules

Adding a rule means: allocate the next code, add a row to the catalog, implement the `Rule`, add tests (below) and a row in [rfc-conformance.md](rfc-conformance.md). Rule implementations must not depend on each other. Third-party rules (extensions, RFC-0014) are out of scope.

## Test Obligations

- One minimal fixture repository **per rule code** that triggers exactly that code, plus a clean repository that triggers none.
- Outcome derivation table tests, exit-code mapping tests with and without `--strict`.
- Determinism: the same fixture produces byte-identical `json` output on repeated runs and on both stores.
- Cycle tests: two-node, long cycle, two disjoint cycles, mixed types (no cross-type cycle).

## Implementation Notes (M7)

- **Layers.** `functor-validation` has pure rules (`rules.rs`, a table of 19 `Rule { code, check }` functions over a `SnapshotLoad`), `evaluate` (runs the rules, scopes and assembles the `ValidationReport`) and `RuleValidationEngine<'a, R>`, which borrows any `RepositoryReader`, loads leniently once per call and stamps the report with the injected clock. The catalog (`CATALOG`, `rule_info`) is a constant table; a test compares its codes and severities with the table above, so the two cannot drift.
- **`LoadIssue` gained context.** The loader now records, where known, the `entity` a file defines and `related` subjects (the file that first defined a duplicate id or slug). Scoped validation uses them; `V0003`, `V0004` and `V0008` use `related`.
- **`V0019`.** Added in M7 to make the `Unknown` outcome explainable (see above).
- **Cycles** use an iterative Tarjan (no recursion, tested on a 200 000-node ring) and are checked against a brute-force mutual-reachability oracle by a property test.
- **Real-file coverage.** `functor-validation/tests/engine.rs` breaks a real `FileStore` repository once per rule code (`V0001` to `V0018`; `V0019` needs an unreadable path and is covered by the unit tests and a CLI test that replaces a directory with a file). A duplicate id cannot also have a matching file name, so `V0003` and `V0008` fixtures also raise `V0017`.
- **Not done in the MVP.** Content-based checks, severity overrides and third-party rules.
