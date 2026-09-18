# Architecture Differences

This note compares the implementation in the current worktree with `Previous_Architecture.md`. It records implemented changes only; it does not treat roadmap or plugin-design proposals as runtime changes.

## Summary

The core editor architecture has not changed: immutable domain updates, `EditorSession` orchestration, workspace synchronization, effect interpretation, input adaptation, and backend-neutral rendering remain in place. The substantive change is in the Avalonia presentation shell and workspace layout persistence.

## Implemented Changes

### `MainView` was replaced by a shell host

- `Views/MainView.axaml` and `Views/MainView.axaml.fs` were removed.
- `MainWindow` now hosts `ShellHostView` directly, with the command palette remaining an overlay owned by the window.
- `AvaloniaApp` now creates `ShellHostView` for single-view/activity lifetimes instead of the former main view.

### A shell state, projection, and view-node layer was added

- `ShellModel` contains isolated layout state for a side panel: visibility, width, active panel, and animation state.
- `ShellUpdate.update` is a pure shell reducer and may issue application-command effects for file-related shell messages.
- `ShellProjection` derives tabs, active-document visibility, editor status, and scrollbar presentation from `EditorControl` and `AppSessionState`.
- `ShellViewNode` describes presentation nodes, and `ShellView` applies the projection and side-panel node to the host view.
- `ShellHostView` replaces the prior view as the imperative bridge: it subscribes to editor notifications while attached and refreshes the visible presentation on the Avalonia UI thread.

This shell boundary does not replace `EditorSession`; it isolates layout state and presentation projection from editor-session state.

### The desktop shell now includes panel and tab controls

- `SidePanelView` is a reusable host with a title, close action, content slot, visibility, and width.
- `DocumentListView` renders workspace tab projections and exposes activation and close events.
- The shell has a panel rail for `Notebook` and `Agent` panels, dedicated tabs, a welcome/editor empty-state switch, status bar, and scrollbars.
- The notebook, agent, and named plugin panels are currently placeholders that display their names. No notebook, agent, or plugin-host runtime integration was added.

### Workspace layout persistence was added

- `WorkspaceLayoutStore` was added to the application layer.
- `MainWindow` loads a layout when the workspace root changes and saves the previous/current layout through the file service.
- `WorkspaceLayout` support and corresponding workspace/application tests were added.

### Tests moved with the presentation architecture

- The former `MainViewTests.fs` was removed.
- Avalonia tests now cover `ShellModel`, `ShellProjection`, `ShellViewNode`, `ShellView`, and `ShellHostView`.
- Application and workspace tests cover `WorkspaceLayoutStore` and workspace layout behavior.

## Unchanged Architectural Boundaries

- `CoreModel`, `WorkspaceModel`, and `AppSessionState` remain the primary editor, workspace, and combined-session state boundaries.
- `EditorSession` remains the coordinator for domain/workspace synchronization, effects, tokenization, pending saves, and session notifications.
- `EditorControl` still owns the session plus concrete clipboard, file, dialog, tokenizer, and effect-interpreter composition.
- Input is still adapted from Avalonia into `Functor.Input`; rendering is still computed in `Functor.Rendering` and drawn by the Avalonia backend.
- LSP, MCP/agent, and plugin-host projects remain outside the active editor composition path.
- Avalonia remains the only wired rendering backend, and desktop remains the principal runtime path.

## Documentation Corrections

The previous note described `MainView` as the surface responsible for editor tabs, scrolling, welcome state, status, and discard confirmation. That responsibility is now implemented by `ShellHostView` and its shell projection path.