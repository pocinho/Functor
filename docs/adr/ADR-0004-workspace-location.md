# ADR-0004: Workspace location and root manifest

## Status

Accepted

## Context

Earlier docs described `crates/functor-*` at the repository root, while the repository skeleton contains `src/crates/`, `src/app/cli/` and `src/app/desktop/`. The root `Cargo.toml` is currently a single `[package]` (`functor`, edition 2024) with no `[workspace]`. `.cargo/config.toml` sends build output to `out/cargo`.

## Decision

- The root `Cargo.toml` becomes a **virtual workspace** (no root package) with `resolver = "3"`.
- Library crates live in `src/crates/functor-*`.
- Applications live in `src/app/*`. The CLI package is `functor-cli` at `src/app/cli` and builds a binary named `functor`.
- `src/app/desktop` is reserved for a future projection and is **not** a workspace member in the MVP.
- Shared dependency versions go in `[workspace.dependencies]` and shared lints in `[workspace.lints]`.
- `examples/` (sample repositories used as fixtures and documentation) sits at the repository root. There is no `roadmaps/` directory; roadmaps live in `docs/`.

## Consequences

- The existing single-package root must be removed or moved when the workspace is created.
- `cargo run -p functor-cli` and `cargo test --workspace` are the standard commands.

## RFC Impact

None. RFC-0011 only requires projections to be separable from the core.
