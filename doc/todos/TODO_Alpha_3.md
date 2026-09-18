# UI Reset

## Plan: Final Alpha 3 TODO

1. [x] Define pure shell types in `Functor.Avalonia`: `ShellPanel`, `ShellLayoutState`, `ShellModel`, `ShellMsg`, and `ShellEffect`. `ShellPanel` has no `None` case; `ActivePanel` is optional. `shellUpdate` returns `ShellModel * ShellEffect list` and owns layout only.

2. [x] Define `ShellProjection` and `ShellViewInput`. Derive tabs from `WorkspaceProjection.tabs`, active-document visibility from `AppSessionState`, status from `EditorStatus`, and scroll presentation from `EditorControl`. Do not copy these values into `ShellModel`.

3. [x] Define immutable `ShellViewNode` descriptions and make `ShellView.applyModel` the sole mutable Avalonia adapter. It updates a stable control tree idempotently and never recreates `ShellHostView` or `EditorControl`.

4. [x] Add `ShellHostView`, initially containing exactly one retained `EditorControl`. It runs the shell loop, translates UI events to `ShellMsg`, executes `ShellEffect` outside `shellUpdate`, refreshes projections from editor events on the Avalonia UI thread, and disposes event subscriptions when detached.

5. [x] Replace `MainWindow`’s direct editor-area host with `ShellHostView`, preserving the editor-area control tree and stable `EditorControl`. Use existing operations:
   - `AppCommand.openFile`
   - `AppCommand.closeDocument`
   - `AppCommand.reopenClosedTab`
   - `EditorControl.ActivateDocument` for activation
   - Single-view platform lifetimes also create `ShellHostView`.

6. [x] Add all new F# files to `Functor.Avalonia.fsproj` in dependency order. Do not add a direct Avalonia-to-Workspace project reference.

7. [x] Migrate the former editor-host responsibilities into `ShellHostView`: tabs, side-panel host, welcome/status, then scrollbars. Remove each old responsibility only after its focused headless regression tests pass.
   - [x] ShellHostView owns projection refreshes for tabs, welcome/status, and scrollbars while attached.
   - [x] Add focused hosted-path and ownership lifecycle tests.
   - [x] Implement the side-panel host and panel controls.
   - [x] Remove the remaining gated fallback handlers after migrating projection regressions behind `ShellHostView`.

8. [x] Remove `MainView` after migrating all responsibilities into `ShellHostView`, which directly owns the single retained `EditorControl`.

9. [x] Implement layout persistence as `.functor/layout.json`: schema/serialization in `Functor.Workspace`, lifecycle coordination in `Functor.Application`, and no persisted document content or domain state.
   - [x] Define the versioned `WorkspaceLayout` JSON contract and validation in `Functor.Workspace`.
   - [x] Add `WorkspaceLayoutStore` path/load/save orchestration over `IFileService` in `Functor.Application`.
   - [x] Wire layout restoration and saving into the shell/workspace lifecycle.

10. [ ] Treat notebook, agent, and plugin panels as later adapters built on the stable `ShellPanel` and side-panel host contracts.

**Verification**
1. [x] Add pure tests for shell updates, width clamping, panel selection, animation state, effects, and view-node generation.
2. [x] Add Avalonia headless tests for stable `EditorControl`/`EditorSession` identity, UI-thread projection updates, and subscription cleanup.
   - [x] Stable control identity.
   - [x] Hosted projection refresh.
   - [x] Ownership release and subscription disposal on detach.
   - [x] Explicit cross-thread dispatcher regression test.
   - [x] Persisted shell layout restoration regression test.
3. [x] Run existing `ShellHostView`/`MainWindow` regression tests after each migration slice.
4. [x] Run `dotnet test .\src\Functor.slnx` after every completed slice.