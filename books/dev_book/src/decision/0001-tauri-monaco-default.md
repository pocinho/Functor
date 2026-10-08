# Decision 0001: Tauri and Monaco Are the Default

- **Status:** Accepted
- **Date:** 2026-10-05
- **Supersedes:** The prior proposal to keep Tauri as a gated UI-host candidate
  before choosing a default direction.

## Context

The Winit/WGPU application contains a custom editor surface while the Tauri
implementation has demonstrated a Rust-owned, revisioned document workflow
with Monaco. Maintaining a custom text editor recreates behavior already
provided by a mature editor component and multiplies input, selection, undo,
text shaping, and accessibility work.

The prototype establishes that Monaco can issue validated edits through a
typed Tauri bridge while Rust retains document state and owns native file
operations. The implementation is incomplete, but the user has accepted this
direction as the default for Functor development.

## Decision

Functor's default desktop UI architecture is Tauri 2 with a trusted HTML UI
and Monaco as the default code editor. Rust remains authoritative for
documents, domain invariants, and native effects. New UI/editor features
should target this architecture.

The product name is **Functor** (singular). Existing Cargo package names,
repository identifiers, executable metadata, and window labels are migration
details; update them in a planned identity/packaging change rather than
changing the product name back to Functors.

## Consequences

- Do not extend the custom Winit/WGPU editor as a competing default UI.
- Keep the Winit application available during the feature-parity transition.
- Migrate needed behavior feature-by-feature and remove the old application
  only after explicit parity and platform validation.
- Keep the Tauri command surface typed, validated, and narrow. Do not expose
  unrestricted filesystem or process APIs to the WebView.
- Accept a Node.js/npm frontend build dependency and operating-system WebView
  requirements for the Tauri application.
- Keep IME, diagnostics, accessibility, offline packaging, cross-platform
  support, and performance as unfinished acceptance work until tested.
- Do not infer mobile, web, or production support from the desktop decision.
