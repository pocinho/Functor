# MVP Scope

Status: Accepted
Related RFCs: RFC-0000 to RFC-0008, RFC-0011, RFC-0015, RFC-0017 (subsets)
Related docs: [cli-mvp-roadmap.md](cli-mvp-roadmap.md), [cli-spec.md](../cli/cli-spec.md), [rfc-conformance.md](rfc-conformance.md), [adr/README.md](../adr/README.md)

## Purpose

Defines what is in and out of the Functor CLI MVP. The MVP exists to validate the architectural foundations of the RFC series (API-first, semantic model, projections) with the least implementation complexity. Success is measured by proving the architecture, not by feature completeness.

This is the completed Phase 0 baseline, not the scope of a product release. Its historical included/excluded scope and exit evidence remain below. The remaining working-prototype phases are specified in [product-roadmap.md](product-roadmap.md); the next implementation increment is [Phase 1: Execution and Provenance](phase-1-execution-provenance-roadmap.md). Later phases may implement these exclusions through explicit new contracts without retroactively expanding the MVP.

## Included Scope

### Repository management

```bash
functor init
```

Create a repository: metadata (`repository.toml` with identity and format version) and the directory structure ([repository-layout.md](../design/repository-layout.md)).

### Artifact management

```bash
functor artifact create <slug>
functor artifact show <artifact>
functor artifact list
functor artifact update <artifact>
```

Create, load, list and update artifacts. Artifacts have an opaque id and a unique slug (ADR-0001) and a `revision` counter. There is **no delete** in the MVP (see exclusions).

### Relationship management

```bash
functor relation add <source> <type> <target>
functor relation list
functor relation show <relation>
```

Built-in relationship types: `depends_on`, `references`, `derived_from`, `extends` (Phase 1 adds `supports`, `contradicts` and `validates`, [ADR-0014](../adr/ADR-0014-evidence-relationships.md)). Other well-formed type names are accepted with `--allow-custom-type` and flagged by validation (ADR-0007). Relationships are immutable.

### Query operations

```bash
functor query artifacts
functor query dependencies <artifact>
functor query dependents <artifact>
functor query relationships <artifact>
```

Lookup, search with filters and relationship traversal with direction and depth ([query-model-mvp.md](../design/query-model-mvp.md)).

### Validation

```bash
functor validate [artifact] [--strict]
```

Rule catalog `V0001` to `V0019`: missing and duplicate identifiers, missing relationship endpoints, invalid references, dependency cycles, repository structure violations, unreadable paths ([validation-rules.md](../design/validation-rules.md)).

### Events

```bash
functor watch
```

Event subset `artifact.created`, `artifact.updated`, `relationship.created`, `validation.completed`, stored as JSON Lines and streamed by `watch` (ADR-0003, [event-model-mvp.md](../design/event-model-mvp.md)).

### Storage

Local filesystem, TOML for entities, JSON Lines for events, entity-per-file layout ([storage-design.md](../design/storage-design.md)).

### Cross-cutting

- Text and JSON output modes with stable JSON schemas and exit codes ([cli-spec.md](../cli/cli-spec.md)).
- Platforms: Windows (x86_64) is supported and blocking; Linux and macOS are best-effort and non-blocking ([nonfunctional-requirements.md](../quality/nonfunctional-requirements.md)).
- The CLI operates **exclusively** through the core API traits.

## Excluded Scope

| Excluded | RFC | Note |
|----------|-----|------|
| Federation and synchronization | 0009 | |
| Dynamic capability discovery | 0010 | |
| Non-CLI projections (desktop, web, notebook) | 0011 | `src/app/desktop` is a placeholder only |
| MCP and interoperability layers | 0012 | |
| Security and trust infrastructure | 0013 | Only the local safety baseline in [nonfunctional-requirements.md](../quality/nonfunctional-requirements.md) |
| Extensions and plugins | 0014 | |
| Versioning and history | 0016 | `revision` is a counter, with no history |
| Event replay, subscription languages | 0017 | Only the subset above is included (log rotation is included, [ADR-0008](../adr/ADR-0008-event-log-rotation.md)) |
| Governance | 0018 | |
| Execution engine | 0007 | Validation only |
| Artifact and relationship deletion, relationship update | | Needs a deletion and tombstone policy |
| Persisted validation reports | 0007 | Reports are printed and summarized in an event |
| Interactive prompts, shell completion, config files | | Possible later; not required to validate the architecture |
| Remote repositories, authentication, encryption | | |
| Import and export, repository migration | | `format_version = 1` only |

## Success Criteria

A user can run, on a clean directory:

```bash
functor init
functor artifact create rfc-0000
functor artifact create rfc-0001
functor relation add rfc-0001 depends_on rfc-0000
functor query dependencies rfc-0001
functor validate
```

and receive correct, reproducible results, with exit code 0 and output matching the golden files in the acceptance test ([testing-strategy.md](../quality/testing-strategy.md)). The walkthrough with expected output is [getting-started.md](../cli/getting-started.md).

## Exit Criteria

The MVP is complete when all of the following are true:

1. Every included command works as specified in [cli-spec.md](../cli/cli-spec.md), with text and JSON output.
2. The acceptance workflow above passes on Windows in CI. Results on Linux and macOS are informational.
3. `functor validate` reports each rule `V0001` to `V0019` correctly on its fixture repository.
4. `MemoryStore` and `FileStore` pass the same conformance suite.
5. The CLI depends on core traits only for behavior (verified by the dependency-edge check in [crate-dependencies.md](../architecture/crate-dependencies.md)); no domain logic lives in `functor-cli`.
6. Adding a second projection would not require changes to `functor-model` or `functor-core` (demonstrated by the library-level integration tests, which exercise the whole workflow without the CLI).
7. Every row of [rfc-conformance.md](rfc-conformance.md) has a status, and every "Implemented" or "Subset" row names a test.
8. Format and output contracts (artifact format, JSON schemas, rule codes, exit codes) are frozen and documented.
9. `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` and `cargo deny check` pass in CI.

### Exit Evidence

Status at the end of M9. Test references are checked by `scripts/check-conformance.ps1`.

| # | Evidence |
|---|----------|
| 1 | One end-to-end suite per command, each with text, JSON and failure cases: `cli:init`, `cli:artifact`, `cli:relation`, `cli:query`, `cli:validate`, `cli:events`, `cli:watch` |
| 2 | `cli:acceptance` runs on the blocking Windows job (`scripts/ci.ps1`, `.github/workflows/ci.yml`); Linux and macOS jobs are informational |
| 3 | `val:rules::the_rule_table_lists_every_catalog_code_in_order`, `val:catalog::the_catalog_matches_the_documentation`, and a triggering and a non-triggering test for each rule in `val:engine` |
| 4 | `store:memory::memory_store_passes_the_conformance_suite`, `store:file_store` (the same suite on the file store) and `store:caching::the_caching_store_passes_the_conformance_suite` |
| 5 | `script:check-deps.ps1` |
| 6 | `acc:the_success_path_and_the_negative_variant_run_through_the_libraries`, `acc:a_repository_written_by_the_libraries_is_read_by_the_cli`, `acc:a_repository_written_by_the_cli_is_read_by_the_libraries` |
| 7 | [rfc-conformance.md](rfc-conformance.md), checked by `script:check-conformance.ps1` |
| 8 | The table below |
| 9 | `scripts/ci.ps1` (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace --all-features`, and `cargo test -p functor-cli` for the release feature set), `cargo deny check` and `cargo doc` with `RUSTDOCFLAGS=-D warnings` |

### Frozen Contracts

These are stable from the MVP on. A change is a new version (`format_version` for files, `schema_version` for JSON, `v` for events), an ADR and, for rule codes and exit codes, a deliberate edit of the table and the tests that pin it; it is never a side effect of another change.

| Contract | Specified in | Pinned by |
|----------|--------------|-----------|
| Artifact and relationship file format (`format_version` 1), repository layout | [artifact-format.md](../design/artifact-format.md), [repository-layout.md](../design/repository-layout.md), [storage-design.md](../design/storage-design.md) | `store:minimal_artifact_renders_canonically`, `store:renders_the_documented_example`, `repo:renders_the_canonical_form` |
| JSON output of every command (`schema_version` 1) | [cli-spec.md](../cli/cli-spec.md), `docs/cli/schemas/` | `cli:query::the_schemas_can_fail`, `cli:artifact::the_schemas_can_actually_fail`, the golden transcripts in `src/app/cli/tests/snapshots/` |
| Validation rule codes `V0001` to `V0019` | [validation-rules.md](../design/validation-rules.md) | `val:catalog::the_catalog_matches_the_documentation` |
| Exit codes | [cli-spec.md](../cli/cli-spec.md), [error-handling.md](../architecture/error-handling.md) | `cli:exit::every_error_kind_has_the_documented_exit_code`, `cli:exit::validation_outcomes_map_to_the_documented_exit_codes` |
| Event envelope (`v` 1) and event types | [event-model-mvp.md](../design/event-model-mvp.md) | `events:wire::encoding_uses_the_documented_key_order_and_shape`, `cli:events::the_event_log_after_a_typical_session_is_stable` |

## Non-Goals

- Performance beyond the targets in [nonfunctional-requirements.md](../quality/nonfunctional-requirements.md).
- Backward compatibility with pre-MVP prototypes (none exist).
- Stability promises for Rust APIs before the first release; contracts that **are** stable are listed in exit criterion 8.
