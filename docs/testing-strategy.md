# Testing Strategy

Status: Draft
Related RFCs: all implemented RFCs (via [rfc-conformance.md](rfc-conformance.md))
Related docs: [development-workflow.md](development-workflow.md), [traits-and-interfaces.md](traits-and-interfaces.md), [validation-rules.md](validation-rules.md), [cli-spec.md](cli-spec.md)

## Purpose

Defines how the Functor CLI MVP is tested, what must be covered at each level, and how tests prove RFC compliance.

## Principles

- Test core semantics first, at the lowest layer that can express them.
- Deterministic tests only: injected clock and id generator, no sleeping on wall-clock time, no dependence on directory order, no network.
- Tests are independent and run in parallel; each uses its own temporary directory.
- **Traceability:** every "Implemented" and "Subset" row in [rfc-conformance.md](rfc-conformance.md) names at least one test. A requirement without a test is not done.
- Test the contract, not the implementation: the same suites run against every implementation of a trait.

## Test Pyramid

| Level | Where | Purpose |
|-------|-------|---------|
| Unit | `#[cfg(test)]` modules in each crate | Invariants and pure logic |
| Property | `proptest` in model, store, query | Round trips, determinism, termination |
| Conformance | `functor-testkit`, run from `functor-store` | Same behavior across `MemoryStore` and `FileStore` |
| Integration (library) | `tests/` of services, using the traits only | Whole workflows without the CLI |
| CLI end-to-end | `src/app/cli/tests/` with `assert_cmd` | Arguments, output, exit codes, files on disk |
| Golden and schema | `insta` snapshots, JSON Schema validation | Output contracts |
| Acceptance | one end-to-end test | The MVP success path |
| Benchmarks | scheduled CI job | [nonfunctional-requirements.md](nonfunctional-requirements.md) budgets |

## Unit and Property Tests by Crate

### functor-model

Every invariant in the table at the end of [domain-model.md](domain-model.md): id round trips, slug grammar tables, relationship type parsing (including the kebab alias and the built-in-never-custom rule), `Revision` non-zero, tag normalization, `ContentPath` rejection cases, outcome derivation.

### functor-core

Error variant to exit code and JSON code mapping (exhaustive), `FixedClock` and `SequentialIds` behavior (in testkit), and a compile-time check that every core trait is object-safe (`&dyn Trait` coerces, [ADR-0010](adr/ADR-0010-async-trait-and-dyn.md)).

### functor-repository

`init` (layout created, `repository.toml` written last, failure leaves no recognizable repository, existing repository and `--nested` rules), `open`, `discover` from nested directories, `format_version` check (`UnsupportedFormat`), `Locked` on lock timeout and lock contention across two processes.

### functor-store

- Canonical serialization of **new** files: `serialize(parse(serialize(x))) == serialize(x)` (property), golden TOML files for artifact, relationship and repository.
- In-place updates ([ADR-0011](adr/ADR-0011-toml-edit-preserving-updates.md)): comments, key order, whitespace, line endings and unknown keys and tables survive an update; a new optional key is inserted at its canonical position; removing a value removes only its attached comments; a no-op update writes nothing; an update of a hand-edited, non-canonical file changes only the affected lines (diff-based assertion).
- `FileStore` specifics: atomic replace with failure injected before the rename, stray `.tmp-` files ignored, read while write, CRLF input, size caps, path traversal in `content.path`, lenient `load()` reporting each `LoadIssue` kind.

### functor-query

Table-driven traversal tests (chain, diamond, cycle, disconnected, depth 0, 1, n and unlimited, each direction, type filters), search filter combinations and ordering, property: traversal terminates and reports each node once on arbitrary generated graphs.

### functor-validation

One minimal fixture repository **per rule code** (`V0001` to `V0019`) that triggers exactly that rule, plus a clean repository with zero findings; outcome and exit-code tables; byte-identical report across runs; cycle cases.

### functor-events

Envelope round trip and unknown-key tolerance; `seq` strictly increasing and gap-free under concurrent writers (multi-process) and across rotation; rotation at 10 MiB with exactly 10 archives retained and the oldest pruned ([ADR-0008](adr/ADR-0008-event-log-rotation.md)); truncated tail handling; watcher behavior (existing events, new events, log created later, `--after`, truncation, rotation while watching, filters), using `count` and `timeout` instead of sleeps where possible.

The crate tests use small rotation limits (`Limits`) so rotation needs a few lines, not 10 MiB; the CLI tests seed a real 10 MiB log once. Watcher tests run both with filesystem notifications and with polling only (`FollowOptions`). Two behaviors are not covered by automated tests: Ctrl-C ending `watch` (the race with the signal future is unit-tested, the signal itself is checked by hand) and the notification backends of other platforms.

## Conformance Suite

`functor-testkit` exports `run_store_conformance(factory)` and one public `async fn` per scenario. The factory receives an `Env` (a `FixedClock` and `SequentialIds` the store must use for time and ids) and returns a `Fresh<S>` (an empty store plus an optional guard such as a temporary directory). The suite is runtime-agnostic: it returns futures, and the calling test drives them (`#[tokio::test]` in `functor-store`). Example:

```rust
run_store_conformance(|env| Fresh::new(MemoryStore::new(env.clock(), env.ids()))).await;
```

It covers `RepositoryReader`, `ArtifactRepository` and `RelationshipRepository` with these scenarios, each against a fresh store:

- create, get, resolve by id and slug, list ordering (`create_get_resolve_and_list`)
- duplicate slug conflict, slug rename releasing the old slug, rename to a taken slug, revision increment, no-op update writes nothing, stale expected revision conflict, update of an unknown artifact
- relationship endpoint existence, self-reference rejection (`references` may self-reference), duplicate triple conflict, direction and types, list order and timestamps
- strict snapshot agrees with individual operations; lenient load of a clean repository has no issues; `info` is stable
- determinism: identical operation sequences with fixed clock and ids give identical state

Strict-versus-lenient behavior on **damaged** data needs store-specific injection and is tested in each store's own tests, not in the shared suite.

`MemoryStore` and `FileStore` both run the suite (`RepositoryManager` is tested in `functor-repository`). A future store must pass it to be accepted. This is what makes the storage independence of RFC-0008 and the projection independence of RFC-0002 enforceable rather than aspirational.

**The suite is itself tested.** `functor-testkit` contains a small reference store that must pass, plus deliberately broken variants (duplicate slugs allowed, expected revision ignored, endpoint check skipped, duplicate relationships allowed, list in reverse, revision bumped on a no-op) that the suite must reject. A suite that cannot fail would prove nothing.

## Fixtures

- Builders in `functor-testkit` create repositories programmatically (`RepoFixture::new().artifact("rfc-0000").artifact("rfc-0001").relation("rfc-0001", "depends_on", "rfc-0000").build()` returns a strict `RepositorySnapshot`).
- Static fixture repositories live in `examples/` and `tests/fixtures/`: `clean/`, `one-per-rule/V00NN/`, `getting-started/`. They are real repositories on disk, created once by tests using the library and committed; a test verifies the checked-in fixtures are still produced by the current serializer (prevents silent drift).
- Fixtures use fixed ids and timestamps.

## CLI End-to-End Tests

Run the built binary with `assert_cmd` in a temporary directory:

- Every command, in `text` and `json` modes.
- Every exit code in [error-handling.md](error-handling.md) from a real scenario.
- Repository discovery from a nested directory, `--repo`, `FUNCTOR_REPO`.
- Stdout vs stderr separation, JSON errors on stderr.
- Closed-pipe behavior for `watch` and large listings.
- Test-only determinism variables (`FUNCTOR_TEST_FIXED_TIME`, `FUNCTOR_TEST_ID_SEED`) under the `test-support` feature.

**Manual check (not automated): Ctrl-C ends `watch`.** The race between the watch loop and the signal is unit-tested, but the signal itself is checked by hand before a release on Windows: run `functor watch` in a repository, press Ctrl-C, and confirm that it exits with code 0 and prints nothing more (`echo $LASTEXITCODE`). Also run `functor watch --from-start | Select-Object -First 1` and confirm it returns quietly.

## Golden Files and Schemas

- Text output goldens for the examples in [cli-spec.md](cli-spec.md); `insta` snapshots stored in repository.
- JSON output of every command validated against `docs/schemas/*.json`, and additionally snapshotted.
- Help text snapshots.
- Updating goldens is a deliberate review step ([development-workflow.md](development-workflow.md)).

### How the harness works (M4)

- **Determinism.** Golden tests run the real binary built with the `test-support` feature, which honors `FUNCTOR_TEST_FIXED_TIME` (the clock stops at that instant) and `FUNCTOR_TEST_ID_SEED` (ids count up from the seed within one process). Each command is a separate process, so every invocation in a test gets its own seed; the sandbox initializes its repository with seed 1 and tests use distinct seeds from 100 upwards. The golden test targets declare `required-features = ["test-support"]`, so a plain `cargo test` skips them; use `--all-features` (CI and `scripts/ci.ps1` do).
- **Sandbox.** `tests/common/mod.rs` provides `Sandbox` (a temporary repository), `run`/`run_at` (deterministic invocations returning exit code, stdout and stderr), `assert_schema` and `assert_schema_rejects`.
- **Transcripts.** A snapshot is `exit code`, `stdout`, `stderr` of a command, so a golden pins the whole observable contract (including that failures print nothing on stdout). Snapshots live in `tests/snapshots/`.
- **Creating and updating goldens.** `INSTA_UPDATE=always cargo test -p functor-cli --all-features --test artifact` (or `--test relation`) writes snapshots; review the diff before committing. Without that variable (and always in CI) a missing or different snapshot fails the test. Each command family has its own test target (`artifact`, `relation`, ...) declared with `required-features = ["test-support"]` in `src/app/cli/Cargo.toml`; a new family needs a new `[[test]]` entry.
- **Schemas.** Every JSON document a command prints is validated with the `jsonschema` crate against `docs/schemas/<name>.json`. A test proves the schemas can fail (`assert_schema_rejects`), and another keeps the duplicated `Artifact` definition in `artifact-document.json` and `artifact-list.json` identical.
- **Cross-process behavior.** Concurrent CLI processes creating different slugs lose nothing, and racing processes creating one slug have exactly one winner (exit 0) with the rest exiting 4.

## Acceptance Test

Implemented in `src/app/cli/tests/acceptance.rs`. It runs the full MVP success path of [getting-started.md](getting-started.md) on a clean temporary directory with fixed time and ids, and compares the transcript (exit code, stdout, stderr) of every step with a golden snapshot:

```bash
functor init --name "Functor Demo"
functor artifact create rfc-0000 --type rfc --title "Functor Vision"
functor artifact create rfc-0001 --type rfc --title "Core Semantic Model"
functor relation add rfc-0001 depends_on rfc-0000
functor query dependencies rfc-0001        # also as JSON, checked against its schema
functor query dependents rfc-0000
functor validate
functor watch --from-start --count 4
```

Further checks in the same file:

- **Reproducible.** The walkthrough runs in two sandboxes; the transcripts and every file of the two repositories are identical.
- **Reference repository.** `examples/getting-started/` is byte-for-byte what the walkthrough produces (line endings normalized, the lock file ignored), so the example and [getting-started.md](getting-started.md) cannot drift. `FUNCTOR_UPDATE_EXAMPLE=1` regenerates it for review. The example must also be a healthy repository the CLI can read.
- **Negative variant.** Deleting `artifacts/<rfc-0000 id>.toml` makes `functor validate` exit 6 with finding `V0007` and a document that matches the schema.

`src/app/cli/tests/library.rs` runs the same scenario through the library API only (no CLI), which proves the CLI is a pure projection (exit criterion 6 in [mvp-scope.md](mvp-scope.md)). It also checks that a repository written by the libraries is read by the CLI and the other way round (the reference repository), that moving a repository directory changes no identity, and that editing repository metadata changes no result.

## Failure Injection

Store functions accept a test hook (compiled under `cfg(test)` or a `test-hooks` feature of `functor-store`) to fail at defined points: before rename, after rename, during event append. Tests assert the crash table in [storage-design.md](storage-design.md).

## Cross-Platform

CI runs the full suite on Windows, which is the supported and blocking platform. Linux and macOS jobs run the same suite as best-effort and non-blocking checks ([nonfunctional-requirements.md](nonfunctional-requirements.md)); their failures are triaged only when someone maintains that port. Known differences to keep covered where possible: path separators in outputs, CRLF checkouts, file replacement while a handle is open (Windows), case-insensitive filesystems, notification vs polling in `watch`.

## Coverage

Coverage is measured with `cargo llvm-cov` and reported, not gated by a percentage, except these hard requirements:

- every validation rule code has a triggering and a non-triggering test;
- every `FunctorError` variant is produced by at least one test and mapped to an exit code;
- every CLI command has a success test and at least one failure test.

## Continuous Integration

Runs formatting checks, clippy with warnings denied, unit, integration, conformance, golden, acceptance and doc tests on Windows (required) and on Linux and macOS (non-blocking), plus dependency checks. See [development-workflow.md](development-workflow.md).

## Test Data Hygiene

No test touches the real user home or the repository working tree: always use `tempfile::tempdir()`. Tests never depend on the order of `read_dir`; any listing comparison sorts explicitly.
