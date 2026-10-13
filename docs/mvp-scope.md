# MVP Scope

Status: Draft
Related RFCs: RFC-0000 to RFC-0008, RFC-0011, RFC-0015, RFC-0017 (subsets)
Related docs: [cli-mvp-roadmap.md](cli-mvp-roadmap.md), [cli-spec.md](cli-spec.md), [rfc-conformance.md](rfc-conformance.md), [adr/README.md](adr/README.md)

## Purpose

Defines what is in and out of the Functor CLI MVP. The MVP exists to validate the architectural foundations of the RFC series (API-first, semantic model, projections) with the least implementation complexity. Success is measured by proving the architecture, not by feature completeness.

## Included Scope

### Repository management

```bash
functor init
```

Create a repository: metadata (`repository.toml` with identity and format version) and the directory structure ([repository-layout.md](repository-layout.md)).

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

Built-in relationship types: `depends_on`, `references`, `derived_from`, `extends`. Other well-formed type names are accepted with `--allow-custom-type` and flagged by validation (ADR-0007). Relationships are immutable.

### Query operations

```bash
functor query artifacts
functor query dependencies <artifact>
functor query dependents <artifact>
functor query relationships <artifact>
```

Lookup, search with filters and relationship traversal with direction and depth ([query-model-mvp.md](query-model-mvp.md)).

### Validation

```bash
functor validate [artifact] [--strict]
```

Rule catalog `V0001` to `V0019`: missing and duplicate identifiers, missing relationship endpoints, invalid references, dependency cycles, repository structure violations, unreadable paths ([validation-rules.md](validation-rules.md)).

### Events

```bash
functor watch
```

Event subset `artifact.created`, `artifact.updated`, `relationship.created`, `validation.completed`, stored as JSON Lines and streamed by `watch` (ADR-0003, [event-model-mvp.md](event-model-mvp.md)).

### Storage

Local filesystem, TOML for entities, JSON Lines for events, entity-per-file layout ([storage-design.md](storage-design.md)).

### Cross-cutting

- Text and JSON output modes with stable JSON schemas and exit codes ([cli-spec.md](cli-spec.md)).
- Platforms: Windows (x86_64) is supported and blocking; Linux and macOS are best-effort and non-blocking ([nonfunctional-requirements.md](nonfunctional-requirements.md)).
- The CLI operates **exclusively** through the core API traits.

## Excluded Scope

| Excluded | RFC | Note |
|----------|-----|------|
| Federation and synchronization | 0009 | |
| Dynamic capability discovery | 0010 | |
| Non-CLI projections (desktop, web, notebook) | 0011 | `src/app/desktop` is a placeholder only |
| MCP and interoperability layers | 0012 | |
| Security and trust infrastructure | 0013 | Only the local safety baseline in [nonfunctional-requirements.md](nonfunctional-requirements.md) |
| Extensions and plugins | 0014 | |
| Versioning and history | 0016 | `revision` is a counter, with no history |
| Event replay, subscription languages | 0017 | Only the subset above is included (log rotation is included, [ADR-0008](adr/ADR-0008-event-log-rotation.md)) |
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

and receive correct, reproducible results, with exit code 0 and output matching the golden files in the acceptance test ([testing-strategy.md](testing-strategy.md)). The walkthrough with expected output is [getting-started.md](getting-started.md).

## Exit Criteria

The MVP is complete when all of the following are true:

1. Every included command works as specified in [cli-spec.md](cli-spec.md), with text and JSON output.
2. The acceptance workflow above passes on Windows in CI. Results on Linux and macOS are informational.
3. `functor validate` reports each rule `V0001` to `V0019` correctly on its fixture repository.
4. `MemoryStore` and `FileStore` pass the same conformance suite.
5. The CLI depends on core traits only for behavior (verified by the dependency-edge check in [crate-dependencies.md](crate-dependencies.md)); no domain logic lives in `functor-cli`.
6. Adding a second projection would not require changes to `functor-model` or `functor-core` (demonstrated by the library-level integration tests, which exercise the whole workflow without the CLI).
7. Every row of [rfc-conformance.md](rfc-conformance.md) has a status, and every "Implemented" or "Subset" row names a test.
8. Format and output contracts (artifact format, JSON schemas, rule codes, exit codes) are frozen and documented.
9. `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` and `cargo deny check` pass in CI.

## Non-Goals

- Performance beyond the targets in [nonfunctional-requirements.md](nonfunctional-requirements.md).
- Backward compatibility with pre-MVP prototypes (none exist).
- Stability promises for Rust APIs before the first release; contracts that **are** stable are listed in exit criterion 8.
