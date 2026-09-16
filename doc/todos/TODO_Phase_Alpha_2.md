# Phase Alpha 2 TODO

This checklist defines the code-verification phase for Functor. It turns the roadmap and architecture guidelines into an evidence-driven backlog for finding unintended behavior, fixing confirmed defects, and adding missing contract coverage.

## Architectural Decision

The documented ViewModel-free MVU architecture remains authoritative:

- Domain, rendering, input, and pure application logic remain platform-neutral and testable.
- Avalonia views project immutable application state and dispatch commands/events.
- Mutable Avalonia state is permitted only for transient control mechanics that cannot belong in application state, such as pointer capture or control-update guards.
- Suspected defects are hypotheses until a focused test, diagnostic, or manual scenario reproduces them.

## Phase 1: Establish the Verification Contract

- [x] Review `ROADMAP.md`, `TODO_Phase_Alpha_1.md`, and the architecture guidelines before changing production code.
- [x] Keep Alpha 2 focused on verification, bug fixing, architecture conformance, and missing tests.
- [x] Require reproduction or a focused disconfirming check before changing code for a suspected issue.
- [x] Exclude new navigation/search features, LSP semantic services, Skia work, browser/mobile hosting, plugins, packaging, and unrelated performance rewrites.

## Phase 2: Dependency and Purity Audit

- [x] Verify project dependency direction and forbidden references for Domain, Rendering, Input, and pure Application modules.
- [x] Audit pure modules for mutable state, hidden I/O, async work, UI types, and broad responsibilities; no confirmed violations found.
- [x] Verify rendering remains backend-neutral and produces platform-independent projections/commands.
- [x] Verify input translation remains at the Avalonia adapter boundary.
- [x] Verify no new ViewModel or DataContext ownership is introduced.
- [x] Document legitimate UI-local mutable mechanics, including pointer capture and control-update guards.

## Phase 3: Behavioral Bug Hunt

### Application Session and Lifecycle

- [x] Propagate cancellation through debounced tokenization effects and ignore canceled tokenizer work when edits supersede it.
- [x] Cover missing-tokenizer service behavior and debounce cancellation for successive edits.
- [x] Audit incremental tokenization state across document identity, revision, line-range, and close transitions; edited-range snapshots are invalidated, later snapshots remapped, and closed-document state removed.
- [x] Audit pending save state when documents are edited, saved repeatedly, switched, or closed; verified Save and Save As no-ops without an active document, overlapping saves preserving each completed revision's saved snapshot, and pending saves being removed on completion, close, and workspace replacement.
- [x] Verify stale tokenization and save completions cannot mutate the wrong document or revision; coverage includes old revisions, closed documents, and inactive documents sharing a revision number.
- [x] Require `FileSaved` completions to match a pending save, including revision-safe untitled saves; unrelated completions no longer clear dirty edits.
- [x] Require `FileSavedForDocument` completions to match a pending save before cleaning an inactive or active document.
- [x] Verify pending-operation cancellation and confirmation transitions for dirty close and new-document commands.

### Effect and Service Boundaries

- [x] Audit `AppEffectInterpreter` behavior when optional services, especially tokenizer services, are absent.
- [x] Document intentional no-op effects or add explicit error/status behavior where the contract is ambiguous.
- [x] Verify file, clipboard, dialog, and tokenizer failures remain represented as application effects/status rather than leaking into domain logic.

### Avalonia State Projection

- [x] Verify repeated command-palette configuration, filtering, selection, close, and reopen sequences leave consistent state.
- [x] Verify settings control updates and preset changes do not produce stale drafts or recursive updates.
- [x] Verify `SettingsView` configure, edit, preset, and reconfigure sequences keep controls and drafts synchronized.
- [x] Verify `MainView` tab, empty-state, status, and recent-document projections follow session state after repeated transitions.
- [x] Verify `MainView` tab, empty-state, status, and dirty-state projections across repeated close transitions.
- [x] Verify `MainWindow` recent-document menu projection across close and clear-history transitions.
- [x] Verify command-palette filtering and configure/reset event sequences in the Avalonia view.
- [x] Use headless Avalonia tests for reproducible event sequences; do not refactor based on theoretical concurrency alone.

### Input and Editor Behavior

- [x] Audit modifier precedence, unsupported keys, modifier-only input, empty text, and extreme pointer coordinates.
- [x] Verify command/editor shortcut routing preserves copy, cut, paste, file commands, cursor movement, deletion, newline, selection, and scrolling.
- [x] Verify platform-neutral input contracts remain independent of Avalonia.

## Phase 4: Coverage Expansion

### Pure Application Tests

- [x] Extend `SettingsDraft` tests for invalid color lengths, non-finite border widths, whitespace/optional values, and preset round trips where applicable.
- [x] Extend `CommandPaletteState` tests for empty commands, filtering to empty, repeated queries, and selection clamping.
- [x] Extend `EditorSession` tests for multi-document switching, dirty/save revision ordering, close/reopen flows, stale results, and pending operations.
- [x] Add `AppEffectInterpreter` tests for optional service behavior and tokenizer/file failure propagation.

### Domain, Rendering, and Workspace Tests

- [x] Add regression coverage and fix the confirmed domain Delete-at-end-of-line and grapheme-boundary defect.
- [x] Expand rendering tests for deterministic layout, viewport slicing, token ranges, glyph runs, and diagnostic geometry where coverage is missing.
- [x] Expand workspace tests for document identity, canonical paths, tab membership, active-document fallback, and lifecycle invariants.
- [x] Add syntax/tokenizer tests for malformed input, large buffers, incremental ranges, cancellation, and lexer-state stabilization where missing.

### Avalonia and Integration Tests

- [x] Extend `InputAdapter` tests for unsupported keys, modifier-only combinations, empty text, whitespace text, and extreme pointer values.
- [x] Add headless tests for reproducible settings, palette, tab, and state-projection event sequences.
- [x] Add integration tests only at contracts crossing Application, Avalonia adapters, platform services, or tokenizer scheduling boundaries.
- [x] Avoid brittle visual tests for platform-specific drawing output; use pure projection/layout assertions instead.

## Phase 5: Implementation and Verification Loop

- [x] For each confirmed defect, make the smallest root-cause fix in the owning module.
- [x] Add a focused regression test for every production bug fix.
- [x] Run the narrowest affected project test immediately after each edit.
- [x] Run affected project builds/tests in dependency order after each verification slice.
- [x] Run `dotnet test .\src\Functor.slnx --no-restore` after all Alpha 2 slices.
- [x] Search for forbidden Avalonia, SkiaSharp, I/O, mutable, ViewModel, and DataContext references in pure projects.
- [x] Perform targeted desktop smoke testing for user-facing fixes and record exactly which workflows were exercised.
- [x] Update this checklist only when work is implemented and verified; leave unresolved hypotheses explicitly labeled.

## Priority Order

1. Reproduce application session, tokenization, save, and document-lifecycle defects.
2. Verify effect/service boundary contracts and error propagation.
3. Add pure application, workspace, syntax, rendering, and input regression coverage.
4. Verify Avalonia state projections and headless event sequences.
5. Complete dependency and purity searches, then run the full solution validation.

## Relevant Files

- `ROADMAP.md`
- `TODO_Phase_Alpha_1.md`
- `doc/architecture/best_practices.md`
- `doc/architecture/code_guidelines.md`
- `src/Functor.Application/EditorSession.fs`
- `src/Functor.Application/AppEffectInterpreter.fs`
- `src/Functor.Application/SettingsDraft.fs`
- `src/Functor.Application/CommandPaletteState.fs`
- `src/Functor.Avalonia/Views/MainWindow.axaml.fs`
- `src/Functor.Avalonia/Views/MainView.axaml.fs`
- `src/Functor.Avalonia/Views/CommandPaletteView.axaml.fs`
- `src/Functor.Avalonia/Views/SettingsView.axaml.fs`
- `src/Functor.Avalonia/InputAdapter.fs`
- `src/Functor.Avalonia/Controls/EditorControl.fs`
- `src/Functor.Input/Keyboard.fs`
- `src/Functor.Input/Mouse.fs`
- `src/Functor.Input/TextInput.fs`
- `src/Functor.Input/Keymap.fs`
- `src/Functor.Domain/`
- `src/Functor.Rendering/`
- `src/Functor.Workspace/`
- `src/Functor.Tests/Functor.Tests.Application/`
- `src/Functor.Tests/Functor.Tests.Avalonia/`
- `src/Functor.Tests/Functor.Tests.Domain/`
- `src/Functor.Tests/Functor.Tests.Input/`
- `src/Functor.Tests/Functor.Tests.Rendering/`
- `src/Functor.Tests/Functor.Tests.Workspace/`

## Working Rules

- Make small, behavior-preserving changes.
- Validate each slice before starting the next.
- Do not revert unrelated user changes.
- Fix root causes in the owning abstraction rather than applying surface-level patches.
- Do not introduce mutable ViewModel state or move transient pointer mechanics into domain/application models.
- Prefer targeted coverage over an arbitrary global coverage percentage unless the repository establishes a stable baseline and tool.
