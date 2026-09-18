# TODO Alpha 4

## Plan: Tab-Scoped Notebook And Agent Workspaces

Redesign the shell so each document tab owns its notebook and agent state. The active tab will show a toolbar above the editor with independent Notebook and Agent toggles; both panels may be open and stack above the editor.

**Steps**

1. ✅ Add tab-scoped auxiliary state to `PerDocumentSessionState`.
   - Initialize notebook and agent state for new documents.
   - Add workspace events and pure update logic for toggling each panel.
   - Preserve this state during tab switching and editor synchronization.

2. ✅ Extend workspace projections.
   - Expose notebook/agent visibility per tab and for the active tab.
   - Remove panel selection as the source of truth from global `ShellModel`.
   - Keep shared shell geometry and animation state global.

3. ✅ Reshape the Avalonia shell.
   - Replace the global panel rail with an active-tab toolbar.
   - Place notebook and agent hosts above the editor.
   - Keep both toggles independent and restore their state when switching tabs.
   - Retain placeholder/adaptor hosts until notebook and agent runtimes exist.

4. ✅ Narrow legacy global panel behavior.
   - Update `ShellModel`, `ShellUpdate`, `ShellProjection`, and `ShellViewNode`.
   - Avoid persisting per-tab auxiliary state in `WorkspaceLayout` during this pass.
   - Preserve existing welcome, tab closing, scrolling, and status behavior.

5. ✅ Add regression coverage.
   - Workspace tests for independent per-document state.
   - Application tests for state surviving `SwitchDocument`.
   - Projection tests for active-tab state.
   - Avalonia headless tests for toolbar placement, toggles, simultaneous panels, and restoration after tab switching.
   - Update architecture notes after implementation.

**Relevant files**

- `WorkspaceModel.fs` — per-tab state.
- `WorkspaceEvent.fs` — tab-scoped events.
- `WorkspaceLogic.fs` — immutable transitions.
- `WorkspaceProjection.fs` — tab projections.
- `EditorSession.fs` — state synchronization.
- `ShellModel.fs` — global shell state.
- `ShellHostView.axaml` — layout.
- `ShellHostView.axaml.fs` — event wiring and refresh.
- `ShellProjection.fs` and `ShellViewNode.fs` — presentation model.
- Workspace, Application, and Avalonia test projects — focused regression coverage.

**Verification**

1. Run narrow Workspace, Application, and Avalonia tests after each layer.
2. Verify tab A and tab B retain different notebook/agent states after switching.
3. Verify both panels can be open simultaneously.
4. Run the full solution build and test suite.

**Scope**

Included: live tab ownership, toolbar layout, toggles, projections, placeholder hosts, and tests.

Excluded: notebook execution, cell parsing, HTML rendering, MCP agent UI, and cross-restart persistence.