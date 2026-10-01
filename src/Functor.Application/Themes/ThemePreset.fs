namespace Functor.Application

open Functor.Rendering

module ThemePreset =
    [<Literal>]
    let GraphiteDark = "Graphite Dark"

    [<Literal>]
    let GraphiteLight = "Graphite Light"

    [<Literal>]
    let Custom = "Custom"

    let all = [ GraphiteDark; GraphiteLight; Custom ]
    let valid = GraphiteDark :: GraphiteLight :: [ Custom ]

    let palette preset =
        if preset = GraphiteLight then
            Theme.graphiteLight
        else
            Theme.defaultPalette

    let ui preset =
        if preset = GraphiteDark then
            { UiThemeDefaults.defaultTheme with
                ResizeHandleColor = 0xFF333333u }
        elif preset = GraphiteLight then
            { UiThemeDefaults.defaultTheme with
                ResizeHandleColor = 0xFF4E3A78u }
        else
            UiThemeDefaults.defaultTheme
