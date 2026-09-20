# Recommended Code Review Actions

## Review Context

The current codebase has a sound pure-domain and rendering foundation. The main maintainability risk is orchestration concentration in the application coordinator and Avalonia shell, not widespread impurity in domain logic.

## Findings By Priority

### High: ShellHostView Is a God Module

`src/Functor.Avalonia/Views/ShellHostView.axaml.fs` coordinates shell projection, workspace-tree loading and invalidation, settings application and export, dialogs, tabs, scrollbars, tool panels, and command routing.

Recommended decomposition:

- `ShellHostProjection.fs` for pure model-to-control projection helpers.
- `WorkspaceTreeCoordinator.fs` for workspace keys, loading, generation checks, and refresh.
- `SettingsCoordinator.fs` for settings validation, apply/save, and export callbacks.
- Keep `ShellHostView` as the Avalonia lifecycle and event adapter.

This is the highest-value refactor because it improves testability without requiring an MVU rewrite.

### High: EditorSession Has Too Many Coordinator Responsibilities

`src/Functor.Application/EditorSession.fs` is an application coordinator, so its mutable state is not a Domain purity violation. However, it combines model updates, event publication, effect requests, tokenization scheduling, cancellation, incremental tokenization, pending-save tracking, and confirmation workflows.

Recommended extraction:

- `EditorSessionUpdate.fs` for pure model and event transitions.
- `EditorSessionTokenization.fs` for scheduling, cancellation, and incremental tokenization.
- `EditorSessionPersistence.fs` for pending-save and save coordination.
- Keep `EditorSession` as the orchestration boundary.

Do not convert this directly to event sourcing unless replay, auditing, or persistence becomes a concrete requirement.

### Medium: SettingsView And Settings Are Broad

`SettingsView.axaml.fs` mixes control synchronization, settings editing, theme-file selection, and settings UI lifecycle.

`src/Functor.Application/SettingsDraft.fs` combines the draft record, color parsing, preset application, projection, numeric validation, and final `AppSettings` construction.

Potential future splits:

- `ThemeParsing.fs`
- `ThemeValidation.fs`
- `ThemeProjection.fs`
- A focused settings UI coordinator for control synchronization and theme-file actions.

The settings record itself can remain centralized as the public settings-page contract.

### Medium: ThemeSettingsLoader Is A Schema Parser And Validator

`src/Functor.Application/ThemeSettingsLoader.fs` handles JSON primitives, schema versioning, palette overlays, UI overlays, and validation.

Potential future splits:

- `ThemeJsonPrimitives.fs`
- `ThemePaletteLoader.fs`
- `ThemeUiLoader.fs`
- Keep `ThemeSettingsLoader.fs` as the schema/orchestration entry point.

This is lower priority than `ShellHostView` because the responsibilities remain within one application/settings boundary.

### Medium: Composition Is Hidden In EditorControl

`src/Functor.Avalonia/Controls/EditorControl.fs` constructs the editor session and platform services directly. This makes substitution and focused testing harder.

Introduce a small composition boundary incrementally, for example:

```fsharp
type EditorServices =
    { Clipboard: IClipboardService
      Files: IFileService
      Dialogs: IDialogService
      Tokenizer: ITokenizerService }
```

The desktop/bootstrap layer can construct these services and pass them into the control. Start with the editor session and file/dialog services.

### Medium: Test Coverage Gaps

Add focused tests for:

- Multi-document edit preservation across tab switches.
- Workspace revision guards and stale-result rejection.
- Async workspace-tree refresh ordering.
- Settings apply, save, and failure paths.
- Tokenization cancellation and stale-result rejection.

## Deliberately Not Recommended Yet

### WorkspaceModel

`src/Functor.Workspace/WorkspaceModel.fs` references several domain subdomains because the workspace aggregate must preserve per-document state across tab switches. This coupling is probably intentional.

Keep the workspace as an aggregate/container, keep mutation orchestration in `WorkspaceLogic`, and test revision guards and state preservation. Do not make the per-document state opaque solely to reduce imports.

### LayoutEngine

`src/Functor.Rendering/LayoutEngine.fs` is large but cohesive. It is pure, UI-agnostic, and responsible for converting editor spans into geometry. Split it only if discoverability or ongoing growth becomes a problem.

### Tokenizers

The tokenizers under `Functor.Application/Tokenizers` should not be moved merely to satisfy a layer rule. They depend on application-level tokenizer abstractions and are already isolated behind `ITokenizerService`. Move them only if syntax analysis becomes a domain-owned capability with a stable domain contract.

## MVU Guideline Recommendation

**Status: Addressed in [practical_guidelines.md](../architecture/practical_guidelines.md).**

The current guideline wording is too strict for the practical Avalonia architecture. It says that views must be stateless and contain no mutable fields, but Avalonia controls commonly need local lifecycle state.

The guideline should distinguish three categories:

1. **Application and domain state**
   - Immutable.
   - Model-owned.
   - Updated through explicit messages, commands, or application operations.
   - Never hidden in a view.

2. **View lifecycle state**
   - Allowed inside Avalonia controls.
   - Includes event subscriptions, control references, disposal handles, transient pointer/drag state, refresh guards, and rendering caches.
   - Must not become a second source of truth for application state.

3. **Business state**
   - Must not be stored only in a control.
   - Must be represented in the application/domain model when it affects editor behavior, persistence, commands, or other views.

A more accurate rule is:

> Avalonia views should be thin projections and event adapters. Persistent application state belongs in the Model. View-local mutable state is allowed only for framework lifecycle, transient interaction, synchronization guards, and caches, and must not duplicate authoritative application state.

This preserves MVU's important guarantee without requiring an impractical stateless-control architecture.

## Recommended Order

1. Extract pure workspace-tree and settings logic from `ShellHostView`.
2. Add multi-document, async-refresh, and stale-result tests.
3. Separate tokenization and save coordination from `EditorSession`.
4. Introduce the `EditorControl` composition boundary.
5. Split the settings contract and `ThemeSettingsLoader` if they continue to grow.

Avoid broad rewrites, event sourcing, opaque workspace state, or a ViewModel layer unless a concrete requirement justifies them.
