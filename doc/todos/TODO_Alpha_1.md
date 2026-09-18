# Phase Alpha 1 TODO

This checklist defines the first refactoring phase for Functor. It converts the architecture guidelines into implementation work while preserving existing editor behavior.

## Architectural Decision

The documented ViewModel-free MVU architecture is authoritative:

- State belongs in immutable application/domain models.
- Avalonia views project state and dispatch commands/events.
- Domain, rendering, and input contracts remain platform-neutral.
- Existing ViewModel work is treated as an incomplete migration to unwind deliberately, not as a reason to revise the architecture rules.

## Phase 1: Establish the Refactor Contract

- [x] Review this checklist before changing production code.
- [x] Preserve the existing pure domain, rendering, and application boundaries.
- [x] Keep this phase limited to architecture, input abstraction, ownership, and verification.
- [x] Do not include search, LSP diagnostics, Skia rendering, browser/mobile hosting, plugin architecture, broad performance rewrites, or unrelated bug fixes.

Phase 1 is complete. The next implementation slice is Phase 2: move application/UI state into immutable MVU state and remove the current ViewModel wiring while preserving behavior.

## Phase 2: Restore ViewModel-Free Avalonia MVU

### Application State

- [x] Keep UI projection state in dedicated immutable application modules alongside `AppSessionState`.
- [x] Model command-palette visibility, query, selection, and related state in immutable application state.
- [x] Model settings editing and validation state without Avalonia controls.
- [x] Keep domain state in `CoreModel` and application/session state in immutable application records.
- [x] Add pure update/transition functions using the existing command/event/effect patterns.
- [x] Keep settings parsing and creation pure, reusing `AppSettings` and `AppSettingsLoader`.
- [x] Extract settings draft parsing and validation into `Functor.Application.SettingsDraft` with focused tests.

### Avalonia Views

- [x] Refactor `MainWindow.axaml.fs` to project immutable shell state and dispatch commands/events.
- [x] Refactor `SettingsWindow.axaml.fs` to remove mutable ViewModel-owned settings state.
- [x] Refactor `CommandPaletteView.axaml.fs` to use application state and explicit dispatch.
- [x] Refactor `MainView.axaml.fs` and related views to remove view-local model duplication.
- [x] Remove direct ViewModel mutation and DataContext-based state ownership.
- [x] Ensure `AvaloniaApp.axaml.fs` no longer instantiates or imports ViewModels.
- [x] Keep only unavoidable transient Avalonia interaction state in controls, such as pointer capture.

### ViewModel Removal

- [x] Remove `ViewModelBase.fs` after all references are migrated.
- [x] Remove placeholder `MainViewModel.fs` and runtime/design-time DataContext wiring.
- [x] Remove `ShellViewModel.fs` and replace shell state with pure `Functor.Application.ShellState`.
- [x] Remove `SettingsViewModel.fs`.
- [x] Remove `CommandPaletteViewModel.fs` and move palette input/state projection into `CommandPaletteView` backed by pure `CommandPaletteState`.
- [x] Remove the `CommunityToolkit.Mvvm` package reference if no other usage remains.
- [x] Update F# compile order and project references in `Functor.Avalonia.fsproj`.
- [x] Search the repository for remaining `ViewModelBase`, `ViewModel`, and MVVM package usages.

## Phase 3: Implement the Platform-Neutral Input Boundary

### Input Contracts

- [x] Define platform-neutral keyboard types in `src/Functor.Input/Keyboard.fs`.
- [x] Define platform-neutral pointer types in `src/Functor.Input/Mouse.fs`.
- [x] Define platform-neutral text input types in `src/Functor.Input/TextInput.fs`.
- [x] Use immutable records and discriminated unions; do not reference Avalonia.
- [x] Use platform-neutral coordinates, buttons, modifiers, and key representations.

### Keymap

- [x] Implement `src/Functor.Input/Keymap.fs` as a pure shortcut-resolution module.
- [x] Separate shortcut matching from the resulting `AppCommand`, `CoreEvent`, or editing event.
- [x] Support platform command modifiers and future rebinding without embedding Avalonia key codes.
- [x] Keep keymap resolution deterministic and independently testable.
- [x] Add the required project references and compile ordering while keeping `Functor.Input` independent of Avalonia.

### Avalonia Adapter

- [x] Extract keyboard translation from `EditorControl.fs` into an Avalonia-to-input adapter.
- [x] Extract text-input translation from `EditorControl.fs` into the adapter boundary.
- [x] Extract pointer translation while keeping focus, pointer capture, invalidation, and scrolling in Avalonia.
- [x] Route pure input values through keymap/event resolution before dispatching commands.
- [x] Remove duplicated hardcoded shortcut matching from `EditorControl.fs` and `MainWindow.axaml.fs`.
- [x] Preserve copy, cut, paste, file commands, command palette/settings shortcuts, cursor movement, deletion, newline, pointer selection, and scrolling behavior.

## Phase 4: Tighten Purity and Ownership

- [x] Review the local mutable grapheme-advance accumulator in `src/Functor.Domain/Editing/EditingLogic.fs`.
- [x] Prefer a recursive or fold-based pure helper if readability and performance remain acceptable.
- [x] If localized mutation is retained, document the narrowly scoped implementation exception and add regression coverage.
- [x] Keep `EditorControl.fs` pointer-selection state transient and UI-local.
- [x] Isolate or document pointer interaction state; do not move pointer capture into domain/application state.
- [x] Audit imports and project references for forbidden UI dependencies in Domain, Rendering, Input, and pure Application modules.
- [x] Preserve the existing rendering backend adapter and composed `CoreModel` dependency direction.

## Phase 5: Verification and Test Coverage

### Pure Tests

- [x] Add input contract and keymap tests.
- [x] Test modifier precedence, platform command modifiers, unmapped shortcuts, and configurable rebinding behavior.
- [x] Test text input and pointer input conversion where applicable.
- [x] Add application/MVU tests for command-palette transitions.
- [x] Add application tests for settings validation and application.
- [x] Extract command-palette filtering and selection into `Functor.Application.CommandPaletteState` with focused tests.
- [x] Add application tests for shell settings and command-palette visibility transitions.
- [x] Add application/MVU tests for shell visibility and command dispatch without Avalonia controls.

### Avalonia Tests

- [x] Test representative keyboard adapter interactions.
- [x] Test text input and pointer-selection adapter interactions.
- [x] Test command-palette and settings command routing.
- [x] Keep platform event construction in Avalonia tests and pure mapping assertions in pure test projects.

### Completion Checks

- [x] Build affected projects after each refactoring slice.
- [x] Run focused tests after each refactoring slice.
- [x] Run pure keymap, application, domain, rendering, and workspace tests independently of Avalonia.
- [x] Run Avalonia tests for adapter and state-projection behavior.
- [x] Build and test the full solution.
- [x] Confirm `Functor.Input` has no Avalonia dependency.
- [x] Confirm no ViewModel or `CommunityToolkit.Mvvm` usages remain after the MVU migration.
- [x] Manually smoke-test desktop startup, opening a file, and writing text.
- [x] Manually smoke-test new/save/close, settings, command palette, copy/paste, cursor movement, pointer selection, and scrolling.

## Relevant Files

- `doc/architecture/code_guidelines.md`
- `doc/architecture/best_practices.md`
- `src/Functor.Application/AppSessionState.fs`
- `src/Functor.Application/AppCommand.fs`
- `src/Functor.Application/AppCommandCatalog.fs`
- `src/Functor.Application/AppEffect.fs`
- `src/Functor.Application/EditorSession.fs`
- `src/Functor.Application/AppSettings.fs`
- `src/Functor.Application/AppSettingsLoader.fs`
- `src/Functor.Input/Keyboard.fs`
- `src/Functor.Input/Mouse.fs`
- `src/Functor.Input/TextInput.fs`
- `src/Functor.Input/Keymap.fs`
- `src/Functor.Input/Functor.Input.fsproj`
- `src/Functor.Avalonia/Controls/EditorControl.fs`
- `src/Functor.Avalonia/Views/MainWindow.axaml.fs`
- `src/Functor.Avalonia/Views/SettingsWindow.axaml.fs`
- `src/Functor.Avalonia/Views/CommandPaletteView.axaml.fs`
- `src/Functor.Avalonia/Views/MainView.axaml.fs`
- `src/Functor.Avalonia/AvaloniaApp.axaml.fs`
- `src/Functor.Avalonia/ViewModels/`
- `src/Functor.Avalonia/Functor.Avalonia.fsproj`
- `src/Functor.Domain/Editing/EditingLogic.fs`
- `src/Functor.Tests/Functor.Tests.Application/`
- `src/Functor.Tests/Functor.Tests.Avalonia/`
- `src/Functor.Tests/Functor.Tests.Domain/`
- `src/Functor.Tests/Functor.Tests.Rendering/`
- `src/Functor.Tests/Functor.Tests.Workspace/`

## Working Rules

- Make small, behavior-preserving changes.
- Validate each slice before starting the next.
- Do not revert unrelated user changes.
- Do not reintroduce mutable ViewModel state to solve an Avalonia update problem; add a small application update bridge if necessary.
