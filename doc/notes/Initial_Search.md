# Phase 7.1 Search Implementation Plan

## Objective

Implement the Phase 7.1 search workflow across the domain, application, rendering, workspace, and Avalonia layers:

- Fast enough regex-free text matching using a linear scan.
- Case-insensitive matching by default with an explicit case-sensitive toggle.
- Search highlights for every match in the active editor.
- A current-match indicator and next/previous navigation.
- Context previews for document and workspace results.
- A floating in-editor search widget and a Search tool panel.
- Current-document and workspace-wide replace operations.
- Search history with predictable scope and lifecycle behavior.

The implementation should preserve the existing UTF-16 document-offset contract and the existing workspace/document identity model.

## Implemented Behavior

- Search is literal and line-local. It is ordinal case-insensitive by default and supports an explicit case-sensitive option. Regex, fuzzy, whole-word, structural, and multiline matching remain outside this phase.
- Active and open documents are searched from their current in-memory buffers, so unsaved edits take precedence over disk content. Workspace search is constrained to the canonical workspace root and applies the configured text-file and binary-file filters.
- Workspace search enumerates and reads all searchable unopened files in one cancellable operation. The Search panel receives the complete result set at once, while cancellation and stale-result rejection protect the UI during refreshes and edits.
- Workspace replacement uses the captured source snapshots. Open documents are edited through normal editor events only when their captured revision is still current. Unopened files are reread before writing; changed files are reported as stale and are not overwritten. Read and write failures are reported separately from stale files.
- Search history is session-only, bounded, deduplicated, excludes empty queries, and is cleared explicitly.

## Current Baseline

The repository already provides several foundations that Phase 7.1 should extend rather than replace:

- `Functor.Domain.Navigation` has `NavigationModel`, `SearchEvent`, `SearchResult`, query state, result selection, and bounded next/previous navigation.
- `PerDocumentSessionState` already preserves `NavigationModel` across tab switches.
- The Phase 7.0.1 shell has a global Search tool registration, tool rail button, resizable left panel, and an empty Search panel placeholder.
- `RenderingModel` already carries position-based ranges, visible text runs, selections, tokens, cursors, and diagnostics.
- `IFileService` currently reads and writes individual files but does not enumerate workspace files.
- Editing and save flows are revision-aware and should be reused for replacement instead of bypassed.

The current `SearchResult` type is sufficient for a simple line/column preview list, but it is not sufficient for highlighting or replacement because it has no match length, document identity, or exact UTF-16 range. Phase 7.1 therefore needs a richer match contract while retaining compatibility with navigation projections where useful.

## Design Decisions To Lock Before Implementation

1. **Match coordinates**: represent every match with a document identity, line, UTF-16 start column, UTF-16 length, and an exact position range. Do not use visual columns or byte offsets.
2. **Matching semantics**: search literal, line-local text only in this phase. Empty queries produce no matches. Matching is ordinal case-insensitive by default; case-sensitive mode uses ordinal comparison. Multiline literal queries are deferred. Regex, whole-word, and fuzzy matching are out of scope.
3. **Result ordering**: document results follow workspace tab/file traversal order; matches within a document follow ascending line and column order. Define and test whether unopened files are sorted by canonical path.
4. **Search state ownership**: per-document match ranges and current-result selection belong with document navigation/session state; query options, workspace result aggregation, and history belong to the application/workspace search state because the Search tool is global.
5. **Highlight precedence**: search highlights must remain visible alongside syntax styling and editor selection. The active match needs a distinct style from inactive matches, with selection/cursor rendering still readable.
6. **Replacement safety**: replace operations must dispatch normal editing/save commands and remain revision-aware. Workspace replace must not silently overwrite a document changed after the search snapshot.
7. **File scope**: workspace search must honor the canonical workspace root and skip directories/files that are not eligible for text search. The initial implementation should expose a small injectable file enumeration API so this policy is testable.

## Implementation Slices

### 1. Define the pure search contract

Add a platform-neutral search module, preferably alongside navigation/application contracts, containing:

- `SearchOptions` with query and case-sensitivity.
- `SearchMatch` with document identity/path, exact UTF-16 range, line/column, length, and preview data.
- `SearchDocument` or equivalent input containing document identity, display path, and text.
- `SearchEngine.findInText` for a single document using a linear scan.
- Workspace aggregation that preserves deterministic ordering.
- Preview generation with bounded context, line information, and safe handling of long lines, empty lines, and matches near document boundaries.

Specify behavior for repeated/overlapping-looking input, CRLF-normalized text, Unicode surrogate pairs, combining marks, and a query longer than the source. The engine must never split a UTF-16 code unit pair when producing a match range.

**Tests:** empty query, no match, multiple matches on one line, multiple lines, default case-insensitivity, case-sensitive mode, Unicode offsets, preview boundaries, deterministic ordering, and large plain-text input.

### 2. Extend navigation and session state

Replace or extend the current `SearchResult` representation without losing the existing `NextSearchResult` and `PrevSearchResult` behavior.

- Store exact match ranges and the selected match identity/index.
- Store the active search options and whether results are stale.
- Keep search state document-scoped so tab switches restore each document's matches correctly.
- Clear or invalidate matches when the document buffer revision changes.
- Keep query state separate from the document's editing selection.
- Define how switching query/options resets the current match and how an empty query clears results.

Add pure events/logic for setting options, applying matches, invalidating stale results, and selecting a match. Preserve bounded navigation and cover no-result behavior.

**Tests:** query changes reset results, edits invalidate old ranges, tab switching preserves independent results, current index bounds, clear behavior, and stale-result rejection.

### 3. Add application-level search orchestration

Introduce an application-facing search state/service that coordinates the pure engine with the active workspace.

- Search the active document immediately from its in-memory buffer.
- Search all open documents without rereading them from disk.
- Search unopened workspace files through an injected file-service abstraction.
- Carry workspace/document identity and buffer revision through asynchronous requests.
- Reject results when the workspace, document, path, or revision no longer matches the request.
- Expose commands/effects for query changes, option changes, refresh, next/previous, and clear.
- Keep cancellation at the file traversal/read boundary so typing in the search box does not queue obsolete workspace scans.
- Project results into a stable list suitable for the Search panel, including path, line, column, preview, and match count.

Extend `IFileService` only as needed for injectable file enumeration and text reads. Keep path filtering and error reporting platform-neutral; the desktop implementation can use the existing platform file service.

**Tests:** active-buffer searches see unsaved text, open-document searches do not lose per-tab state, unopened files are read through the abstraction, cancellation/stale results are ignored, out-of-root paths are excluded, inaccessible files produce recoverable status rather than crashing, and workspace ordering is deterministic.

### 4. Integrate match ranges into rendering

Add search highlight ranges to the backend-neutral rendering input and output.

- Convert visible search matches into geometry using the same `TextMeasurer` and UTF-16 range normalization used for selections and tokens.
- Support matches spanning multiple visual lines only if the contract is later expanded to allow multiline literal queries. The current contract is line-local, so multiline literal queries are deferred and return no matches.
- Render inactive matches with a subtle highlight and the selected match with a stronger accent.
- Preserve syntax foreground, editor selection, cursor visibility, and diagnostics.
- Avoid rebuilding or mutating syntax token caches just to display search state.
- Ensure horizontal/vertical scrolling and emoji/grapheme text keep highlight geometry aligned with the editor text.

Add a rendering model field for search geometry or a clearly defined overlay layer. Update Avalonia drawing in one place so future backends can consume the same contract.

**Tests:** visible and off-screen matches, multiple matches on one line, match after emoji/combining text, horizontal scrolling, active versus inactive styling, syntax overlap, selection overlap, and empty-result rendering.

### 5. Implement current-document navigation and editor commands

Wire the search engine and navigation state to editor behavior.

- Opening the search widget focuses the query field and initializes it from the current selection only if that matches the chosen product behavior.
- Next/previous moves the selected search match and scrolls the active editor to reveal it.
- Activating a result moves the cursor/selection to the exact UTF-16 range without corrupting grapheme boundaries.
- Search refreshes after edits according to a defined policy: immediate for the active document, debounced for workspace scans.
- Add keyboard commands and command-palette entries for open search, next, previous, replace, and replace-all where the existing input/command architecture supports them.
- Keep search focus and editor focus transitions deterministic, including when there is no active document.

**Tests:** command routing, focus behavior, next/previous scrolling, result activation, no-active-document handling, and query changes while the editor is being edited.

### 6. Build the floating search widget

Replace the placeholder Search tool content with a reusable Avalonia search control and a compact floating in-editor widget where appropriate.

The control should provide:

- Query input.
- Case-sensitivity toggle.
- Match count/current-match indicator.
- Previous/next buttons with accessible names and tooltips.
- Replace input and replace/replace-all actions.
- Close action returning focus to the editor.
- Loading, empty, error, and no-results states.
- Search history access without obscuring the active editor.

Use the existing theme resources, panel sizing, accessibility conventions, and control-hosting patterns. Keep the view as a projection/dispatcher; do not put matching or replacement logic in code-behind.

**Tests:** headless Avalonia coverage for rendering, toggles, query dispatch, navigation dispatch, close/focus behavior, loading/error/empty states, replacement controls, and panel width/layout stability.

### 7. Add workspace search results and file activation

Populate the global Search panel with grouped or clearly ordered results.

- Show document/file path, line/column, preview, and current-result styling.
- Activate a result by opening the file if necessary, then selecting/revealing the exact match.
- Reuse duplicate-open prevention and canonical path behavior from the workspace model.
- Preserve unsaved open-document content as the search source.
- Keep results valid across tab/workspace switches and clear them when the owning workspace changes.
- Surface file read/filter errors without making the entire result list unusable.

**Tests:** result rendering, unopened-file activation, duplicate-open prevention, unsaved open-file precedence, workspace switching, tab switching, and result identity after refresh.

### 8. Implement replacement safely

Start with current-document replacement, then add workspace replacement once the revision contract is covered.

- Replace the selected/current match only after validating its document identity and buffer revision.
- Replace all matches from the end of the document toward the beginning, or use a pure batch edit that guarantees original ranges are not shifted during application.
- Preserve undo/redo semantics if the editing model supports a batch event; otherwise define the expected granularity before implementation.
- For workspace replace, group edits by document, apply only to unchanged search snapshots, and route dirty documents through the normal save/dirty-state flow.
- Never write unopened files directly without the normal file-service error and save policy.
- Report skipped stale documents and write failures explicitly.

**Tests:** replacement text containing Unicode, replacement length changes, zero matches, current-only replace, replace-all, multiple files, dirty documents, stale revisions, write failures, and partial failure reporting.

### 9. Add search history and persistence policy

Implement bounded, deduplicated history for completed non-empty queries.

- Decide whether history is session-only initially; session-only is recommended for Phase 7.1 to avoid expanding settings persistence.
- Keep history independent from document tabs and clear it only through an explicit action or application reset.
- Do not record every keystroke; record a query when a search is executed or the widget closes with a valid query.
- Define maximum history length and case/options behavior.

**Tests:** deduplication, ordering, bounded length, empty-query exclusion, option handling, tab/workspace changes, and reset behavior.

### 10. Documentation and acceptance validation

Update the roadmap and relevant architecture notes after implementation, not before behavior is complete. Document the literal-search limitations, file filtering policy, replacement safety behavior, and history scope.

Acceptance checks:

- Search works on unsaved active text.
- Search highlights every active-document match and distinguishes the selected match.
- Next/previous navigation is bounded and scrolls the match into view.
- Workspace search finds eligible unopened files and activates results correctly.
- Case-insensitive search is the default and can be disabled.
- Replace and replace-all preserve UTF-16/grapheme integrity and revision safety.
- Search state survives tab switches where intended and is cleared on workspace replacement.
- Headless Avalonia tests and the full solution test suite pass.
- Desktop smoke test covers search, navigation, replacement, workspace results, and focus transitions.

## Suggested File/Project Impact

Likely additions or changes, subject to the existing F# compile order:

- `src/Functor.Domain/Navigation/*` or a new domain search module for pure match contracts and events.
- `src/Functor.Application/` for search orchestration, commands/effects, file enumeration, cancellation, and projections.
- `src/Functor.Workspace/` for workspace-scoped search state or search input projection if the application contract requires it.
- `src/Functor.Rendering/` for search ranges and overlay geometry.
- `src/Functor.Avalonia/` for Search controls, shell projection, floating widget, and input wiring.
- `src/Functor.Platform/` for desktop file enumeration/read implementation.
- `src/Functor.Tests/Functor.Tests.Domain/`, `Functor.Tests.Application/`, `Functor.Tests.Rendering/`, `Functor.Tests.Workspace/`, and `Functor.Tests.Avalonia/` for focused coverage.
- `doc/architecture/` and `ROADMAP.md` for the completed behavior and deferred limitations.

Keep generated `bin/` and `obj/` outputs out of source searches and commits.

## Recommended Delivery Order

1. Pure match contract and engine.
2. Navigation/session invalidation and selection state.
3. Application orchestration for active/open documents.
4. Rendering highlight overlay.
5. Current-document commands and floating widget.
6. Injectable workspace file enumeration and workspace results.
7. Safe current-document and workspace replacement.
8. Search history, documentation, and full validation.

Each slice should land with focused tests before the next slice begins. The first implementation checkpoint should be a pure engine test proving exact UTF-16 ranges and deterministic match ordering; it is the cheapest check that the central search assumption is correct.

## Explicit Non-Goals For Phase 7.1

- Regex, fuzzy, whole-word, structural, or semantic search.
- Search indexing or background indexing optimization.
- LSP symbol/search integration.
- Compiler-backed matching.
- Cross-workspace search.
- Advanced replace preview/diff UI.
- Persisted history unless a later product decision requires it.
- Skia-specific rendering behavior beyond consuming the backend-neutral overlay contract.
