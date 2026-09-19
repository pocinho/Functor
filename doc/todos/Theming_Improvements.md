# Theming Improvements

Implementation checklist for making Functor's visual styling fully dynamic and centrally controlled.

> Existing colors, fonts, sharp corners, dimensions, and platform fallbacks remain supported but should be themeable.

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
- [x] Complete the initial runtime propagation for generated tabs, workspace rows, and the editor renderer so custom `UiThemeDefaults` values update existing controls.
- [x] Added shared palette brushes for application surfaces, selection, borders, and focus states, with initial Button, TextBox, and ComboBox overrides.
- [x] Added a validated, persisted cursor-width setting and wired it through the settings page and renderer.
- [x] Added a validated, persisted gutter-separator-width setting and wired it through the settings page and renderer.
- [x] Presented settings as a shell tab document with inline Apply, Save, and Close actions; removed the floating settings window.

## Findings

- [ ] Replace the mixed styling sources: FluentTheme defaults, direct F# brush assignments, XAML literals, and renderer-local constants.
- [ ] Treat the existing Graphite Light and Graphite Dark values as named presets rather than scattered styling literals.
- [ ] Centralize the existing color defaults in the theme model while preserving their current appearance:
  - [ ] Side-panel resize handle color `#DCDC3C3C`.
  - [x] Command-palette shadow color `#66000000`.
  - [x] Workspace tree separator color `ARGB(110,160,160,160)`.
  - [ ] Transparent backgrounds where theme-controlled surfaces are more appropriate.
  - [x] Renderer fallback `Brushes.White` used for text measurement.
- [ ] Centralize the existing font defaults in the theme model while preserving platform-specific fallbacks:
  - [x] Editor font `Consolas`.
  - [x] Windows emoji fallback `Segoe UI Emoji`.
  - [x] Window-control and tab-close icon font `Segoe MDL2 Assets`.
- [ ] Centralize typography defaults: editor font-size calculation, workspace font size `12`, command-palette font size `14`, welcome-title font size `28`, and bold/semi-bold weights.
- [ ] Centralize shape and effect defaults: zero corner radius, remaining border widths, command-palette shadow, and control dimensions.
- [ ] Centralize layout defaults that may become density settings: padding, margins, spacing, tab height, tree indentation, title-bar height, and control sizes.
- [ ] Keep purely structural layout values local unless there is a clear user-facing density or accessibility requirement.
- [x] Extend theme persistence to cover the six named syntax colors exposed by the rendering palette.

## Theme Model

- [ ] Introduce a complete semantic theme model, separate from view-specific control code.
- [ ] Define semantic color tokens:
  - [ ] `SurfaceBackground`.
  - [ ] `PanelBackground`.
  - [ ] `InputBackground`.
  - [ ] `TextPrimary`.
  - [ ] `TextMuted`.
  - [ ] `Border`.
  - [ ] `Separator`.
  - [ ] `Accent`.
  - [ ] `Selection`.
  - [ ] `Hover`.
  - [ ] `Pressed`.
  - [ ] `Disabled`.
  - [ ] `Focus`.
  - [ ] `Error`.
  - [ ] `Warning`.
  - [ ] `Information`.
  - [ ] `ResizeHandle`.
  - [ ] `Shadow`.
- [ ] Define semantic typography tokens for UI and editor fonts, fallback fonts, sizes, and weights.
- [ ] Define shape and density tokens for corner radius, border thickness, control height, spacing, and padding.
- [ ] Preserve Graphite Light and Graphite Dark as complete preset definitions, including the current custom fonts, colors, sharp corners, and sizing defaults.
- [ ] Support custom themes by overlaying user-supplied values on a selected preset without losing unspecified defaults.
- [ ] Preserve platform-specific defaults, such as the Windows icon font and emoji fallback, while allowing them to be overridden.
- [x] Add a theme schema version to persisted settings; warn and skip unversioned themes so future migrations are explicit.

## Avalonia Integration

- [x] Add an Avalonia-side `ThemeManager` responsible for applying a complete theme snapshot.
- [x] Convert semantic theme values into shared Avalonia resources (`IBrush`, `FontFamily`, numeric values, and related objects).
- [ ] Register resource keys such as:
  - [ ] `Theme.SurfaceBackground`.
  - [ ] `Theme.PanelBackground`.
  - [ ] `Theme.InputBackground`.
  - [ ] `Theme.TextPrimary`.
  - [ ] `Theme.TextMuted`.
  - [ ] `Theme.Border`.
  - [ ] `Theme.Accent`.
  - [ ] `Theme.Error`.
  - [ ] `Theme.ControlCornerRadius`.
  - [ ] `Theme.ControlFontFamily`.
- [ ] Replace direct XAML colors and dimensions with `DynamicResource` references where they represent theme tokens, using centralized defaults as the initial resource values.
- [x] Replace direct `SolidColorBrush` construction in view code with shared theme resources or a single theme helper, while preserving the current default brushes.
- [ ] Keep `FluentTheme` for control templates and behavior, while overriding its visual resources at the application level.
- [x] Add centralized styles for `Button`, `TextBox`, `ComboBox`, `ListBox`, `TabControl`, `Menu`, `MenuItem`, `ScrollBar`, and `Window`.
- [x] Style normal, pointer-over, pressed, focused, disabled, and validation states from semantic tokens.
- [x] Apply the sharp-corner preference through the shared corner-radius resource instead of per-control literals.
- [x] Theme popup, menu, tooltip, scrollbar, and focus visuals, which currently remain closest to FluentTheme defaults.

## Renderer Integration

- [ ] Pass a complete theme snapshot into `RenderingSurface`.
- [ ] Move editor font family, emoji fallback, font size, and tab size into the renderer theme.
- [x] Move gutter padding, minimum width, and line-number positioning into the renderer theme.
- [x] Replace renderer-local font constants with theme values initialized from the current editor font defaults.
- [x] Replace renderer-local measurement colors with a theme-independent or theme-provided measurement brush while preserving measurement behavior.
- [x] Invalidate default-advance and grapheme measurement caches when font family, fallback font, font size, or tab size changes.
- [x] Trigger editor redraw when the active theme changes.
- [x] Use the active editor tab size for both renderer measurement and tab expansion, with loader validation for non-positive values.

## Settings And Update Flow

- [ ] Extend `ThemeSettingsLoader` and `AppSettingsLoader` to load and save all theme tokens.
- [ ] Add settings UI for future density values; current UI colors, syntax colors, fonts, and corner radius are exposed.
- [ ] Validate color, font, numeric, and enum values with clear errors.
- [ ] Define the update flow:

      Settings changed
          -> AppSettings
          -> ThemeManager.Apply
          -> Application resources updated
          -> DynamicResource-bound controls refresh
          -> Renderer invalidates caches and redraws

- [ ] Ensure theme changes update open windows, command palette, tabs, shell panels, settings, and editor surfaces consistently; tabs, workspace panels, and editor surfaces are now covered.
- [x] Ensure the selected light/dark preset updates `RequestedThemeVariant` without overriding custom semantic colors.
- [x] Avoid duplicating theme application logic across `MainWindow`, `ShellHostView`, `SettingsView`, `SettingsWindow`, and `CommandPaletteView`.

## Migration Order

- [ ] Add the semantic theme model and complete Graphite presets.
- [x] Add persisted schema support and loader tests for versioning and syntax colors.
- [x] Implement `ThemeManager` and application resources.
- [x] Migrate `AvaloniaApp.axaml` control styles and Fluent state overrides.
- [ ] Migrate `MainWindow` and shell-level brushes.
- [ ] Migrate tabs, workspace tree, panels, status bar, welcome view, settings, and command palette.
- [ ] Migrate editor renderer fonts, measurements, and geometry-related visual tokens.
- [ ] Remove only duplicate/local definitions after their values have been moved into centralized defaults; retain the existing visual defaults and customization points.
- [ ] Add live theme application and redraw/cache invalidation.
- [ ] Review remaining literals and classify each as theme, density, geometry, content, or platform behavior.

## Verification

- [ ] Test Graphite Light and Graphite Dark preset loading.
- [ ] Test custom colors and syntax colors round-trip through JSON persistence.
- [x] Test invalid theme values and rejection of unversioned settings files.
- [ ] Test runtime light/dark switching.
- [ ] Test runtime color, font, and corner-radius changes.
- [ ] Test renderer measurement-cache invalidation after font changes.
- [ ] Test Fluent control hover, pressed, focused, disabled, popup, and menu states.
- [ ] Verify all application surfaces use the same semantic tokens.
- [ ] Search for remaining hardcoded colors, fonts, and visual constants and classify intentional exceptions.
- [ ] Build and run the desktop application after each migration stage.
