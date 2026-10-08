# Tauri Default-Architecture Transition

**Status:** Complete; Tauri 2 and Monaco are the active default architecture.
Feature development continues under Functor's product direction; full parity
with historical applications and release readiness are not transition gates.

## Completed foundation

- Isolated Tauri 2 host and Monaco 0.57.0 frontend.
- Typed command DTOs, explicit errors, revisioned edits, stale/replay
  rejection, and UTF-16 position conversion.
- Shared Rust Model/MVU workflow with multiple documents, native open/save,
  save-snapshot handling, cursor/selection synchronization, and undo/redo
  dirty-state reconciliation.
- Moved pure model/update logic and its headless tests into `functor_core/`;
  native operations and bridge/session code remain in `functor/`.
- Established `functor_web/` as the Vite React/TypeScript target and
  `functor_ui/` as the reusable React component package; Tauri loads the built
  web target.
- Windows development and release smoke tests.

## Work after the transition

The former Winit/WGPU application and widget crates are historical references,
not an active host awaiting feature parity or retirement. F# capabilities that
were partial or unfinished are not inherited requirements. Ongoing editor
usability, diagnostics providers, notebook structure, and other Functor
features are scoped independently in the product direction and their own
feature work.

Accessibility, platform coverage, packaging, IME validation, and performance
remain useful product-quality topics to address when relevant, but they do not
block current feature development merely to satisfy a migration checklist.
