# Tabbing Improvements

Implementation checklist for language-aware indentation and tab rendering.

## Goal

Treat indentation style, indentation size, and visual tab width as editor settings rather than as a single global theme value.

A literal tab's visual width and the number of spaces inserted by indentation commands are related, but they are not the same setting.

## Current State

- `FileType.languageId` already identifies known languages.
- `UiThemeDefaults.EditorTabSize` is currently one global persisted value used by renderer tab expansion.
- `WorkspaceSettings.IndentSize` exists but is not yet connected to rendering or editing commands.
- There is no language-specific preference map, document override, or `.editorconfig` resolver.

## Required Implementation

### 1. Define the editor settings model

- [ ] Add an `IndentStyle` discriminated union with `Spaces` and `Tabs` cases.
- [ ] Add an `IndentSettings` record containing:
  - [ ] `Style`.
  - [ ] `TabWidth` for visual expansion of literal tab characters.
  - [ ] `IndentSize` for indentation commands and inserted whitespace.
- [ ] Define global editor defaults.
- [ ] Define language-specific overrides keyed by language ID.
- [ ] Keep editor indentation settings separate from `UiThemeDefaults`.

### 2. Define precedence and resolution

- [ ] Resolve settings in this order:
  1. Explicit document override.
  2. Workspace or `.editorconfig` setting.
  3. Language-specific user preference.
  4. Global user preference.
  5. Built-in default.
- [ ] Resolve settings from the active document's language ID and path.
- [ ] Provide a sensible fallback for unknown languages and unsaved documents.
- [ ] Make the resolved settings available as part of the active editor session or editor snapshot.

### 3. Persist user preferences

- [ ] Add global indentation defaults to application settings JSON.
- [ ] Add language-specific overrides to application settings JSON.
- [ ] Add backward-compatible loading for existing settings that only contain `editorTabSize`.
- [ ] Decide whether existing `editorTabSize` migrates to global `TabWidth`, `IndentSize`, or both.
- [ ] Validate positive widths and supported indentation styles.

### 4. Support workspace and `.editorconfig` settings

- [ ] Extend workspace settings with optional indentation overrides.
- [ ] Detect `.editorconfig` from the document or workspace path.
- [ ] Parse the relevant properties: `indent_style`, `indent_size`, and `tab_width`.
- [ ] Apply section and file-pattern matching correctly.
- [ ] Keep parsing isolated from the editor and renderer so it can be tested independently.

### 5. Wire editor behavior

- [ ] Use resolved `TabWidth` when expanding literal tab characters in the renderer.
- [ ] Use resolved `IndentSize` for indent, unindent, newline, and auto-indent commands.
- [ ] Use resolved `IndentStyle` when inserting indentation.
- [ ] Preserve existing tab characters when rendering and editing unless the user explicitly converts indentation.
- [ ] Add commands to convert indentation between tabs and spaces only as a separate, explicit operation.

### 6. Expose settings in the UI

- [ ] Add global indentation controls to editor settings.
- [ ] Add language-specific override controls.
- [ ] Show the effective settings for the active document.
- [ ] Clearly distinguish tab width from indent size.
- [ ] Add document-level actions for temporarily overriding or resetting settings.

### 7. Test the behavior

- [ ] Test language-specific resolution for F#, C#, JSON, Markdown, and unknown files.
- [ ] Test precedence between document, workspace, `.editorconfig`, language, and global settings.
- [ ] Test tabs and spaces independently.
- [ ] Test renderer expansion with different tab widths.
- [ ] Test indentation commands with different indent sizes and styles.
- [ ] Test settings persistence and migration from the existing global editor tab size.
- [ ] Test unsaved documents and documents without a recognized language.
- [ ] Add a desktop smoke test for changing effective settings while switching documents.

## Relationship To Theming

The theming work can be completed without implementing this entire checklist.

The following can continue independently:

- Semantic colors and palette resources.
- UI and editor typography.
- Control geometry and density.
- Document-tab appearance.
- Shell, workspace, command-palette, and dialog styling.
- Renderer color, font, caret, gutter, and cache invalidation work.

The tabbing work should remain separate from visual theme persistence. The renderer may consume resolved tab metrics, but those metrics should come from editor settings rather than `UiThemeDefaults`.

Until this checklist is implemented, retain the existing `EditorTabSize` behavior as a compatibility path. Do not claim language-aware indentation support in the theming checklist until the resolver and editor commands are wired.

## Suggested Slices

1. Add the settings model, resolver, defaults, and unit tests.
2. Wire renderer tab expansion and editing commands.
3. Add persistence and migration.
4. Add workspace and `.editorconfig` support.
5. Add settings UI and effective-setting indicators.
6. Add conversion commands and final desktop verification.
