# Phase 7.1 Search Implementation Todo

Source plan: [Initial_Search.md](../notes/Initial_Search.md)

## Working Rules

- [ ] Preserve UTF-16 document offsets; never introduce byte offsets into editor contracts.
- [ ] Keep matching literal and regex-free for this phase.
- [ ] Keep search logic platform-neutral and keep Avalonia code focused on projection and input dispatch.
- [ ] Add focused tests with each implementation slice before starting the next slice.
- [ ] Do not modify generated `bin/` or `obj/` files.

## Slice 1 - Pure Search Contract

### Contract and engine

- [X] Decide the owning project/module for the pure search contract.
- [x] Define `SearchOptions` with query and case-sensitivity.
- [x] Define `SearchDocument` with document identity, path/display name, and text.
- [x] Define `SearchMatch` with document identity, exact UTF-16 range, line, column, length, and preview.
- [x] Define empty-query behavior as no matches.
- [x] Define ordinal case-insensitive matching as the default.
- [x] Define ordinal case-sensitive matching when enabled.
- [x] Define deterministic result ordering for documents and matches.
- [x] Decide whether multiline literal queries are supported or explicitly deferred.
- [x] Implement `SearchEngine.findInText` with a linear scan.
- [x] Ensure matches never split a UTF-16 surrogate pair.
- [x] Generate bounded previews near line and document boundaries.
- [x] Handle empty lines, long lines, repeated text, and query-longer-than-source input.
- [x] Normalize or document newline handling consistently with the editor buffer.

### Tests

- [x] Test empty query.
- [x] Test query with no matches.
- [x] Test multiple matches on one line.
- [x] Test matches across multiple lines.
- [x] Test default case-insensitive matching.
- [x] Test case-sensitive matching.
- [x] Test emoji and surrogate-pair UTF-16 offsets.
- [x] Test combining-mark text and exact range boundaries.
- [x] Test preview generation at the start and end of a document.
- [x] Test deterministic ordering.
- [x] Test a large plain-text input without indexing.

## Slice 2 - Navigation and Session State

### State and events

- [x] Extend or replace `SearchResult` with the richer match representation without breaking navigation semantics.
- [x] Store exact match ranges and the active match index/identity.
- [x] Store active search options and the result revision/snapshot identity.
- [x] Keep per-document matches in document session/navigation state.
- [x] Keep global query options and history outside per-document editing selection state.
- [x] Add events for changing options, applying matches, invalidating matches, and selecting a match.
- [x] Reset the active match when the query or options change.
- [x] Clear matches when the query is empty.
- [x] Invalidate matches when the document buffer revision changes.
- [x] Preserve bounded next/previous behavior when there are no results.
- [x] Define stale-result behavior for document, workspace, path, and revision mismatches.

### Tests

- [x] Test query changes reset results and selection.
- [x] Test option changes reset results and selection.
- [x] Test edits invalidate old ranges.
- [x] Test global search state across document tab switches.
- [ ] Test next/previous bounds and no-result behavior.
- [x] Test clear-search behavior.
- [x] Test stale-result rejection.

## Slice 3 - Application Search Orchestration

### Application contracts

- [x] Add application commands for query changes, option changes, refresh, next, previous, and clear.
- [x] Add application effects or services for workspace file enumeration and text reads.
- [x] Extend `IFileService` only with the minimum injectable enumeration API required.
- [x] Define platform-neutral file filtering and recoverable error behavior.
- [x] Carry workspace identity, document identity, path, and buffer revision through requests.
- [x] Add cancellation at workspace traversal and file-read boundaries.
- [x] Define stale asynchronous result rejection rules.

### Search sources and projections

- [x] Search the active document from its current in-memory buffer.
- [x] Search all open documents from in-memory buffers without rereading disk files.
- [x] Search unopened files through the injected file service.
- [x] Prefer unsaved open-document content over disk content.
- [x] Enforce the canonical workspace root.
- [x] Exclude directories and unsupported/binary files according to an explicit policy.
- [x] Produce a stable Search panel projection with path, line, column, preview, and counts.
- [x] Clear workspace results when the owning workspace changes.
- [x] Preserve deterministic ordering across open and unopened documents.

### Tests

- [x] Test active-buffer search sees unsaved edits.
- [x] Test open documents are not reread from disk.
- [x] Test unopened files are read through the abstraction.
- [ ] Test cancellation and stale results.
- [x] Test out-of-root files are excluded.
- [ ] Test inaccessible files produce recoverable status.
- [x] Test deterministic workspace ordering.
- [ ] Test workspace identity prevents old results from being applied.

## Slice 4 - Rendering Search Highlights

### Rendering contract

- [x] Add search ranges to backend-neutral rendering input/output.
- [x] Define inactive and active search highlight styles.
- [x] Convert match ranges to geometry with the existing `TextMeasurer`.
- [x] Reuse UTF-16 range normalization used by selections and tokens.
- [ ] Define overlay precedence with syntax, selection, cursor, and diagnostics.
- [ ] Keep search highlights independent from syntax token caches.
- [x] Update Avalonia drawing in one rendering location.
- [ ] Ensure scrolling and hit testing remain aligned with highlighted ranges.
- [x] Document multiline-query behavior if it remains deferred.

### Tests

- [x] Test visible matches.
- [ ] Test off-screen matches.
- [ ] Test multiple matches on one line.
- [ ] Test match geometry after emoji and combining text.
- [ ] Test horizontal scrolling.
- [x] Test active versus inactive styling.
- [ ] Test overlap with syntax styling.
- [ ] Test overlap with editor selection and cursor.
- [ ] Test empty-result rendering.

## Slice 5 - Current-Document Navigation and Commands

### Editor behavior

- [ ] Define whether opening search initializes from the current selection.
- [ ] Focus the query input when search opens.
- [x] Make next/previous select the active match.
- [x] Scroll the active editor to reveal the selected match.
- [x] Activate a result by moving to its exact UTF-16 range.
- [ ] Preserve grapheme-safe cursor and selection behavior.
- [ ] Refresh active-document search immediately or according to the selected debounce policy.
- [ ] Keep workspace searches cancellable/debounced after edits.
- [ ] Define behavior when no document is active.
- [ ] Add keyboard shortcuts where the input architecture supports them.
- [ ] Add command-palette entries for search, next, previous, replace, and replace-all.
- [ ] Make editor/search focus transitions deterministic.

### Tests

- [ ] Test command routing.
- [ ] Test search open and close focus behavior.
- [x] Test next/previous scrolling.
- [x] Test exact result activation.
- [ ] Test no-active-document behavior.
- [ ] Test query changes during editing.

## Slice 6 - Search Widget and Tool Panel

### Avalonia UI

- [x] Replace the Search tool placeholder with a reusable search control.
- [x] Add query input.
- [x] Add case-sensitivity toggle with accessible name and tooltip.
- [x] Add current-match and total-match indicator.
- [x] Add previous and next buttons.
- [x] Add an explicit action to search all open in-memory documents without a workspace.
- [x] Add replace input and replace/replace-all actions.
- [ ] Add close action that restores editor focus.
- [ ] Add loading, empty, error, and no-results states.
- [x] Add search history access.
- [ ] Add the floating in-editor search widget if the final UX keeps both surfaces.
- [ ] Reuse existing theme resources and panel sizing.
- [ ] Keep matching and replacement out of code-behind.
- [ ] Preserve accessible names and predictable control ordering.

### Tests

- [ ] Test query dispatch from the control.
- [ ] Test case-sensitivity dispatch.
- [x] Test open-files search dispatch.
- [x] Test next/previous dispatch.
- [ ] Test close and focus restoration.
- [ ] Test loading, empty, error, and no-results states.
- [ ] Test replacement control dispatch.
- [ ] Test panel width and layout stability.
- [x] Test accessible names/tooltips.

## Slice 7 - Workspace Results and File Activation

### Results UI and activation

- [x] Render file path, line/column, preview, and current-result styling.
- [x] Group or order results according to the deterministic result contract.
- [x] Activate an open-document result and reveal its range.
- [x] Open an unopened file when its result is activated.
- [x] Reuse canonical path identity and duplicate-open prevention.
- [x] Preserve unsaved open-file precedence during activation.
- [x] Keep result identity valid across tab switches.
- [x] Clear or invalidate results on workspace replacement.
- [ ] Show file filtering/read errors without discarding valid results.

### Tests

- [x] Test result rendering.
- [x] Test activation of an open-file result.
- [x] Test activation of an unopened-file result.
- [x] Test duplicate-open prevention.
- [x] Test unsaved open-file precedence.
- [x] Test workspace switching.
- [x] Test tab switching.
- [ ] Test result identity after refresh.

## Slice 8 - Safe Replacement

### Current-document replacement

- [x] Define whether replace acts on the current match or the editor selection.
- [x] Validate document identity and buffer revision before replacing.
- [x] Implement current-match replacement through normal editing events.
- [x] Implement replace-all using descending original ranges or a pure batch edit.
- [ ] Preserve or explicitly define undo/redo granularity.
- [x] Recompute/invalidate search ranges after replacement.

### Workspace replacement

- [x] Group replacements by document.
- [x] Apply replacements only to unchanged search snapshots.
- [ ] Route dirty documents through normal dirty-state and save flows.
- [x] Avoid direct writes that bypass file-service error handling.
- [x] Report stale documents separately from write failures.
- [ ] Define partial-failure behavior and user-visible status.

### Tests

- [x] Test replacement text containing Unicode.
- [x] Test replacement length changes.
- [x] Test zero matches.
- [x] Test current-match replace.
- [x] Test current-document replace-all.
- [x] Test replace-all across multiple files.
- [ ] Test dirty documents.
- [x] Test stale revisions.
- [x] Test write failures.
- [ ] Test partial failure reporting.
- [ ] Test undo/redo behavior.

## Slice 9 - Search History

### History policy

- [x] Decide and document session-only history for the initial implementation.
- [x] Define maximum history length.
- [x] Deduplicate history entries.
- [x] Exclude empty queries.
- [x] Record completed searches rather than every keystroke.
- [x] Define whether case-sensitivity is part of a history entry.
- [x] Keep history independent of document tabs and workspace replacement.
- [x] Add explicit history clear behavior.

### Tests

- [x] Test deduplication.
- [x] Test ordering.
- [x] Test bounded length.
- [x] Test empty-query exclusion.
- [x] Test option handling.
- [x] Test tab/workspace changes.
- [x] Test reset behavior.

## Slice 10 - Documentation and Acceptance

### Documentation

- [x] Document literal-search limitations.
- [x] Document file filtering and workspace scope.
- [x] Document replacement revision-safety behavior.
- [x] Document search history scope.
- [ ] Update relevant architecture notes after implementation is complete.
- [ ] Mark completed Phase 7.1 items in `ROADMAP.md` only after acceptance checks pass.

### Acceptance checks

- [ ] Search works on unsaved active text.
- [ ] All active-document matches are highlighted.
- [ ] The selected match is visually distinct.
- [ ] Next/previous navigation is bounded and scrolls into view.
- [ ] Workspace search finds eligible unopened files.
- [ ] Workspace results activate the correct file and range.
- [ ] Case-insensitive search is the default and can be disabled.
- [ ] Replace and replace-all preserve UTF-16 and grapheme integrity.
- [ ] Replacement honors document revision safety.
- [x] Search state survives tab switches globally.
- [ ] Search state clears on workspace replacement.
- [ ] Focus transitions work from keyboard and mouse flows.
- [ ] Focused domain/application/rendering/workspace tests pass.
- [ ] Headless Avalonia tests pass.
- [ ] The full solution test suite passes.
- [ ] Desktop smoke testing covers search, navigation, replacement, workspace results, and focus transitions.

## Recommended Execution Order

- [ ] Complete Slice 1 and its pure engine tests.
- [ ] Complete Slice 2 and its navigation/session tests.
- [ ] Complete Slice 3 and its application orchestration tests.
- [ ] Complete Slice 4 and its rendering tests.
- [ ] Complete Slice 5 and its editor command tests.
- [ ] Complete Slice 6 and its Avalonia widget tests.
- [ ] Complete Slice 7 and its workspace activation tests.
- [ ] Complete Slice 8 and its replacement tests.
- [ ] Complete Slice 9 and its history tests.
- [ ] Complete Slice 10 and record final acceptance results.
