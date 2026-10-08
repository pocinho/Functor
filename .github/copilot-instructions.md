# Functor Repository Instructions

Read `books/dev_book/src/architecture/project.md` and
`books/dev_book/src/architecture/system.md` before making structural changes.
The accepted default is Tauri 2 with a trusted HTML interface and Monaco;
feature parity and migration from the Winit/WGPU application are incomplete.

## Architecture

For new UI/editor work, preserve this dependency direction:

```text
Tauri host -> HTML/CSS/JavaScript UI with Monaco -> typed bridge DTOs
           -> Rust session -> semantic Message -> pure update -> Model
           -> native effects and typed replies -> UI state
```

- Keep Tauri/WebView dependencies out of Rust domain and update logic.
- Keep document invariants, identity, revisions, and edit validation in Rust
  domain/session code; keep DOM and Monaco types in the frontend.
- Use typed, validated command DTOs; do not expose generic domain messages or
  unrestricted filesystem access to the WebView.
- Keep native file operations and dialogs behind the Tauri host.

## Change Discipline

- Prefer the smallest change that preserves the ownership boundaries in the
  developer book.
- Add headless tests for model, update, bridge mapping, and layout behavior
  before GUI changes.
- For changes to window lifecycle, input routing, or visible geometry, run a
  live Tauri smoke test after the relevant Rust checks and frontend build.
- Track accessibility, IME, persistence, diagnostics, packaging, and platform
  coverage as unfinished until implemented and verified.
- Keep documentation synchronized with actual implementation status.
- Never revert unrelated user changes in the worktree.
