# Optimization Pass

## Completed

- [x] Reuse the workspace document control across shell refreshes so folder expansion state survives file and tab changes.
- [x] Update only the changed workspace section: Open Editors rows update independently from the file tree.
- [x] Skip workspace UI updates when the tree and tabs projections are unchanged.
- [x] Cache workspace tabs and the file tree by immutable workspace identity so resize, scroll, and status refreshes do not re-enumerate directories.
- [x] Coalesce pending UI refreshes posted to the Avalonia dispatcher.
- [x] Remove the duplicate `EditorStatusChanged` shell refresh subscription.
- [x] Remove the obsolete `SidePanelView` compatibility alias.
- [x] Keep side-panel width as a logical stored value instead of falling back to an unattached control's `NaN` width.

## Remaining Tasks

### High Impact

- [x] Move genuine workspace file-tree creation off the UI thread and publish the completed snapshot back to the UI thread.
- [x] Replace repeated path scans in `WorkspaceFileTree` with a path-to-document index built once per workspace snapshot.
- [x] Build the workspace tree with one sorting pass per directory and remove the defensive sort in `WorkspaceDocumentControl`.
- [x] Reuse document tab controls when tab order is unchanged; update active, dirty, and theme properties in place.

### Medium Impact

- [x] Diff `DocumentListView.ApplyTabs` instead of clearing and recreating every tab button on each projection.
- [x] Add a regression test proving that unchanged tab order reuses existing tab controls.
- [x] Cache resolved tab brushes and only recreate them when the editor theme changes.
- [x] Add performance tests for large workspace trees and rapid side-panel resizing.
- [x] Add a regression test proving that switching existing files does not rebuild the workspace tree.

### Validation

- [x] Resolve the auxiliary-panel projection test where the expected width is `320` but the detached panel reports `0`.
- [x] Validate file-open and resize paths with deterministic workspace and rapid-resize tests; defer wall-clock latency baselines to a profiled environment rather than making the test suite machine-dependent.

## Conclusion

The optimization pass is complete for the current architecture. Workspace tree enumeration is cached and asynchronous, rendering updates are incremental, theme brushes are reused, and shell refreshes are coalesced. The focused workspace and Avalonia suites provide structural and stress coverage; wall-clock latency remains an environment-specific profiling exercise rather than an automated acceptance threshold.
