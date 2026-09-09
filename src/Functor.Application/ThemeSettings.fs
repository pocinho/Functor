namespace Functor.Application

open Functor.Rendering

/// Application-level editor theme settings.
/// This keeps theme selection outside the rendering pipeline but still makes the
/// palette source configurable without hard-coding a single default.
type ThemeSettings =
    { ThemeSource: ThemeSource }

module ThemeSettings =
    let defaultTheme =
        { ThemeSource = Theme.defaultSource }

    let fromSource (themeSource: ThemeSource) =
        { ThemeSource = themeSource }

    let fromPalette (palette: ThemePalette) =
        { ThemeSource = ThemeSource.fixedPalette palette }
