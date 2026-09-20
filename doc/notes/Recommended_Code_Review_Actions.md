# Recommended Code Review Actions

## Review Status

The high-priority actions in this review have been completed. Shell coordination, editor-session responsibilities, settings boundaries, theme loading, service composition, and the associated focused tests are now implemented. The remaining items below are conditional follow-ups and should only be reopened when concrete coupling, cohesion, or product requirements justify them.

## Review Context

The current codebase has a sound pure-domain and rendering foundation. The main maintainability risk is orchestration concentration in the application coordinator and Avalonia shell, not widespread impurity in domain logic.

## Findings By Priority

### High: ShellHostView Is a God Module

`src/Functor.Avalonia/Views/ShellHostView.axaml.fs` coordinates shell projection, workspace-tree loading and invalidation, settings application and export, dialogs, tabs, scrollbars, tool panels, and command routing.

Completed decomposition:

- `ShellHostProjection.fs` for pure model-to-control projection helpers.
- `WorkspaceTreeCoordinator.fs` for workspace keys, loading, generation checks, and refresh.
- `SettingsCoordinator.fs` for settings validation, apply/save, and export callbacks.
- Keep `ShellHostView` as the Avalonia lifecycle and event adapter.

This refactor improved testability without requiring an MVU rewrite.

### High: EditorSession Has Too Many Coordinator Responsibilities

`src/Functor.Application/EditorSession.fs` is an application coordinator, so its mutable state is not a Domain purity violation. However, it combines model updates, event publication, effect requests, tokenization scheduling, cancellation, incremental tokenization, pending-save tracking, and confirmation workflows.

Completed extraction:

- `EditorSessionUpdate.fs` for pure model and event transitions.
- `EditorSessionTokenization.fs` for scheduling, cancellation, and incremental tokenization.
- `EditorSessionPersistence.fs` for pending-save and save coordination.
- Keep `EditorSession` as the orchestration boundary.

Do not convert this directly to event sourcing unless replay, auditing, or persistence becomes a concrete requirement.

### Medium: SettingsView And Settings Are Broad

`SettingsView.axaml.fs` mixes control synchronization, settings editing, theme-file selection, and settings UI lifecycle.

The settings form modules combine the editable values, color parsing, preset application, projection, numeric validation, and final `AppSettings` construction.

Completed decomposition:

- `SettingsFormModel.fs`, `SettingsFormValidation.fs`, and `SettingsFormProjection.fs` separate the settings contract responsibilities.
- `SettingsCoordinator.fs` owns settings validation, apply/save, and export coordination.

The settings form model remains centralized as the public settings-page contract.

### Medium: ThemeSettingsLoader Is A Schema Parser And Validator

`src/Functor.Application/ThemeSettingsLoader.fs` handles JSON primitives, schema versioning, palette overlays, UI overlays, and validation.

Completed decomposition:

- `ThemeSettingsJson.fs` owns JSON primitives and schema concerns.
- `ThemeSettingsPalette.fs` owns palette loading.
- `ThemeSettingsUi.fs` owns UI loading.
- `ThemeSettingsLoader.fs` remains the schema/orchestration entry point.

This is lower priority than `ShellHostView` because the responsibilities remain within one application/settings boundary.

### Medium: Composition Is Hidden In EditorControl

`src/Functor.Avalonia/Controls/EditorControl.fs` constructs the editor session and platform services directly. This makes substitution and focused testing harder.

Completed composition boundary:

```fsharp
type EditorServices =
    { Clipboard: IClipboardService
      Files: IFileService
      Dialogs: IDialogService
      Tokenizer: ITokenizerService }
```

The desktop/bootstrap layer constructs these services and passes them into the control, allowing reusable controls and tests to substitute platform dependencies.

### Medium: Test Coverage Gaps

Focused tests now cover:

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

1. Reassess `WorkspaceModel` only if aggregate coupling creates a concrete ownership or testability problem.
2. Reassess `LayoutEngine` only if its cohesion or discoverability degrades.
3. Reconsider tokenizer placement only if syntax analysis gains a stable domain-owned contract.

Avoid broad rewrites, event sourcing, opaque workspace state, or a ViewModel layer unless a concrete requirement justifies them.
