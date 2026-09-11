namespace Functor.Application

open Functor.Rendering

/// Application-level editor theme settings.
/// This keeps theme selection outside the rendering pipeline but still makes the
/// palette source configurable without hard-coding a single default.
type ThemeSettings =
    { ThemeSource: ThemeSource;
            Preset: string }

module ThemeSettings =
    let defaultTheme =
        { ThemeSource = Theme.defaultSource; Preset = "Graphite Dark" }

    let fromSource (themeSource: ThemeSource) =
        { ThemeSource = themeSource; Preset = "Custom" }

    let fromPalette (palette: ThemePalette) =
        { ThemeSource = ThemeSource.fixedPalette palette; Preset = "Custom" }

    let fromPaletteWithPreset preset palette =
        { ThemeSource = ThemeSource.fixedPalette palette; Preset = preset }
