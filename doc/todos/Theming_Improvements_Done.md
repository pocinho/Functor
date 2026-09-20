# Theming Improvements

Implementation checklist for making Functor's visual styling fully dynamic and centrally controlled.

> Existing colors, fonts, sharp corners, dimensions, and platform fallbacks remain supported but should be themeable.

## User-Editable Scope

Theme persistence should serve Functor's editor requirements, not expose Avalonia as a general-purpose theme editor.

### Required User Customization

- [x] Editor palette: surfaces, foreground text, selection, cursor, line numbers, gutter, diagnostics, editor border, and named syntax colors.
- [x] Editor typography: editor font, UI font, fallback fonts, editor size, line height, tab size, workspace size, and command-palette size.
- [x] Editor ergonomics: cursor width, gutter padding and minimum width, document-tab sizing, and tab spacing.
- [x] Command palette: width, maximum height, padding, item spacing, gesture-column spacing, overlay offset, shadow, and related semantic colors.
- [x] Theme presets and shareable theme files preserve all required user-editable values.

Optional density, platform-specific, FluentTheme, and structural-shell work is tracked in [Theming_Future_Considerations.md](Theming_Future_Considerations.md). The persisted schema contains stable Functor concepts, not a general-purpose Avalonia theme description.

## Progress

- [x] Added centralized `UiThemeDefaults` for current fonts, sizes, colors, shadow, separator, and corner-radius defaults.
- [x] Published initial `Theme.*` Avalonia resources and refresh them when application settings are applied.
- [x] Migrated initial XAML values for control corner radius, icon font, command-palette font size, welcome title size, and resize-handle color.
- [x] Migrated tab-close icons, workspace typography defaults, workspace separator color, and editor font defaults to the centralized defaults model.
- [x] Migrated the command-palette shadow geometry and color to a dynamic `BoxShadows` resource.
- [x] Persisted centralized UI fonts, metrics, corner radius, and visual colors through the existing theme JSON format with backward-compatible defaults.
- [x] Added application-level round-trip coverage for non-default UI theme values.
- [x] Added configuration-page fields and validation for every current `UiThemeDefaults` value.
- [x] Added configuration-page fields, persistence, and round-trip tests for the six named syntax color roles.
- [x] Added explicit validation for persisted theme preset names.
- [x] Load valid `.functortheme` files from `%APPDATA%/Functor/themes` and list them beside the built-in Graphite themes.
- [x] Add named theme export with validation that prevents overwriting an existing built-in or user theme.
- [x] Project settings control width through the shared layout-density resources while preserving the 200px default.
- [x] Project settings color-picker width and height through shared layout-density resources while preserving the 64x32 default.
- [x] Project settings-row bottom spacing through a shared layout-density resource while preserving the 8px default.
- [x] Project settings page padding and spacing through shared layout-density resources while preserving the 16px and 4px defaults.
- [x] Project settings section-heading margins through shared layout-density resources while preserving the existing 0/12px top and 8px bottom spacing.
- [x] Separated the general UI font from the monospace editor font, including settings and persistence.
- [x] Complete the initial runtime propagation for generated tabs, workspace rows, and the editor renderer so custom `UiThemeDefaults` values update existing controls.
- [x] Added shared palette brushes for application surfaces, selection, borders, and focus states, with initial Button, TextBox, and ComboBox overrides.
- [x] Applied semantic surface, input, border, and text resources to normal Button, TextBox, ComboBox, NumericUpDown, and ListBox states.
- [x] Added a validated, persisted cursor-width setting and wired it through the settings page and renderer.
- [x] Added a validated, persisted gutter-separator-width setting and wired it through the settings page and renderer.
- [x] Presented settings as a shell tab document with inline Apply, Save, and Close actions; removed the floating settings window.
- [x] Applied the active theme to the unsaved-changes confirmation window.
- [x] Added a shared themed dialog-window path with consistent chrome, borders, and title-bar dragging; see [dialog findings](../notes/Dialog_Findings.md).
- [x] Batched Avalonia theme-resource updates to reduce repeated resource propagation; see [Avalonia optimization findings](../notes/Avalonia_Optimization.md).
- [x] Matched Avalonia's client-area title-bar hint to the shared 32-pixel custom chrome.
- [x] Bound the native client-area title-bar hint to the shared dynamic title-bar resource.
- [x] Made the editor caret visible only while the editor is focused, with redraws on focus transitions.
- [x] Added a selected document-tab style so active tabs use the semantic selection and focus colors.

## Findings

- [x] Treat the existing Graphite Light and Graphite Dark values as named presets rather than scattered styling literals.
- [x] Extend theme persistence to cover the six named syntax colors exposed by the rendering palette.

## Theme Model

- [x] Define semantic typography tokens for UI and editor fonts, fallback fonts, sizes, and weights through `ThemeTypography` resource projection.
- [x] Project existing document-tab geometry, separator thickness, and corner radius through `ThemeShapeDensity`; spacing and padding remain separate density work.
- [x] Project document-tab padding, spacing, and border width through `ThemeShapeDensity` and dynamic resources.
- [x] Project document-tab label/close-button content spacing through `ThemeShapeDensity`.
- [x] Project tab overflow navigation button width through `ThemeShapeDensity` and dynamic resources.
- [x] Keep document-tab overflow arrows at the visible edges of the constrained tab viewport.
- [x] Project workspace-tree indentation and collapse-button size through `ThemeShapeDensity`.
- [x] Project workspace explorer/editor/tree row spacing through `ThemeShapeDensity`.
- [x] Project shared title-bar, window-control, and command-bar geometry through `ThemeShapeDensity` and dynamic resources.
- [x] Project title-bar window-control padding through `ThemeShapeDensity` and dynamic resources.
- [x] Keep the main window and title-bar center column wide enough for the command bar and window controls.
- [x] Project side-panel resize-handle width and padding through `ThemeShapeDensity` and dynamic resources.
- [x] Project command-palette sizing, overlay offset, padding, item spacing, and gesture-column gap through `ThemeShapeDensity`.
- [x] Project shell tab-bar, tool-rail, settings-footer, and status-bar spacing through `ThemeLayoutDensity`.
- [x] Project confirmation-dialog content and button spacing through `ThemeLayoutDensity`.
- [x] Project welcome-view width, section spacing, action spacing, and muted text opacity through shared resources.
- [x] Support custom themes by overlaying user-supplied values on a selected preset without losing unspecified defaults.
- [x] Add a theme schema version to persisted settings; warn and skip unversioned themes so future migrations are explicit.

## Avalonia Integration

- [x] Add an Avalonia-side `ThemeManager` responsible for applying a complete theme snapshot.
- [x] Convert semantic theme values into shared Avalonia resources (`IBrush`, `FontFamily`, numeric values, and related objects).
- [x] Register resource keys such as:
  - [x] `Theme.SurfaceBackground`.
  - [x] `Theme.PanelBackground`.
  - [x] `Theme.InputBackground`.
  - [x] `Theme.TextPrimary`.
  - [x] `Theme.TextMuted`.
  - [x] `Theme.TextMutedOpacity`.
  - [x] `Theme.Border`.
  - [x] `Theme.Separator`.
  - [x] `Theme.Accent`.
  - [x] `Theme.Error`.
  - [x] `Theme.ResizeHandle`.
  - [x] `Theme.Shadow`.
  - [x] `Theme.ControlCornerRadius`.
  - [x] `Theme.ControlFontFamily`.
- [x] Replace direct `SolidColorBrush` construction in view code with shared theme resources or a single theme helper, while preserving the current default brushes.
- [x] Add centralized styles for `Button`, `TextBox`, `ComboBox`, `ListBox`, `TabControl`, `Menu`, `MenuItem`, `ScrollBar`, and `Window`.
- [x] Style normal, pointer-over, pressed, focused, disabled, and validation states from semantic tokens.
- [x] Apply the sharp-corner preference through the shared corner-radius resource instead of per-control literals.
- [x] Theme popup, menu, tooltip, scrollbar, and focus visuals, which currently remain closest to FluentTheme defaults.

## Renderer Integration

- [x] Pass a complete theme snapshot into `RenderingSurface`.
- [x] Move editor font family, emoji fallback, and tab size into the renderer theme.
- [x] Move gutter padding, minimum width, and line-number positioning into the renderer theme.
- [x] Replace renderer-local font constants with theme values initialized from the current editor font defaults.
- [x] Replace renderer-local measurement colors with a theme-independent or theme-provided measurement brush while preserving measurement behavior.
- [x] Invalidate default-advance and grapheme measurement caches when font family, fallback font, font size, or tab size changes.
- [x] Trigger editor redraw when the active theme changes.
- [x] Use the active editor tab size for both renderer measurement and tab expansion, with loader validation for non-positive values.

## Settings And Update Flow

- [x] Extend `ThemeSettingsLoader` and `AppSettingsLoader` to load and save all user-editable Functor theme tokens; keep internal Avalonia styling out of the persisted schema.
- [x] Expose the persisted editor gutter padding and minimum-width settings in the configuration page.
- [x] Expose persisted document-tab height, close-button size, padding, and spacing settings in the configuration page.
- [x] Validate color, font, numeric, and enum values with clear errors.
- [x] Define the update flow:

      Settings changed
          -> AppSettings
          -> ThemeManager.Apply
          -> Application resources updated
          -> DynamicResource-bound controls refresh
          -> Renderer invalidates caches and redraws

- [x] Ensure theme changes update open windows, command palette, tabs, shell panels, settings, and editor surfaces consistently; tabs, workspace panels, editor surfaces, and shared dialog chrome are covered.
- [x] Ensure the selected light/dark preset updates `RequestedThemeVariant` without overriding custom semantic colors.
- [x] Avoid duplicating theme application logic across `MainWindow`, `ShellHostView`, `SettingsView`, `SettingsWindow`, and `CommandPaletteView`.

## Migration Order

- [x] Add persisted schema support and loader tests for versioning and syntax colors.
- [x] Implement `ThemeManager` and application resources.
- [x] Migrate `AvaloniaApp.axaml` control styles and Fluent state overrides.
- [x] Migrate `MainWindow` and shell-level brushes.
- [x] Migrate tabs, workspace tree, panels, status bar, welcome view, settings, and command palette.
- [x] Migrate editor renderer fonts, measurements, and geometry-related visual tokens.
- [x] Add live theme application and redraw/cache invalidation.
- [x] Review remaining literals and classify each as theme, density, geometry, content, or platform behavior.

## Verification

- [x] Test Graphite Light and Graphite Dark preset loading.
- [x] Test custom colors and syntax colors round-trip through JSON persistence.
- [x] Test invalid theme values and rejection of unversioned settings files.
- [x] Test runtime light/dark switching.
- [x] Test runtime color, font, and corner-radius changes.
- [x] Test that the semantic control resource contract is published during runtime theme application.
- [x] Test renderer measurement-cache invalidation after font changes. See [renderer cache findings](../notes/Theming_Renderer_Cache.md).
- [x] Verify that batched theme-resource updates preserve runtime resource and dialog refresh behavior.
- [x] Search for remaining hardcoded colors, fonts, and visual constants and classify intentional exceptions. Remaining transparent menu chrome is structural; renderer-local brushes are required for canvas drawing.
