namespace Functor.Application

open Functor.Rendering

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
          Ui = ThemePreset.ui ThemePreset.GraphiteDark }

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
          Ui = ThemePreset.ui preset }

    let fromPaletteWithPresetAndUi preset palette ui =
        { ThemeSource = ThemeSource.fixedPalette palette
          Preset = preset
          Ui = ui }
