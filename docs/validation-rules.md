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

## Cycle Detection (`V0011`)

Run Tarjan's strongly connected components on the subgraph of acyclic types (each type separately). Each SCC with more than one node produces **one** finding listing the participating artifacts (sorted by id) in `related`. Self-loops are covered by `V0009`.

## Dependency Between Rules

Rules that need the graph (`V0006`, `V0007`, `V0011`) run on the well-formed subset of entities. A malformed file is reported by `V0001` and excluded; relationships pointing to an excluded artifact then also raise `V0007`, which is intended (the user sees both the cause and the effect).

If a rule cannot complete (for example, a directory cannot be listed), the engine records an `Unknown` marker for that rule instead of failing, and the outcome becomes `Unknown` unless an error exists.

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

`functor validate [<artifact>]` restricts **reported** findings to those whose subject is the artifact or a relationship touching it. Rules still run on the whole snapshot, because uniqueness and reachability need it. Repository-level findings (`V0013`, `V0014`) are always included.

## Extending Rules

Adding a rule means: allocate the next code, add a row to the catalog, implement the `Rule`, add tests (below) and a row in [rfc-conformance.md](rfc-conformance.md). Rule implementations must not depend on each other. Third-party rules (extensions, RFC-0014) are out of scope.

## Test Obligations

- One minimal fixture repository **per rule code** that triggers exactly that code, plus a clean repository that triggers none.
- Outcome derivation table tests, exit-code mapping tests with and without `--strict`.
- Determinism: the same fixture produces byte-identical `json` output on repeated runs and on both stores.
- Cycle tests: two-node, long cycle, two disjoint cycles, mixed types (no cross-type cycle).
