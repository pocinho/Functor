namespace Functor.Application

open Functor.Rendering

module ThemePreset =
    [<Literal>]
    let GraphiteDark = "Graphite Dark"

    [<Literal>]
    let GraphiteLight = "Graphite Light"

    [<Literal>]
    let Custom = "Custom"

    let all = [ GraphiteDark; GraphiteLight ]
    let valid = GraphiteDark :: GraphiteLight :: [ Custom ]

    let palette preset =
        if preset = GraphiteLight then
            Theme.graphiteLight
        else
            Theme.defaultPalette

/// Application-level editor theme settings.
/// This keeps theme selection outside the rendering pipeline but still makes the
/// palette source configurable without hard-coding a single default.
type ThemeSettings =
    { ThemeSource: ThemeSource
      Preset: string
      Ui: UiThemeDefaults }

module ThemeSettings =
    let defaultTheme =
        { ThemeSource = Theme.defaultSource
          Preset = ThemePreset.GraphiteDark
          Ui = UiThemeDefaults.defaultTheme }

    let fromSource (themeSource: ThemeSource) =
        { ThemeSource = themeSource
          Preset = ThemePreset.Custom
          Ui = UiThemeDefaults.defaultTheme }

    let fromPalette (palette: ThemePalette) =
        { ThemeSource = ThemeSource.fixedPalette palette
          Preset = ThemePreset.Custom
          Ui = UiThemeDefaults.defaultTheme }

    let fromPaletteWithPreset preset palette =
        { ThemeSource = ThemeSource.fixedPalette palette
          Preset = preset
          Ui = UiThemeDefaults.defaultTheme }

    let fromPaletteWithPresetAndUi preset palette ui =
        { ThemeSource = ThemeSource.fixedPalette palette
          Preset = preset
          Ui = ui }
