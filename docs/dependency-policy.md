# Dependency Policy

Status: Draft
Related ADRs: ADR-0005, ADR-0009, ADR-0010, ADR-0011
Related docs: [crate-dependencies.md](crate-dependencies.md), [development-workflow.md](development-workflow.md)

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
| TOML read, create and in-place edit | `toml_edit` (with its `serde` feature for reading) | repository, store |
| Serialization framework | `serde` (derive) | repository, store, events, cli |
| JSON | `serde_json` | events, cli |
| Async trait support | `async-trait` | core and the crates implementing core traits |
| Async runtime | `tokio` (features: `rt`, `fs`, `sync`, `time`, `macros`, `signal`, `io-util`) | repository, store, events, cli |
| File locking | `fd-lock` | repository |
| Filesystem notifications | `notify` | events |
| CLI parsing | `clap` (derive) | cli |
| CLI error glue | `anyhow` | cli only |
| Logging | `tracing`, `tracing-subscriber` | cli (subscriber), libraries (`tracing` macros only) |
| Terminal color | `anstream` / `anstyle` (or `owo-colors`) honoring `NO_COLOR` | cli |
| Table rendering | a small in-house helper (two-space separated columns); no table crate | cli |
| Test | `proptest`, `insta`, `tempfile`, `assert_cmd`, `predicates`, `jsonschema` | dev only |

Notes:

- TOML: `toml_edit` is used for both reading and writing so that updates preserve comments and formatting ([ADR-0011](adr/ADR-0011-toml-edit-preserving-updates.md)). The plain `toml` crate is not used.
- Async traits: `async-trait` on the core traits so they are dyn-compatible ([ADR-0010](adr/ADR-0010-async-trait-and-dyn.md), which supersedes ADR-0002). This is the approved exception to the rule that lower layers need an ADR for new dependencies.
- `jsonschema` validates CLI JSON against `docs/schemas/*.json` in tests.

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
