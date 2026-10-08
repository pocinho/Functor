# Proposal: Browser Smoke Target Backed by Client-Side WASM

## Status and intent

This is a proposal for a developer-focused browser smoke target, not a decision
to support Functor as a general-purpose web product. It aims to provide a fast,
automatable way to exercise the shared editor UI and Rust behavior during local
development, while retaining Tauri as the supported desktop host.

The browser target should reuse the existing React application and pure Rust
core. It must not become a second implementation of document editing or imply
that Tauri automatically provides a browser runtime.

## Current baseline

- [`functor_core/`](../functor_core/) contains the Rust model, MVU update
  behavior, syntax types, and effect descriptions. It has no Tauri dependency.
- [`functor/`](../functor/) owns the Tauri host, native file operations,
  protocol DTOs, and `EditorSession`.
- [`functor_web/`](../functor_web/) contains the React application and Monaco
  integration. Its bridge currently calls Tauri's global `invoke` API directly
  in [`bridge.js`](../functor_web/src/bridge.js).
- The session performs revision tracking, snapshot projection, request
  validation, and Monaco UTF-16 to Rust position mapping in
  [`session.rs`](../functor/src/session.rs) and
  [`protocol.rs`](../functor/src/protocol.rs). Filesystem operations themselves
  remain in [`file_io.rs`](../functor/src/file_io.rs).

The session and bridge mapping are therefore the main boundaries to resolve
before the same editor workflows can run through a client-side WASM target.

## Proposed structure

```text
functor_core
    Pure model, update logic, and domain/effect types
          |
          v
functor_session (candidate shared crate)
    Transport-neutral session, revision, validation, and projection behavior
       ^                                     ^
       |                                     |
functor (Tauri host)                  functor_browser (WASM adapter)
       ^                                     ^
       |                                     |
       +------------ functor_web ------------+
                 React UI and Monaco
```

`functor_session` is a candidate name for the transport-neutral portion of the
current Tauri session and protocol mapping. Confirm the extraction boundary
during implementation; it must not acquire Tauri, DOM, or filesystem
dependencies. Target-specific file-picker and file-save effects remain in
their respective hosts.

`functor_browser` would be a Rust `cdylib` compiled for
`wasm32-unknown-unknown`, with a small typed JavaScript API generated using
`wasm-bindgen` (or an equivalent browser-WASM binding tool). It would depend
on the shared session behavior and `functor_core`, not duplicate the reducer.
WASI is not proposed: ordinary browsers do not provide a WASI runtime.

`functor_web` should depend on a typed editor-bridge interface rather than
calling Tauri APIs directly. The Tauri build supplies a Tauri adapter; the
browser smoke build supplies a WASM adapter. Both adapters should implement
the same user-facing document operations and response/error shapes.

## Initial smoke-test scope

Start with workflows that can share the same document semantics:

- create and switch documents;
- edit text, selections, and cursor state;
- verify document revisions, stale-edit rejection, dirty state, and undo/redo
  reconciliation;
- render snapshots, errors, and tab state through the same React components.

Define browser file behavior explicitly. For a smoke target, use browser-safe
file selection and download behavior (or fixture documents) rather than
pretending that the browser has unrestricted native filesystem access.
File System Access API support may be an optional enhancement, not a baseline
assumption. Browser paths, permissions, external-file modification checks, and
native dialog behavior need not be identical to desktop behavior, but those
differences must be documented and tested.

Do not claim full browser feature parity, production hosting support, or
replacement of Tauri desktop validation from a successful WASM smoke run.

## Implementation plan

1. **Prove target compatibility.** Add the `wasm32-unknown-unknown` target and
   run a compile check for `functor_core`. Resolve any unsupported dependency
   or standard-library assumptions before committing to a WASM package.
2. **Define the typed frontend boundary.** Replace direct Tauri access in
   `functor_web` with an explicit editor-bridge interface, keeping Monaco and
   React in the frontend and keeping Rust `Message` values private.
3. **Extract shared session behavior.** Move only the transport-neutral
   session operations, revision rules, request validation, and snapshot/Monaco
   mapping needed by both targets into a WASM-compatible shared Rust package.
   Keep native I/O and Tauri command registration in `functor`.
4. **Add `functor_browser`.** Expose the shared operations to JavaScript via
   WASM bindings. Add browser-specific file effects with an explicit
   capability policy; do not expose host filesystem or process access.
5. **Select the adapter for development.** Provide a browser development/build
   mode for `functor_web` that loads the WASM adapter, while the Tauri target
   continues to load the Tauri adapter.
6. **Automate parity checks.** Exercise shared editing/session scenarios
   through the browser adapter with browser automation. Keep Tauri smoke tests
   for IPC, native dialogs and file operations, WebView behavior, window
   lifecycle, and packaging.
7. **Document support boundaries.** Record supported browser engines and
   versions, browser-specific file semantics, WASM build prerequisites, and
   which capabilities remain desktop-only.

## Advantages

- Faster edit/build/reload feedback than launching a desktop window for every
  UI iteration.
- Headless browser automation can run repeatable React, Monaco, and
  session-level smoke scenarios in CI.
- Reuses `functor_core` and, after extraction, shared session semantics rather
  than introducing a parallel application reducer.
- Tests browser-specific behavior early and clarifies what is genuinely
  portable versus Tauri-only.

## Disadvantages and maintenance costs

- Adds a WASM target, binding generation, generated artifacts, toolchain setup,
  caching, and CI maintenance.
- Requires a second host adapter and a stable bridge contract. DTO drift or
  duplicated session logic can make browser and desktop behavior diverge.
- Extracting session and protocol behavior out of `functor` adds a Rust
  package boundary and migration/testing work.
- Browser file APIs have different permissions, support, path semantics, and
  save behavior from native dialogs and filesystem operations.
- Browser smoke tests cannot validate Tauri IPC, native effects, WebView
  differences, desktop lifecycle, or packaging; those checks remain necessary.
- Supporting multiple browser engines and versions creates an additional
  compatibility matrix.

## Acceptance criteria for the smoke target

- `functor_core` and the shared session layer compile for
  `wasm32-unknown-unknown`.
- The browser build runs the same shared reducer/session rules as the Tauri
  app; no browser-only duplicate edit/revision implementation is introduced.
- Automated browser tests cover editing, selection, switching, revision/error
  behavior, dirty state, and the agreed browser file workflows.
- Existing native Rust tests and Tauri builds continue to pass, with targeted
  Tauri smoke tests covering native-only behavior.
- Browser limitations are visible and documented; no unavailable capability
  silently returns a success-shaped result.

## Recommendation

Proceed only as a developer smoke target if the faster feedback loop is worth
the WASM and adapter maintenance. Begin with the target-compatibility proof and
typed bridge boundary; defer full browser product support until browser file
semantics, supported engines, and feature-parity requirements are explicitly
accepted.