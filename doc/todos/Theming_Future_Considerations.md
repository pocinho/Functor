# Theming Future Considerations

This document holds theming work that is intentionally outside Functor's current editor-theme contract. Revisit an item only when a concrete editor, accessibility, platform, or maintenance requirement justifies it.

## Scope Boundary

Functor's persisted theme schema should contain Functor concepts, not a general-purpose Avalonia theme description. FluentTheme templates, generic control states, popup chrome, scrollbars, native title-bar behavior, and structural shell constraints remain implementation details unless a specific editor requirement promotes them into the public contract.

## Optional Functor-Owned Features

- Optional density settings for workspace-tree indentation, row spacing, title-bar geometry, welcome spacing, and dialog spacing.
- Additional platform font overrides beyond the current editor/UI/fallback font settings.
- Per-preset typography and geometry when Graphite Light and Graphite Dark need meaningfully different complete definitions.
- Additional accessibility-oriented geometry or contrast controls backed by a concrete user workflow.

## Avalonia Integration And Cleanup

- Replace remaining direct XAML colors and dimensions with `DynamicResource` references where they represent genuine theme tokens.
- Decide whether to expand the semantic model to cover additional shape, border, control-height, spacing, and padding tokens.
- Remove duplicate or local styling definitions after confirming they are not deliberate structural, content, platform, or renderer exceptions.
- Replace remaining mixed styling sources only where doing so improves consistency without turning Functor into an Avalonia theme editor.
- Review transparent backgrounds and other visual polish that does not affect the current editor customization contract.
- Verify that every application surface uses the same semantic tokens.

## FluentTheme And Desktop Verification

- Verify hover, pressed, focused, disabled, popup, menu, scrollbar, and tooltip states through desktop smoke tests.
- Confirm FluentTheme templates continue to provide behavior while application-level resources provide Functor-owned visuals.
- Run desktop smoke tests across the supported frontends and platforms when a theme migration affects platform behavior.

## Possible Future Schema Work

Only introduce these into persisted settings if a concrete requirement appears:

- More workspace and shell density controls.
- Platform-specific typography overrides.
- Additional semantic colors or control-state tokens.
- Per-preset geometry or typography overrides.

Any schema expansion should preserve versioning, backward-compatible defaults, validation, and export/import round trips.
