# Decision 0002: Separate Rust Core and React WebView Frontend

- **Status:** Accepted
- **Date:** 2026-10-06
- **Builds on:** [Decision 0001](0001-tauri-monaco-default.md)

## Context

Functor's accepted Tauri/Monaco architecture had its Rust model, pure update
logic, bridge, host effects, and frontend build under the single `functor/`
package. The model/update code is independent of Tauri, while the frontend
needs a reusable React component boundary as workspace UI grows.

## Decision

- Keep `functor/` as the Tauri host, session/command boundary, typed bridge,
  and owner of native dialogs and filesystem effects.
- Put deterministic model/update logic and its pure Rust types/tests in the
  `functor_core/` crate. Keep Tauri, WebView, DOM, and Monaco dependencies out
  of this crate.
- Put the main Vite React/TypeScript application in `functor_web/`, loaded in
  the Tauri WebView, and use `functor_ui/` as Functor's canonical React widget
  library.
- Keep the IPC surface typed and validated. Do not serialize the Rust
  `Message` enum or grant the WebView unrestricted native access.
- Treat standalone browser-only, mobile, plugin, and cross-platform support as
  future possibilities, not capabilities established by this package split.

## Consequences

- Rust and frontend builds have explicit package boundaries, and the model
  and update engine can be tested without the Tauri host.
- React owns the workspace DOM; Monaco remains the editor surface and retains
  its model, selection, and undo responsibilities.
- The initial React shell is a foundation. Feature parity, broader widget
  extraction, accessibility, IME, persistence, packaging, and platform
  validation remain unfinished transition work.
