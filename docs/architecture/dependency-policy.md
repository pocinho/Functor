# Dependency Policy

Status: Accepted
Related ADRs: ADR-0005, ADR-0009, ADR-0010, ADR-0011
Related docs: [crate-dependencies.md](crate-dependencies.md), [development-workflow.md](../quality/development-workflow.md)

## Purpose

Which third-party crates the MVP uses, where, and the rules for adding more. Versions are not pinned in this document: the root `Cargo.toml` `[workspace.dependencies]` is the single source of versions, and `Cargo.lock` is committed.

## Principles

- Prefer the standard library when the need is small.
- Prefer widely used, maintained, permissively licensed crates (MIT, Apache-2.0, BSD, ISC, Unicode, Zlib).
- Every dependency has an owner layer (see [crate-dependencies.md](crate-dependencies.md)). Lower layers stay dependency-light.
- No `unsafe` in Functor code (`unsafe_code = "forbid"` at the workspace level). Dependencies may use it.
- Disable default features and enable only what is needed.

## Approved Crates

| Purpose | Crate | Layers |
|---------|-------|--------|
| Id generation and parsing | `ulid` | model |
| Error types | `thiserror` | model, core and other libraries |
| TOML read, create and in-place edit | `toml_edit` (document API, no serde) | repository, store, runtime (read-only, workflow format) |
| Serialization framework | `serde` (derive) | events, cli (JSON output) |
| JSON | `serde_json` | events, cli, store (evidence documents, [ADR-0016](../adr/ADR-0016-evidence-files-and-repair.md)) |
| SHA-256 | `sha2` (RustCrypto; MIT or Apache-2.0; pure Rust with CPU-feature detection through `cpufeatures`) | store, runtime, cli (reviewed workflow and report-subject digests) |
| Process-tree termination | `process-wrap` (Tokio wrappers; features `tokio1`, `kill-on-drop`, `job-object`, `process-group`; see the vetting record below) | runtime |
| Async trait support | `async-trait` | core and the crates implementing core traits |
| Async runtime | `tokio` (features: `rt`; later `time`, `signal`, `io-util`, `sync` where needed; `macros` in dev-dependencies for `#[tokio::test]`) | repository, store, events, cli |
| File locking | the standard library (`File::try_lock`, stable since Rust 1.89); no crate | repository |
| Filesystem notifications | `notify` | events |
| CLI parsing | `clap` (derive, `env`) | cli |
| CLI error glue | none: every error is a `FunctorError` or a `clap` usage error (`anyhow` was considered and not needed) | |
| Logging and terminal color | none in the MVP (`tracing`, `tracing-subscriber` and `anstream` were considered and deferred) | |
| Table rendering | a small in-house helper (two-space separated columns); no table crate | cli |
| Test | `proptest`, `insta`, `tempfile`, `assert_cmd`, `predicates`, `jsonschema` | dev only |

Notes:

- TOML: `toml_edit` is used for both reading and writing so that updates preserve comments and formatting ([ADR-0011](../adr/ADR-0011-toml-edit-preserving-updates.md)). The plain `toml` crate is not used. Files are mapped to and from model types by hand from the `toml_edit` document (not through `serde` derives), because the loader must classify every kind of problem separately (`LoadIssueKind`) and keep unknown keys; `serde` therefore appears only where JSON is produced.
- File locking: the write lock uses `std::fs::File::try_lock` with a bounded poll, so no locking crate is needed.
- Blocking filesystem work runs through `tokio::task::spawn_blocking` (`functor_repository::run_blocking`), so `tokio` needs only the `rt` feature, not `fs`.
- Async traits: `async-trait` on the core traits so they are dyn-compatible ([ADR-0010](../adr/ADR-0010-async-trait-and-dyn.md), which supersedes ADR-0002). This is the approved exception to the rule that lower layers need an ADR for new dependencies.
- `jsonschema` validates CLI JSON against `docs/cli/schemas/*.json` in tests.
- `notify` (8.x) is licensed **CC0-1.0**, a public-domain dedication, so `deny.toml` lists `CC0-1.0` explicitly. `watch` still works if notifications are unavailable: the watcher polls every 500 ms without them and every 5 s as a safety net with them ([event-model-mvp.md](../design/event-model-mvp.md)). `tokio` gains `time` and `sync` in `functor-events` and `time` and `signal` in `functor-cli`; `select!` (and so `macros`) is not used outside tests.

## Vetting Record: `process-wrap` (ADR-0012)

Recorded before P1.3 as ADR-0012 requires.

| Check | Result |
|-------|--------|
| Problem | Kill a child and everything it started on timeout and cancellation. The standard library can only kill the direct child; Windows needs a Job Object, and Functor code forbids `unsafe` |
| Alternatives | `win32job` (a bare Job Object wrapper: the child is assigned after it started, so it can start grandchildren first); `command-group` (the older, unmaintained predecessor of `process-wrap`); writing the Windows API calls ourselves (needs `unsafe`, refused) |
| Why this one | `process-wrap` spawns the child suspended, assigns it to the job and then resumes it, so nothing escapes the job; it also provides process groups on Unix |
| License | `Apache-2.0 OR MIT` (`cargo deny check licenses` passes) |
| Maintenance | `watchexec` project; about 19 million downloads; version 10.0.1, last updated September 2026; not yanked |
| Advisories | No RustSec entry for the crate (checked by hand against the advisory database); `cargo deny check advisories` could not fetch the database on the development machine, so it must pass in CI |
| Footprint | Default features are disabled. P1.6 uses `tokio1`, `kill-on-drop`, `job-object` and `process-group`: `indexmap`, the `windows` crates (Windows only), existing `tokio` with `process`/`io-util`/`time`, and the `futures` family. `cargo tree -p functor-runtime -e normal --depth 2` records the direct footprint; no new workspace dependency or network adapter |
| Caveat | The wrapper's `wait` and `try_wait` both read the job's completion port, so using them together can block forever. `functor-runtime` polls and reaps the leader process directly and uses the job only to terminate (found by test; see the P1.3 notes in [execution-and-evidence.md](../design/execution-and-evidence.md)) |
| P1.6 correction | The std Job Object wrapper passes `kill_on_drop = false`; normal cancellation tests did not prove owner-death cleanup. The Tokio wrapper with `KillOnDrop` enables `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` before resuming the child. The Windows CLI owner-kill test proves descendants stop and the lease is recoverable. No `unsafe` was added to Functor |
| P1.6 recheck | `cargo deny check` passed advisories, licenses, bans and sources on 2026-10-14, including the new transitive footprint; the earlier advisory-database fetch limitation did not recur |

## Adding a Dependency

A pull request that adds or upgrades a dependency must state:

1. What problem it solves and why `std` or existing crates are insufficient.
2. Which crate and layer will use it (and that this is allowed by [crate-dependencies.md](crate-dependencies.md)).
3. License and maintenance status (last release, open advisories).
4. Transitive footprint (`cargo tree -p <crate> -e normal`).

Dependencies for lower layers (`functor-model`, `functor-core`, `functor-query`, `functor-validation`) need an ADR.

## Automated Checks

- `cargo deny check` (licenses, bans, advisories, sources) in CI.
- `cargo audit` (or the advisories part of `cargo deny`) on a schedule.
- `cargo machete` or equivalent to find unused dependencies (advisory in CI).
- Dependabot or Renovate for updates, grouped weekly.

## Toolchain

- Edition 2024 (already set).
- A pinned stable toolchain in `rust-toolchain.toml` with `rustfmt` and `clippy` components.
- MSRV equals the pinned toolchain until the first release; then MSRV is declared as `workspace.package.rust-version` and tested in CI.
