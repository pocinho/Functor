# CLI MVP Roadmap

Status: Draft
Related docs: [mvp-scope.md](mvp-scope.md), [workspace-layout.md](workspace-layout.md), [testing-strategy.md](testing-strategy.md), [development-workflow.md](development-workflow.md), [product-roadmap.md](product-roadmap.md) (long term)

## Purpose

The ordered, milestone-level plan to build the CLI MVP. The goal is not to implement every RFC; it is to validate the core architectural assumptions of RFC-0000 to RFC-0018 through a working API-first implementation. Scope is in [mvp-scope.md](mvp-scope.md); this document says in what order and with what acceptance evidence.

## Guiding Rules

- **API first.** Each capability is implemented and tested through core traits before the CLI exposes it. The CLI never becomes the system of record.
- **Semantic first.** Follow the RFC-derived model in [domain-model.md](domain-model.md).
- **Every milestone is usable and shippable.** It leaves `main` green: format, clippy, tests and the dependency-edge check pass.
- **Tests with the code.** A milestone is not done until its test obligations in the linked docs exist.
- **Contracts before code.** If implementation requires changing a contract doc, change the doc (or add an ADR) in the same pull request.

## Milestone Map

```text
M0 Bootstrap -> M1 Model -> M2 Core + Testkit -> M3 Storage -> M4 Artifact CLI
   -> M5 Relationship CLI -> M6 Query -> M7 Validation -> M8 Events -> M9 Hardening
```

M6, M7 and M8 depend only on M5 and may proceed in parallel.

## M0: Workspace Bootstrap

- Convert the root `Cargo.toml` to a virtual workspace and create the empty crates ([workspace-layout.md](workspace-layout.md), ADR-0004).
- `rust-toolchain.toml`, `rustfmt.toml`, workspace lints, `deny.toml`.
- CI workflow: fmt, clippy, test, deny on Windows (blocking), with Linux and macOS test jobs as non-blocking ([development-workflow.md](development-workflow.md)).
- Dependency-edge check script ([crate-dependencies.md](crate-dependencies.md)).

Done when: CI is green on Windows with empty crates and `functor --version` runs.

Status: implemented. `scripts/ci.ps1` (fmt, clippy, dependency edges, tests, links), `cargo deny check` and `functor --version` pass locally on Windows; the first GitHub Actions run happens when the branch is pushed.

## M1: Semantic Model (`functor-model`)

RFC-0001, 0003, 0004, 0005. Identifiers, slug, artifact, relationship and type enums, snapshot with indexes, query, validation and event value types.

Done when: every invariant in [domain-model.md](domain-model.md) has a unit or property test, and artifacts and relationships can be created entirely in memory.

Status: implemented. `functor-model` has all types from `domain-model.md` (see its "Implementation Notes"), 52 unit tests, 14 property tests and an in-memory end-to-end test. Search matching and traversal execution belong to M6 (`functor-query`).

## M2: Core Contracts and Testkit (`functor-core`, `functor-testkit`)

RFC-0002. Traits ([traits-and-interfaces.md](traits-and-interfaces.md)), `FunctorError` ([error-handling.md](error-handling.md)), `Clock`, `IdGenerator`, test doubles and the **store conformance suite** (written against the traits, initially without an implementation).

Done when: the traits compile with doc comments and the conformance suite compiles; the error-to-exit-code mapping function has its exhaustive test.

Status: implemented.
- `functor-core`: all service traits (dyn-compatible, with a compile-time test), `FunctorError` and `ErrorKind`, `LoadIssue` types with `SnapshotLoad::into_snapshot`, `SystemClock` and `UlidGenerator`.
- `functor-testkit`: `FixedClock`, `SequentialIds`, `RepoFixture` and the store conformance suite (14 scenarios), validated by a reference store that passes it and six deliberately broken stores that it rejects.
- `functor-cli`: `exit_code(ErrorKind)` with its table test.

Real stores (`MemoryStore`, `FileStore`) run the suite in M3.

## M3: Repository and Storage (`functor-repository`, `functor-store`)

RFC-0008, 0015 subset ([ADR-0009](adr/ADR-0009-functor-repository-crate.md)).

1. `functor-repository`: layout and path helpers, `repository.toml`, `format_version` check, `init`, `open`, `discover`, the write lock ([repository-layout.md](repository-layout.md), [repository-lifecycle.md](repository-lifecycle.md)).
2. `functor-store`: `MemoryStore` first (passes the conformance suite), then `FileStore`: artifact and relationship persistence, strict and lenient loading, atomic writes and in-place `toml_edit` updates ([storage-design.md](storage-design.md), [artifact-format.md](artifact-format.md), [ADR-0011](adr/ADR-0011-toml-edit-preserving-updates.md)).

CLI: `functor init`.

Done when: `functor-repository` passes its lifecycle and lock tests; both stores pass the conformance suite; `FileStore` passes its specific tests (atomic replace, lock contention, CRLF, comment and unknown-key preservation on update, path traversal); `functor init` works and creates the documented layout.

## M4: Artifact Management

CLI: `artifact create|show|list|update`, argument resolver ([identity-and-addressing.md](identity-and-addressing.md)), text and JSON rendering, error rendering and exit codes ([cli-spec.md](cli-spec.md)). Introduce the golden-file and JSON-schema test harness.

Done when: artifacts can be stored and retrieved through the CLI and through the library API; golden tests cover success and every error path of these commands. (Event emission is added in M8 once `functor-events` exists; handlers already call a `publish` seam backed by a no-op publisher until then.)

## M5: Relationship Management

CLI: `relation add|list|show`. Endpoint existence, duplicate and self-reference rules ([traits-and-interfaces.md](traits-and-interfaces.md), ADR-0007).

Done when: artifacts form a navigable graph on disk; `functor relation add rfc-0001 depends_on rfc-0000` works with slugs, ids and the kebab alias.

## M6: Query (`functor-query`)

RFC-0006 subset. Snapshot engine, `query artifacts|dependencies|dependents|relationships` ([query-model-mvp.md](query-model-mvp.md)).

Done when: table-driven and property tests pass for traversal on both stores, and users can discover and navigate repository content.

## M7: Validation (`functor-validation`)

RFC-0007 validation subset. The rule catalog `V0001` to `V0018`, report, exit-code policy, `validate [artifact] [--strict]` ([validation-rules.md](validation-rules.md)).

Done when: each rule has its own fixture repository and the clean fixture passes; output is byte-identical across runs.

## M8: Events (`functor-events`)

RFC-0017 subset ([event-model-mvp.md](event-model-mvp.md), ADR-0003). Event log, publisher, subscription, `functor watch`; replace the no-op publisher in the mutating commands and `validate`.

Done when: each mutating command emits exactly one event, sequence numbers are gap-free under concurrent writers and across rotation (10 MiB, 10 archives, [ADR-0008](adr/ADR-0008-event-log-rotation.md)), `watch` passes its tests (existing, new, truncated, rotation, filters) and a failed append only warns.

## M9: Hardening and MVP Exit

- Acceptance workflow on Windows (the blocking platform); Linux and macOS results are informational ([testing-strategy.md](testing-strategy.md)).
- `examples/getting-started/` fixture matching [getting-started.md](getting-started.md); JSON Schemas in `docs/schemas/`.
- Performance check against [nonfunctional-requirements.md](nonfunctional-requirements.md).
- [rfc-conformance.md](rfc-conformance.md) fully populated; README and docs index updated.

Done when: all exit criteria in [mvp-scope.md](mvp-scope.md) are met.

## Parallelization

After M3, one engineer can work on the artifact CLI (M4) while another builds query (M6) and validation (M7) against `MemoryStore` and fixtures; M8 only needs the publisher trait from M2 and the repository lock from M3.

## Deferred Beyond the MVP

```text
RFC-0009 Federation and Synchronization
RFC-0010 Capability Model
RFC-0012 Interoperability and Integration
RFC-0013 Security and Trust Model
RFC-0014 Extension Model
RFC-0016 Versioning Model
RFC-0018 Governance Model
Execution (RFC-0007) and other projections (RFC-0011)
```

They are implemented only after the core architecture proves itself; see [product-roadmap.md](product-roadmap.md).

## Decisions Taken on Earlier Open Questions

| Question | Decision | Record |
|----------|----------|--------|
| Event log rotation and size policy | Rotate at 10 MiB, keep 10 archives | [ADR-0008](adr/ADR-0008-event-log-rotation.md) |
| Extract `functor-repository`? | Yes, in the MVP | [ADR-0009](adr/ADR-0009-functor-repository-crate.md) |
| Dynamic dispatch for core traits? | Allowed; `async-trait` | [ADR-0010](adr/ADR-0010-async-trait-and-dyn.md) |
| Must hand-edited TOML comments survive updates? | Yes; `toml_edit` | [ADR-0011](adr/ADR-0011-toml-edit-preserving-updates.md) |
| Slug history after rename | None in the MVP; the old slug is released | [identity-and-addressing.md](identity-and-addressing.md) |
