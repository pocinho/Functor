# CLI MVP Roadmap

Status: Accepted
Related docs: [mvp-scope.md](mvp-scope.md), [workspace-layout.md](../architecture/workspace-layout.md), [testing-strategy.md](../quality/testing-strategy.md), [development-workflow.md](../quality/development-workflow.md), [product-roadmap.md](product-roadmap.md) (long term)

## Purpose

The ordered, milestone-level plan to build the CLI MVP. The goal is not to implement every RFC; it is to validate the core architectural assumptions of RFC-0000 to RFC-0018 through a working API-first implementation. Scope is in [mvp-scope.md](mvp-scope.md); this document says in what order and with what acceptance evidence.

## Guiding Rules

- **API first.** Each capability is implemented and tested through core traits before the CLI exposes it. The CLI never becomes the system of record.
- **Semantic first.** Follow the RFC-derived model in [domain-model.md](../design/domain-model.md).
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

- Convert the root `Cargo.toml` to a virtual workspace and create the empty crates ([workspace-layout.md](../architecture/workspace-layout.md), ADR-0004).
- `rust-toolchain.toml`, `rustfmt.toml`, workspace lints, `deny.toml`.
- CI workflow: fmt, clippy, test, deny on Windows (blocking), with Linux and macOS test jobs as non-blocking ([development-workflow.md](../quality/development-workflow.md)).
- Dependency-edge check script ([crate-dependencies.md](../architecture/crate-dependencies.md)).

Done when: CI is green on Windows with empty crates and `functor --version` runs.

Status: implemented. `scripts/ci.ps1` (fmt, clippy, dependency edges, tests, links), `cargo deny check` and `functor --version` pass locally on Windows; the first GitHub Actions run happens when the branch is pushed.

## M1: Semantic Model (`functor-model`)

RFC-0001, 0003, 0004, 0005. Identifiers, slug, artifact, relationship and type enums, snapshot with indexes, query, validation and event value types.

Done when: every invariant in [domain-model.md](../design/domain-model.md) has a unit or property test, and artifacts and relationships can be created entirely in memory.

Status: implemented. `functor-model` has all types from `domain-model.md` (see its "Implementation Notes"), 52 unit tests, 14 property tests and an in-memory end-to-end test. Search matching and traversal execution belong to M6 (`functor-query`).

## M2: Core Contracts and Testkit (`functor-core`, `functor-testkit`)

RFC-0002. Traits ([traits-and-interfaces.md](../architecture/traits-and-interfaces.md)), `FunctorError` ([error-handling.md](../architecture/error-handling.md)), `Clock`, `IdGenerator`, test doubles and the **store conformance suite** (written against the traits, initially without an implementation).

Done when: the traits compile with doc comments and the conformance suite compiles; the error-to-exit-code mapping function has its exhaustive test.

Status: implemented.
- `functor-core`: all service traits (dyn-compatible, with a compile-time test), `FunctorError` and `ErrorKind`, `LoadIssue` types with `SnapshotLoad::into_snapshot`, `SystemClock` and `UlidGenerator`.
- `functor-testkit`: `FixedClock`, `SequentialIds`, `RepoFixture` and the store conformance suite (14 scenarios), validated by a reference store that passes it and six deliberately broken stores that it rejects.
- `functor-cli`: `exit_code(ErrorKind)` with its table test.

Real stores (`MemoryStore`, `FileStore`) run the suite in M3.

## M3: Repository and Storage (`functor-repository`, `functor-store`)

RFC-0008, 0015 subset ([ADR-0009](../adr/ADR-0009-functor-repository-crate.md)).

1. `functor-repository`: layout and path helpers, `repository.toml`, `format_version` check, `init`, `open`, `discover`, the write lock ([repository-layout.md](../design/repository-layout.md), [repository-lifecycle.md](../design/repository-lifecycle.md)).
2. `functor-store`: `MemoryStore` first (passes the conformance suite), then `FileStore`: artifact and relationship persistence, strict and lenient loading, atomic writes and in-place `toml_edit` updates ([storage-design.md](../design/storage-design.md), [artifact-format.md](../design/artifact-format.md), [ADR-0011](../adr/ADR-0011-toml-edit-preserving-updates.md)).

CLI: `functor init`.

Done when: `functor-repository` passes its lifecycle and lock tests; both stores pass the conformance suite; `FileStore` passes its specific tests (atomic replace, lock contention, CRLF, comment and unknown-key preservation on update, path traversal); `functor init` works and creates the documented layout.

Status: implemented.
- `functor-repository` (44 tests): `Layout`, `repository.toml` codec, `atomic_write` (with failure injection before the rename), `WriteLock` (contention and timeout tests), `FileRepositoryManager` (`init` with failure cleanup, `open`, `discover`, `--nested` rule, format version check) and the shared `toml_support` helpers.
- `functor-store` (60 tests): `MemoryStore` and `FileStore` both pass the conformance suite; artifact and relationship codecs with canonical rendering and in-place edits; the lenient loader with every `LoadIssueKind`; 29 `FileStore`-specific scenarios. Notes in [storage-design.md](../design/storage-design.md#implementation-notes-m3).
- `functor-cli`: `functor init [PATH] [--name] [--description] [--nested]` with global `--repo` and `--output text|json`, error rendering in both modes and exit codes (12 end-to-end tests).

## M4: Artifact Management

CLI: `artifact create|show|list|update`, argument resolver ([identity-and-addressing.md](../design/identity-and-addressing.md)), text and JSON rendering, error rendering and exit codes ([cli-spec.md](../cli/cli-spec.md)). Introduce the golden-file and JSON-schema test harness.

Done when: artifacts can be stored and retrieved through the CLI and through the library API; golden tests cover success and every error path of these commands. (Event emission was added in M8, once `functor-events` existed; the handlers already called a recorder seam.)

Status: implemented. `artifact create|show|list|update`, the resolver (id, slug, canonical, with suggestions), `-v`/`-q`, text and JSON rendering, four JSON Schemas in `docs/cli/schemas/`, and a golden/schema/cross-process e2e harness (47 artifact tests, 28 CLI unit tests; see [testing-strategy.md](../quality/testing-strategy.md)). Golden tests need `--all-features`; CI and `scripts/ci.ps1` also run default features to prove release builds ignore the test-only variables. Color and `tracing` logs are not part of the MVP ([Decisions Closed Before M9](#decisions-closed-before-m9)). Event emission arrived in M8.

## M5: Relationship Management

CLI: `relation add|list|show`. Endpoint existence, duplicate and self-reference rules ([traits-and-interfaces.md](../architecture/traits-and-interfaces.md), ADR-0007).

Done when: artifacts form a navigable graph on disk; `functor relation add rfc-0001 depends_on rfc-0000` works with slugs, ids and the kebab alias.

Status: implemented. `relation add|list|show`, the relationship resolver (`rel_` id or canonical reference), the `relationship.created` event (appended since M8), schemas `relationship-document.json` and `relationship-list.json`, and 32 e2e tests (golden transcripts, schemas, filters, every error path, cross-process races) plus unit tests for the resolver, event builder and argument grammar. Cycles across relationships are not rejected at add time; `validate` reports them (M7). See [cli-spec.md](../cli/cli-spec.md#implementation-notes-m5).

## M6: Query (`functor-query`)

RFC-0006 subset. Snapshot engine, `query artifacts|dependencies|dependents|relationships` ([query-model-mvp.md](../design/query-model-mvp.md)).

Done when: table-driven and property tests pass for traversal on both stores, and users can discover and navigate repository content.

Status: implemented. `functor-query` has `search`, `traverse`, `relationships_of` and `SnapshotQueryEngine` (30 unit tests, 5 oracle-checked property tests, 5 engine tests that compare `MemoryStore` and `FileStore` and pin one snapshot per call); the CLI has `query artifacts|dependencies|dependents|relationships` with schemas `query-traversal.json` and `query-relationships.json` and 29 e2e tests (golden transcripts, schemas, filters, depth and type options, cycles, read-only check, error paths). See [query-model-mvp.md](../design/query-model-mvp.md#implementation-notes-m6) and [cli-spec.md](../cli/cli-spec.md#implementation-notes-m6).

## M7: Validation (`functor-validation`)

RFC-0007 validation subset. The rule catalog `V0001` to `V0019`, report, exit-code policy, `validate [artifact] [--strict]` ([validation-rules.md](../design/validation-rules.md)).

Done when: each rule has its own fixture repository and the clean fixture passes; output is byte-identical across runs.

Status: implemented. `functor-validation` has the 19-rule catalog (kept equal to [validation-rules.md](../design/validation-rules.md) by a test), pure rules, iterative Tarjan cycle detection, scoping and `RuleValidationEngine` (59 unit tests including a cycle property test, 27 real-file engine tests with one broken repository per rule code); the CLI has `validate [ARTIFACT] [--strict]`, the exit-code policy, schema `validate.json` and 28 e2e tests (golden transcripts, every outcome, scoping, damaged repositories, byte-identical output across clocks). One rule was added to the planned catalog: `V0019` (an unreadable path) makes the `Unknown` outcome explainable. Notes in [validation-rules.md](../design/validation-rules.md#implementation-notes-m7) and [cli-spec.md](../cli/cli-spec.md#implementation-notes-m7).

## M8: Events (`functor-events`)

RFC-0017 subset ([event-model-mvp.md](../design/event-model-mvp.md), ADR-0003). Event log, publisher, subscription, `functor watch`; replaces the no-op recorder of the mutating commands and `validate`.

Done when: each mutating command emits exactly one event, sequence numbers are gap-free under concurrent writers and across rotation (10 MiB, 10 archives, [ADR-0008](../adr/ADR-0008-event-log-rotation.md)), `watch` passes its tests (existing, new, truncated, rotation, filters) and a failed append only warns.

Status: implemented. `functor-events` has the wire format, `FileEventLog` (publisher and event source: gap-free numbering under the write lock, torn-line repair, rotation and pruning) and `Follower` (notifications with a polling fallback, archives, rotation, truncation and restarts); 63 tests (7 wire, 3 cursor, 28 log, 25 follower). The CLI appends the events of `artifact create|update`, `relation add` and `validate` through a recorder that only warns on failure, and has `functor watch` with all its options; 19 events e2e tests and 26 watch e2e tests (a real 10 MiB rotation, multi-process numbering, failed appends, live following across processes, damaged and pruned logs, a closed pipe, never writing). Dependency additions: `notify` (CC0-1.0, now allowed in `deny.toml`) and `tokio` features `time`, `sync` (events) and `time`, `signal` (CLI). Not covered by an automated test: Ctrl-C ending `watch`. Notes in [event-model-mvp.md](../design/event-model-mvp.md#implementation-notes-m8) and [cli-spec.md](../cli/cli-spec.md#implementation-notes-m8).

## M9: Hardening and MVP Exit

- Acceptance workflow on Windows (the blocking platform); Linux and macOS results are informational ([testing-strategy.md](../quality/testing-strategy.md)).
- `examples/getting-started/` fixture matching [getting-started.md](../cli/getting-started.md); JSON Schemas in `docs/cli/schemas/`.
- Performance check against [nonfunctional-requirements.md](../quality/nonfunctional-requirements.md).
- [rfc-conformance.md](rfc-conformance.md) fully populated; README and docs index updated.
- Decisions deferred from earlier milestones were reviewed and closed before this milestone ([Decisions Closed Before M9](#decisions-closed-before-m9)). Two of them name a trigger to revisit: the second snapshot read of traversals and `notify` for `watch`; check both against the performance results.
- Run the manual Ctrl-C check for `watch` ([testing-strategy.md](../quality/testing-strategy.md)).

Done when: all exit criteria in [mvp-scope.md](mvp-scope.md) are met.

Status: implemented. The manual Ctrl-C check for `watch` was run on Windows and passed.

- **Acceptance and example.** `src/app/cli/tests/acceptance.rs` (golden transcripts of the walkthrough, reproducibility across two sandboxes, the negative variant with `V0007`) and `src/app/cli/tests/library.rs` (the same workflow through the libraries only, cross-reads between the libraries and the CLI, a moved repository, repository metadata that changes no result). `examples/getting-started/` is checked byte for byte against what the walkthrough produces.
- **Conformance.** Every row of [rfc-conformance.md](rfc-conformance.md) has a status and every Implemented or Subset row names a test that exists; `scripts/check-conformance.ps1` enforces it (also for the evidence tables of [mvp-scope.md](mvp-scope.md) and [nonfunctional-requirements.md](../quality/nonfunctional-requirements.md)) in `scripts/ci.ps1` and CI. `scripts/check-deps.ps1` also fails on a network-capable crate in the shipped dependency tree.
- **Performance.** `scripts/bench.ps1` measured the first implementation at 0.56 s (`artifact show`) and 1.6 s (`relation add`) for 1,000 artifacts, over the 300 ms budget. The loader now reads and parses files in parallel (about 6 times faster) and the CLI reads the repository once per command (`CachingStore`). At 1,000 artifacts every budget is met (90 to 155 ms); at 10,000 artifacts a full load costs about 0.55 s, so the budgets for `artifact show` and `relation add` were revised to 0.75 s and 1.5 s and partial loading was deferred, to keep "a damaged repository is refused" for every read ([nonfunctional-requirements.md](../quality/nonfunctional-requirements.md#performance-budgets), [storage-design.md](../design/storage-design.md#implementation-notes-m9)). The baseline is `docs/quality/benchmarks/baseline.json`; CI runs the benchmark as an informational job.
- **Hardening found by the audit.** Event lines over 64 KiB are skipped with a warning (the baseline promised it, the follower did not do it yet). Found by the new wiring: `watch` must read the store directly, because a cache lives for one command and a follower resolves slugs for hours.
- **Decisions with a trigger.** The second snapshot read of traversals is gone (one read per command through the cache). `notify` for `watch` is unaffected by the performance results and stays.
- **Contracts.** The frozen contracts and the evidence for each exit criterion are in [mvp-scope.md](mvp-scope.md#exit-evidence).

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
| Event log rotation and size policy | Rotate at 10 MiB, keep 10 archives | [ADR-0008](../adr/ADR-0008-event-log-rotation.md) |
| Extract `functor-repository`? | Yes, in the MVP | [ADR-0009](../adr/ADR-0009-functor-repository-crate.md) |
| Dynamic dispatch for core traits? | Allowed; `async-trait` | [ADR-0010](../adr/ADR-0010-async-trait-and-dyn.md) |
| Must hand-edited TOML comments survive updates? | Yes; `toml_edit` | [ADR-0011](../adr/ADR-0011-toml-edit-preserving-updates.md) |
| Slug history after rename | None in the MVP; the old slug is released | [identity-and-addressing.md](../design/identity-and-addressing.md) |

## Decisions Closed Before M9

Reviewed with the project owner after M8. The choices that were not simply "keep as built":

| Question | Decision | Where it is recorded |
|----------|----------|----------------------|
| Color and `--no-color`, `tracing` logs, `-vv` | Not in the MVP. `-v` stays the error-cause-chain switch; output has no ANSI sequences. The spec, the dependency policy and the non-functional requirements no longer mention color, `NO_COLOR`, `RUST_LOG` or `tracing` | [cli-spec.md](../cli/cli-spec.md), [dependency-policy.md](../architecture/dependency-policy.md), [nonfunctional-requirements.md](../quality/nonfunctional-requirements.md), [error-handling.md](../architecture/error-handling.md) |
| Duplicate-relationship message | Keep the artifact ids (unambiguous and scriptable); no slug rewriting | [cli-spec.md](../cli/cli-spec.md#implementation-notes-m5) |
| `notify` for `watch` | Keep, with the polling fallback. Revisit only if the M9 performance check or a platform shows it to be a liability | [event-model-mvp.md](../design/event-model-mvp.md), [dependency-policy.md](../architecture/dependency-policy.md) |
| Ctrl-C ending `watch` | Manual check, no automated test | [testing-strategy.md](../quality/testing-strategy.md) |
| `-q` for `validate` and `watch` | Unchanged: `-q` silences only the confirmation of mutating commands in text mode; reports and streams are always printed | [cli-spec.md](../cli/cli-spec.md) |

Confirmed as built (no change): `artifact show --content` text and `content.text`; `--content-path` relative to the current directory; `functor-events` depending on `functor-repository` (ADR-0009); hand-written TOML codecs (ADR-0011); `FunctorError::Conflict` displaying only its detail; cycles not rejected by `relation add` (`validate` reports `V0011`); `query artifacts` printing what `artifact list` prints; two snapshot reads per traversal (one since M9, see below); string `--meta` matching and Unicode lowercasing in queries; the empty-traversal lines; rule `V0019`; `validate` finding the repository without parsing `repository.toml`; scoped `validate`; a duplicate id also raising `V0017`; plural counts in `validate` text; the idle `--timeout` of `watch`; the follower's handling of a restarted log; fixed event rotation limits; the runtime built with `enable_all`.
