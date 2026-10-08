# Rust Development Practices

This chapter defines project-specific Rust practices for the active Functor
workspace. It supplements Rust language documentation with the boundaries and
failure modes that matter in this application; it is not a general Rust
tutorial.

The active Rust workspace uses edition 2024. The canonical architecture is
described in [System Architecture](system.md) and
[Project Identity and Status](project.md).

## Crate responsibilities and dependency direction

Keep responsibilities aligned with the active crates:

- **`functor_core`** owns domain state, invariants, semantic messages,
  deterministic updates, and pure tests. It must not depend on Tauri, the DOM,
  Monaco, or native filesystem APIs.
- **`functor`** owns Tauri command registration, session composition, bridge
  DTOs, native effects, and filesystem operations. It maps validated requests
  and effect results to and from the core.
- **`functor_web`** is the React/TypeScript frontend loaded in Tauri's WebView.
  It owns application-level UI state, presentation, and Monaco integration;
  it is not, by itself, a separately supported browser target.
- **`functor_ui`** is Functor's canonical React widget library. It owns shared,
  reusable UI components used to build the frontend.
- Frontend types are explicit projections of the bridge contract, not a
  serialization of internal Rust messages.

Keep these boundaries narrow. Add a module or crate when it establishes a
clear ownership, dependency, or testing boundary—not just to mirror a
potential future architecture. Prefer cohesive modules, focused private
implementation, and small public APIs over broad facades and speculative
abstractions.

## State, ownership, and invariants

- Keep each invariant with the state that owns it. `functor_core` validates
  document identity, revisions, text, cursor/selection positions, and
  workspace transitions. The host owns filesystem canonicalization and
  containment checks. The frontend owns DOM and Monaco objects.
- Make API ownership clear. Borrow data that is only observed; transfer
  ownership when a transition consumes or replaces state. Cloning is
  appropriate when it clarifies an ownership boundary or avoids disproportionate
  complexity, but do not clone defensively by default.
- Mutation of owned local state is idiomatic. Preserve deterministic behavior
  at the model/update boundary rather than requiring every implementation
  detail to be immutable.
- Do not make `Arc<Mutex<T>>` or other shared mutable state the default.
  Prefer ownership, messages, and immutable snapshots. If synchronization is
  needed, keep its scope explicit and do not hold session locks across blocking
  filesystem work or other long-running effects.
- Treat revisions and stable identities as contracts, not UI conveniences.
  Async results must identify the request or document they belong to and must
  be checked against the current model before being applied.

## MVU and effects

The core update function computes a transition from the current model and a
semantic message. It may mutate its owned model internally, but it must not
perform I/O, read clocks or global state, start processes, or call platform
APIs. The same model and message should produce the same model changes and
declared commands.

Represent effects as explicit core commands. The host interprets them, reports
success or failure, and sends a semantic result message through the update
path. Background work must not mutate the model directly or retain hidden
access to session state. Keep platform event types and Tauri command details
out of core messages.

## Typed boundaries and validation

Treat every frontend command and external effect result as untrusted input.
Use narrow, typed DTOs for bridge requests, snapshots, and structured errors;
do not expose the internal `Message` enum as a generic IPC API.

Validate data at the boundary where it enters a subsystem, then enforce
domain invariants in the owning core operation. In particular:

- Convert Monaco's UTF-16 positions explicitly and reject invalid bounds or
  surrogate boundaries before applying Rust-side edits.
- Check revision and identity on edit and asynchronous completion messages so
  stale or replayed work cannot silently replace newer state.
- Resolve workspace-relative paths in the host and verify that canonical
  paths remain inside the active workspace. Do not grant the WebView
  unrestricted filesystem access.
- Return stable, intentional error shapes across the bridge; do not leak
  internal implementation details as part of a public contract.

See [Document and Editor Workflow](document-workflow.md) for the current
document and filesystem boundary details.

## Errors and failure behavior

Use `Result` for failures that the application can recover from or report,
including file access, invalid requests, workspace discovery, and effect
execution. Add operation context when crossing a boundary, and preserve useful
causes for diagnostics.

Do not hide runtime failures with `unwrap`, `expect`, broad error catches, or
success-shaped fallback values. Surface failures through the established
error/logging or user-notification path. Reserve panics for programmer errors
or invariants that truly make continued execution impossible; make such
assumptions explicit and give failures useful context. Avoid exposing raw
internal errors or sensitive paths to the frontend.

## Async and concurrency

Keep state transitions synchronous and small. Run blocking or long-running
work outside the session lock, then return results as messages. When adding an
async effect, define how its request identity, completion, failure, and—where
needed—cancellation and shutdown are represented. Do not introduce shared
mutable state merely to make an async callback convenient.

## Testing

Test behavior at the narrowest boundary that owns it:

- In `functor_core`, cover invariants, message-to-transition behavior, command
  generation, and invalid or stale results without launching Tauri.
- In `functor`, cover DTO validation and mapping, session/effect behavior,
  filesystem containment, and error propagation.
- In frontend tests, cover bridge mapping and visible loading, cancellation,
  and error states. Keep Monaco and DOM types out of Rust domain tests.
- Include failure paths and boundary cases, not only successful examples.
  Prefer semantic assertions over tests coupled to private implementation
  details.

Use the checks documented in [Development Requirements](development-requirements.md)
as appropriate to the change. For Rust changes, the standard workspace checks
are:

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
```

Changes to window lifecycle, input routing, or visible behavior also require
the relevant live Tauri smoke test; headless Rust tests alone do not verify
the WebView and native host integration.

## Public APIs, unsafe code, and dependencies

Document public library contracts, error behavior, safety requirements, and
non-obvious invariants with rustdoc. Private helpers do not need exhaustive
prose. Keep architectural documentation synchronized when ownership or
observable behavior changes.

Avoid unsafe Rust unless a concrete requirement cannot be met safely. If it is
needed, isolate the unsafe operation, document its safety invariants at the
operation site, and add focused tests for the surrounding safe API. Keep
dependencies in the crate that owns the capability; in particular, do not add
Tauri or WebView dependencies to `functor_core`.

Prefer the simplest design that preserves the ownership boundaries and
behavior above. Measure before optimizing, and record intentional exceptions
when the reason would not be clear from the code.
