# Development Workflow

Status: Accepted
Related docs: [dependency-policy.md](../architecture/dependency-policy.md), [testing-strategy.md](testing-strategy.md), [workspace-layout.md](../architecture/workspace-layout.md), [../CONTRIBUTING.md](../../CONTRIBUTING.md)

## Purpose

How code is built, checked and merged for the Rust implementation. These are the mechanics behind the "Continuous Integration" and "Definition of Done" requirements. [../CONTRIBUTING.md](../../CONTRIBUTING.md) remains the general contribution guide; this document adds Rust-specific rules.

## Toolchain

- `rust-toolchain.toml` pins a stable toolchain with the `rustfmt` and `clippy` components. Updating it is a normal pull request.
- Edition 2024; resolver 3 (ADR-0004).
- Build output goes to `out/cargo` (`.cargo/config.toml`, already configured and git-ignored via `out/`).

- **Scratch directory.** Tests that start real processes (`functor-runtime`) create their repositories under `tests/scratch` at the repository root instead of the system temp directory, so a failed run can be inspected and the folder can be excluded from virus scanning (a freshly built program can take many seconds to launch the first time it is scanned). `tests/*` is git-ignored, each test removes its own directory, `FUNCTOR_TEST_DIR` overrides the location, and `scripts/ci.ps1` removes `tests/` when it finishes.

## Standard Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --workspace --doc
cargo deny check
cargo run -p functor-cli -- <args>
```

`scripts/ci.ps1` runs the blocking checks locally (fmt, clippy, dependency edges, RFC conformance matrix, tests, markdown links). `scripts/check-deps.ps1`, `scripts/check-conformance.ps1` and `scripts/check-docs-links.ps1` can be run on their own; `scripts/bench.ps1` measures the performance budgets (see [nonfunctional-requirements.md](nonfunctional-requirements.md)) and is not part of `ci.ps1`.

## Workspace Lints

`[workspace.lints]` in the root manifest, inherited by every crate:

```text
rust:   unsafe_code = "forbid", missing_docs = "warn" (library crates), unused_must_use = "deny"
clippy: all = "warn", pedantic = "warn" (selected lints allowed with a reason),
        unwrap_used = "warn", expect_used = "warn", dbg_macro = "deny", print_stdout/print_stderr = "deny"
        in library crates (the CLI crate is the only crate that may print)
```

CI runs clippy with `-D warnings`, so warnings are failures. `clippy.toml` allows `unwrap` and `expect` inside tests only. A lint is allowed locally with `#[allow(...)]` plus a comment giving the reason; the workspace does not blanket-allow any pedantic lint.

## Continuous Integration

A GitHub Actions workflow (`.github/workflows/ci.yml`) on pull requests and pushes to `develop` and `main`:

| Job | Runs on | Steps |
|-----|---------|-------|
| `check` | Windows | `cargo fmt --check`, `cargo clippy -D warnings`, dependency-edge check (including the no-network-crate guard), [rfc-conformance.md](../planning/rfc-conformance.md) check |
| `test` | Windows (required) | `cargo test --workspace --all-features` (includes acceptance and golden tests; `--all-features` enables `functor-cli`'s `test-support`, which the golden tests require) |
| `test-other` | Linux, macOS (`continue-on-error: true`, non-blocking) | same command; failures are informational |
| `deny` | Windows | `cargo deny check` |
| `docs` | Windows | `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS=-D warnings`; markdown link check over `docs/` |
| `bench` (non-gating) | Windows | `scripts/bench.ps1` at 10,000 artifacts against `docs/quality/benchmarks/baseline.json`: reports the budgets of [nonfunctional-requirements.md](nonfunctional-requirements.md); hosted runners differ from the baseline machine, so the job never blocks |

Use `Swatinem/rust-cache` or an equivalent cache keyed on `Cargo.lock`. A pull request is mergeable only when `check`, `test` and `deny` (all on Windows) are green. `test-other` never blocks a merge (see the platform policy in [nonfunctional-requirements.md](nonfunctional-requirements.md)).

## Branching and Commits

- Work happens on `pp/<topic>` or feature branches and is merged into `develop` through pull requests (see the repository history).
- Conventional commits, matching the existing history (`docs:`, `feat:`, `fix:`, `test:`, `refactor:`, `chore:`, `ci:`), with the crate as an optional scope: `feat(store): add atomic write helper`.
- One logical change per pull request. Mixed refactors and behavior changes are split.

## Definition of Done (per pull request)

- [ ] Traces to an RFC row in [rfc-conformance.md](../planning/rfc-conformance.md) or an ADR. New decisions have an ADR.
- [ ] Behavior matches the relevant contract document; if the contract needed to change, the doc is updated **in the same pull request**.
- [ ] Tests exist for new behavior at the right level ([testing-strategy.md](testing-strategy.md)), including error paths.
- [ ] Public items are documented (purpose, inputs, outputs, errors, guarantees).
- [ ] No new dependency without the checklist in [dependency-policy.md](../architecture/dependency-policy.md).
- [ ] The dependency-edge check passes; no library crate prints or reads the environment.
- [ ] Golden files and JSON schemas updated deliberately (reviewed as part of the diff).
- [ ] `cargo fmt`, `clippy -D warnings`, `cargo test --workspace --all-features` pass locally (`scripts/ci.ps1` runs them).

## Pull Request Description

Use the repository pull request template (`.github/pull_request_template.md`). Add:

- **RFC / ADR:** the conformance rows or decisions touched.
- **Validation evidence:** the exact commands run and their results.

## Code Review Focus

1. Architecture: layer boundaries, composition root, no domain logic in the CLI.
2. Contracts: ordering, determinism, error variants, exit codes.
3. Safety: path handling, atomicity, lock usage, cancellation behavior.
4. Tests: failure injection, platform differences.

## Golden and Snapshot Files

- Located next to the tests that use them (`tests/snapshots/`).
- Updated only with the explicit `INSTA_UPDATE=always` (or `cargo insta review`) workflow, and the diff reviewed. Details in [testing-strategy.md](testing-strategy.md#how-the-harness-works-m4).
- Test-only environment variables for fixed time and ids exist only under the `test-support` feature ([cli-spec.md](../cli/cli-spec.md)), which is off by default and never enabled in release builds (the golden test targets declare `required-features`, so only `--all-features` test runs compile it in). A release build with default features does not recognize the variables.

## Releases

The completed Phase 0 CLI baseline and subsequent working-prototype milestones are not product releases. The first product release is eligible only after all Phases 0-7 are implemented and the [product-readiness gate](../planning/product-roadmap.md#product-readiness-gate) passes. Phase completion does not automatically authorize publication.

Every crate remains `publish = false` (set workspace-wide, see [workspace-layout.md](../architecture/workspace-layout.md)); enabling publication is a separate deliberate decision that starts with an ADR. After release authorization, define a tag-based release workflow, `cargo-dist` or equivalent for binaries, and a changelog from conventional commits.
