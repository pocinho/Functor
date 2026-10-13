# Functor Workspace Layout

Status: Draft
Related RFCs: RFC-0002, RFC-0011 (projections), RFC-0008
Related ADRs: ADR-0004, ADR-0005, ADR-0009, ADR-0010
Related docs: [crate-dependencies.md](crate-dependencies.md), [dependency-policy.md](dependency-policy.md)

## Purpose

Defines the Rust workspace for the CLI MVP: where code lives, which crates exist now, which are deferred, and which RFCs each crate implements.

## Design Principles

- API-first: contracts in `functor-core`, implementations elsewhere.
- Semantic model at the center; projections at the edge.
- Minimal coupling; one reason to change per crate.
- Grow by adding crates, not by widening existing ones.

## Current State of the Repository

The skeleton in the repository today:

```text
Cargo.toml            # a single [package] "functor" (edition 2024), no [workspace]
.cargo/config.toml    # build.target-dir = "out/cargo"
src/app/cli/          # empty
src/app/desktop/      # empty (future projection)
src/crates/           # empty
```

Earlier drafts of this document described `crates/` at the repository root and a `roadmaps/` directory. Both are obsolete (ADR-0004).

## Target Layout (MVP)

```text
functor/
├── Cargo.toml                 # virtual workspace; no root package
├── rust-toolchain.toml        # pinned toolchain (see development-workflow.md)
├── rustfmt.toml
├── deny.toml                  # cargo-deny: licenses, bans, advisories
├── .cargo/config.toml
├── .github/workflows/ci.yml
├── scripts/                   # check-deps.ps1, check-conformance.ps1, check-docs-links.ps1, bench.ps1, ci.ps1
├── src/
│   ├── crates/
│   │   ├── functor-model/        # domain types, no I/O
│   │   ├── functor-core/         # traits, errors, Clock and IdGenerator
│   │   ├── functor-repository/   # layout, repository.toml, init/open/discover, write lock
│   │   ├── functor-store/        # FileStore, MemoryStore, entity TOML DTOs
│   │   ├── functor-query/        # snapshot query engine
│   │   ├── functor-validation/   # rule catalog and engine
│   │   ├── functor-events/       # event log, rotation, publisher, watcher
│   │   └── functor-testkit/      # test doubles, conformance suite, fixtures
│   └── app/
│       ├── cli/                  # package functor-cli, binary "functor"
│       └── desktop/              # reserved; not a workspace member in the MVP
├── examples/                     # sample repositories used as docs and fixtures (getting-started/, checked by the acceptance test)
├── docs/                         # this documentation (including docs/schemas/ JSON Schemas and docs/benchmarks/ baseline)
├── rfcs/
└── vision/
```

Root `Cargo.toml` outline:

```toml
[workspace]
resolver = "3"
members = ["src/crates/*", "src/app/cli"]

[workspace.package]
edition = "2024"
license = "MIT"
rust-version = "<pinned MSRV>"
publish = false            # no crate is published until an ADR decides otherwise; crates inherit it with `publish.workspace = true`

[workspace.dependencies]   # all external crates and internal path dependencies, versions in one place
[workspace.lints.rust]     # unsafe_code = "forbid", missing_docs = "warn"
[workspace.lints.clippy]   # see development-workflow.md
```

Each crate sets `[lints] workspace = true`.

## Crate Responsibilities

| Crate | Responsibility | Implements |
|-------|----------------|------------|
| `functor-model` | Identifiers, `Artifact`, `Relationship`, `RepositoryInfo`, `RepositorySnapshot`, query, validation and event value types. No I/O, no serde, no runtime | RFC-0001, 0003, 0004, 0005 |
| `functor-core` | Service traits (`async-trait`, dyn-compatible, [ADR-0010](adr/ADR-0010-async-trait-and-dyn.md)), `FunctorError`, `Clock`, `IdGenerator`, system implementations | RFC-0002 |
| `functor-repository` | Foundation service ([ADR-0009](adr/ADR-0009-functor-repository-crate.md)): repository layout and paths, `repository.toml`, `format_version` check, `init`, `open`, `discover`, the write lock | RFC-0015 (subset) |
| `functor-store` | `FileStore` and `MemoryStore`: artifact and relationship persistence, strict and lenient load, atomic writes, in-place TOML edits, entity TOML DTOs | RFC-0008 |
| `functor-query` | `SnapshotQueryEngine`: lookup, search, traverse | RFC-0006 |
| `functor-validation` | Rule catalog `V0001..`, `RuleValidationEngine`, report assembly | RFC-0007 (validation half) |
| `functor-events` | `FileEventLog`: publish, rotation and pruning, subscribe, tailing, event DTO | RFC-0017 (subset) |
| `functor-testkit` | `FixedClock`, `SequentialIds`, fixture builders, store conformance suite, snapshot helpers | testing support |
| `functor-cli` | Argument parsing (clap), command handlers, rendering (text and JSON DTOs), exit-code mapping, composition root | RFC-0011 |

Repository lifecycle lives in its own crate from the start ([ADR-0009](adr/ADR-0009-functor-repository-crate.md)). Versioning (RFC-0016) and federation (RFC-0009) will extend `functor-repository` rather than create it.

## Deferred Crates

| Crate | Reason | RFC |
|-------|--------|-----|
| `functor-runtime` | Execution is out of the MVP | RFC-0007 (execution), 0010 |
| `functor-capabilities` | No capability discovery in the MVP | RFC-0010, 0014 |
| `functor-sync`, `functor-security` | Federation and trust | RFC-0009, 0013 |
| `functor-extensions` | No extensions | RFC-0014 |
| `functor-mcp`, `functor-web`, `functor-editor` | Other projections and integrations | RFC-0011, 0012 |

## RFC to Crate Mapping (MVP)

| RFC | Primary crates | MVP status |
|-----|----------------|------------|
| 0001 | model | Implemented |
| 0002 | core, cli | Implemented |
| 0003 | model, store | Implemented (version as `revision`) |
| 0004 | model, store, validation | Subset (4 built-in types, custom allowed) |
| 0005 | model | Implemented (slug extension in ADR-0001) |
| 0006 | query | Subset |
| 0007 | validation | Validation only |
| 0008 | store | Subset (file store, no history) |
| 0009 | | Deferred |
| 0010 | | Deferred |
| 0011 | cli | Subset (CLI is the only projection) |
| 0012 | | Deferred |
| 0013 | | Deferred (basic local safety in [nonfunctional-requirements.md](nonfunctional-requirements.md)) |
| 0014 | | Deferred |
| 0015 | model, repository, store | Subset |
| 0016 | | Deferred (`revision` counter only) |
| 0017 | events, cli | Subset (ADR-0003) |
| 0018 | | Deferred |

The authoritative, requirement-level table is [rfc-conformance.md](rfc-conformance.md).

## Rules

- Dependencies flow downward only; see [crate-dependencies.md](crate-dependencies.md).
- New crates require a short justification in the pull request and an update to this document.
- Crate names are `functor-<noun>`; the binary is named `functor`.
- All crates are `publish = false` until decided otherwise: set once in `[workspace.package]` and inherited by every crate. Enabling publication for any crate requires an ADR.
