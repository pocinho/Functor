# Code Review After Theming Improvements

Follow-up roadmap for the architecture and maintainability actions identified after the theming improvements. This document tracks practical refactoring and test work; it does not reopen the bounded Functor-owned theming scope.

## Context

- [x] Complete the Functor-owned editor theming scope.
- [x] Record future or optional theming work in [Theming_Future_Considerations.md](Theming_Future_Considerations.md).
- [x] Clarify practical MVU boundaries in [practical_guidelines.md](../architecture/practical_guidelines.md).
- [x] Mark the MVU guideline concern as addressed in [Recommended_Code_Review_Actions.md](../notes/Recommended_Code_Review_Actions.md).

## Phase 1: Shell And Workspace Coordination

- [x] Extract pure shell projection helpers from `ShellHostView.axaml.fs` into `ShellHostProjection.fs`.
- [x] Extract workspace-tree loading, keys, generation checks, and refresh coordination into `WorkspaceTreeCoordinator.fs`.
- [x] Extract settings editing, apply, save, and export coordination from `ShellHostView.axaml.fs`.
- [x] Keep `ShellHostView` focused on Avalonia lifecycle, control wiring, and event adaptation.
- [x] Add focused tests for workspace-tree refresh ordering and stale-result rejection.

## Phase 2: Editor Session Boundaries

- [x] Add focused tests for multi-document edit preservation across tab switches.
- [x] Add focused tests for tokenization cancellation and stale-result rejection.
- [x] Extract pure model and event transitions into `EditorSessionUpdate.fs`.
- [x] Extract scheduling, cancellation, and incremental tokenization into `EditorSessionTokenization.fs`.
- [x] Extract pending-save and save coordination into `EditorSessionPersistence.fs`.
- [x] Keep `EditorSession` as the application orchestration boundary.

## Phase 3: Composition And Settings Boundaries

- [x] Introduce an `EditorServices` composition record for clipboard, file, dialog, and tokenizer dependencies.
- [x] Move service construction toward the desktop/bootstrap composition boundary.
- [x] Add settings apply, save, and failure-path tests.
- [x] Split the settings contract into parsing, validation, and projection modules as its responsibilities grew.
- [x] Split `ThemeSettingsLoader` into schema, palette, and UI loaders as its responsibilities became distinct.

## Deferred Structural Work

- [ ] Reassess `WorkspaceModel` only if aggregate coupling creates a concrete ownership or testability problem.
- [ ] Reassess `LayoutEngine` only if its cohesion or discoverability degrades.
- [ ] Reassess tokenizer placement only if syntax analysis gains a stable domain-owned contract.
- [ ] Do not introduce event sourcing, opaque workspace state, or a ViewModel layer without a concrete requirement.

## Completion Criteria

- [x] High-priority orchestration modules have focused ownership and tests.
- [x] Async operations reject stale results at the relevant boundaries.
- [x] Editor services can be substituted without constructing platform implementations inside reusable controls.
- [x] Settings failure paths are covered.
- [x] The practical MVU boundary remains consistent: authoritative state is model-owned, while bounded framework-local lifecycle state remains allowed.

## Source Notes

- Review findings: [Recommended_Code_Review_Actions.md](../notes/Recommended_Code_Review_Actions.md)
- Practical architecture guidance: [practical_guidelines.md](../architecture/practical_guidelines.md)
- Deferred theming scope: [Theming_Future_Considerations.md](Theming_Future_Considerations.md)
