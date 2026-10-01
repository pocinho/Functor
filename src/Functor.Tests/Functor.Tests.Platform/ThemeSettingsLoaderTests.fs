namespace Functor.Tests.Platform

open Functor.Application
open Functor.Platform
open Functor.Rendering
open Xunit


type ThemeSettingsLoaderTests() =
    [<Fact>]
    member _.``loads the graphite light preset``() =
        let result =
            ThemeSettingsLoader.loadText "{ \"theme\": { \"schemaVersion\": 1, \"preset\": \"Graphite Light\" } }"

        let settings =
            match result with
            | Ok value -> value
            | Error error -> failwith error

        Assert.Equal("Graphite Light", settings.Preset)
        Assert.Equal(Theme.graphiteLight.Background, settings.ThemeSource.Resolve().Background)

    [<Fact>]
    member _.``loads the graphite dark preset by default``() =
        let result = ThemeSettingsLoader.loadText "{ \"theme\": { \"schemaVersion\": 1 } }"

        let settings =
            match result with
            | Ok value -> value
            | Error error -> failwith error

        Assert.Equal("Graphite Dark", settings.Preset)
        Assert.Equal(Theme.dark.Background, settings.ThemeSource.Resolve().Background)

    [<Fact>]
    member _.``loads the explicit graphite dark preset``() =
        let result =
            ThemeSettingsLoader.loadText "{ \"theme\": { \"schemaVersion\": 1, \"preset\": \"Graphite Dark\" } }"

        let settings =
            match result with
            | Ok value -> value
            | Error error -> failwith error

        Assert.Equal("Graphite Dark", settings.Preset)
        Assert.Equal(Theme.dark.Foreground, settings.ThemeSource.Resolve().Foreground)

    [<Fact>]
    member _.``rejects unknown theme presets``() =
        let result =
            ThemeSettingsLoader.loadText "{ \"theme\": { \"schemaVersion\": 1, \"preset\": \"Unknown\" } }"

        Assert.Equal(Error "Invalid theme preset: Unknown", result)

    [<Fact>]
    member _.``loads nested theme colors and border settings``() =
        let json =
            """
            {
              "theme": {
                                "schemaVersion": 1,
                "background": "#202020",
                "foreground": "#F0F0F0",
                "editorBorder": "#555555",
                "editorBorderWidth": 2.0
              }
            }
            """

        let result = ThemeSettingsLoader.loadText json

        let palette =
            match result with
            | Ok settings -> settings.ThemeSource.Resolve()
            | Error error -> failwith error

        Assert.Equal(0xFF202020u, palette.Background)
        Assert.Equal(0xFFF0F0F0u, palette.Foreground)
        Assert.Equal(Some 0xFF555555u, palette.EditorBorder)
        Assert.Equal(2.0f, palette.EditorBorderWidth)

    [<Fact>]
    member _.``custom theme overlays preserve unspecified preset values``() =
        let result =
            ThemeSettingsLoader.loadText
                "{ \"theme\": { \"schemaVersion\": 1, \"preset\": \"Custom\", \"foreground\": \"#FFF0F0F0\" } }"

        let palette =
            match result with
            | Ok settings -> settings.ThemeSource.Resolve()
            | Error error -> failwith error

        Assert.Equal(0xFFF0F0F0u, palette.Foreground)
        Assert.Equal(Theme.defaultPalette.Background, palette.Background)
        Assert.Equal(Theme.defaultPalette.Selection, palette.Selection)

    [<Fact>]
    member _.``loads colors without a leading hash``() =
        let result =
            ThemeSettingsLoader.loadText "{ \"schemaVersion\": 1, \"background\": \"FF010203\" }"

        let palette =
            match result with
            | Ok settings -> settings.ThemeSource.Resolve()
            | Error error -> failwith error

        Assert.Equal(0xFF010203u, palette.Background)

    [<Fact>]
    member _.``invalid json returns an error``() =
        let result = ThemeSettingsLoader.loadText "{ invalid"

        Assert.True(result |> Result.isError)

    [<Fact>]
    member _.``rejects legacy themes with a warning``() =
        let result =
            ThemeSettingsLoader.loadText "{ \"theme\": { \"preset\": \"Graphite Light\" } }"

        Assert.Equal(Error "Theme schema warning: missing schemaVersion; theme was not loaded", result)

    [<Fact>]
    member _.``rejects malformed schema versions``() =
        let result =
            ThemeSettingsLoader.loadText "{ \"theme\": { \"schemaVersion\": \"1\" } }"

        Assert.Equal(Error "Invalid theme schema version: expected an integer", result)

    [<Fact>]
    member _.``rejects unsupported future schema versions``() =
        let result = ThemeSettingsLoader.loadText "{ \"theme\": { \"schemaVersion\": 2 } }"

        Assert.Equal(Error "Unsupported theme schema version: 2", result)

    [<Fact>]
    member _.``application settings serialize and load theme values``() =
        let settings =
            { Theme.defaultPalette with
                Background = 0xFF101112u
                Foreground = 0xFFFAFBFCu
                EditorBorder = Some 0xFF333435u
                EditorBorderWidth = 3.0f }
            |> ThemeSettings.fromPalette
            |> AppSettings.fromTheme

        let json = AppSettingsLoader.toJson settings
        Assert.Contains("\"schemaVersion\": 1", json)

        let result = json |> AppSettingsLoader.loadText

        let palette =
            match result with
            | Ok settings -> settings.Theme.ThemeSource.Resolve()
            | Error error -> failwith error

        Assert.Equal(0xFF101112u, palette.Background)
        Assert.Equal(0xFFFAFBFCu, palette.Foreground)
        Assert.Equal(Some 0xFF333435u, palette.EditorBorder)
        Assert.Equal(3.0f, palette.EditorBorderWidth)

    [<Fact>]
    member _.``application settings serialize and load UI theme values``() =
        let ui =
            { UiThemeDefaults.defaultTheme with
                EditorFontFamily = "Cascadia Code"
                UiFontFamily = "Segoe UI Variable"
                EditorFontSize = 15.0
                EditorLineHeight = 18.0
                EditorTabSize = 2
                CursorWidth = 2.5
                GutterSeparatorWidth = 1.75
                TabCloseIconSize = 11.0
                ControlCornerRadius = 3.0
                WorkspaceSeparatorColor = 0xFF123456u }

        let settings =
            ThemeSettings.fromPaletteWithPresetAndUi "Custom" Theme.defaultPalette ui
            |> AppSettings.fromTheme

        let result = settings |> AppSettingsLoader.toJson |> AppSettingsLoader.loadText

        let loaded =
            match result with
            | Ok value -> value.Theme.Ui
            | Error error -> failwith error

        Assert.Equal("Cascadia Code", loaded.EditorFontFamily)
        Assert.Equal("Segoe UI Variable", loaded.UiFontFamily)
        Assert.Equal(15.0, loaded.EditorFontSize)
        Assert.Equal(18.0, loaded.EditorLineHeight)
        Assert.Equal(2, loaded.EditorTabSize)
        Assert.Equal(2.5, loaded.CursorWidth)
        Assert.Equal(1.75, loaded.GutterSeparatorWidth)
        Assert.Equal(11.0, loaded.TabCloseIconSize)
        Assert.Equal(3.0, loaded.ControlCornerRadius)
        Assert.Equal(0xFF123456u, loaded.WorkspaceSeparatorColor)

    [<Fact>]
    member _.``rejects a non-positive editor tab size``() =
        let result =
            ThemeSettingsLoader.loadText "{ \"theme\": { \"schemaVersion\": 1, \"editorTabSize\": 0 } }"

        Assert.True(result |> Result.isError)

    [<Fact>]
    member _.``rejects a non-positive cursor width``() =
        let result =
            ThemeSettingsLoader.loadText "{ \"theme\": { \"schemaVersion\": 1, \"cursorWidth\": 0 } }"

        Assert.True(result |> Result.isError)

    [<Fact>]
    member _.``rejects a non-positive gutter separator width``() =
        let result =
            ThemeSettingsLoader.loadText "{ \"theme\": { \"schemaVersion\": 1, \"gutterSeparatorWidth\": 0 } }"

        Assert.True(result |> Result.isError)

    [<Fact>]
    member _.``application settings serialize and load syntax colors``() =
        let palette =
            { Theme.defaultPalette with
                SyntaxColors =
                    Theme.defaultPalette.SyntaxColors.Add("keyword", 0xFF010203u).Add("function", 0xFF040506u) }

        let settings =
            ThemeSettings.fromPaletteWithPreset "Custom" palette |> AppSettings.fromTheme

        let result = settings |> AppSettingsLoader.toJson |> AppSettingsLoader.loadText

        let loadedPalette =
            match result with
            | Ok value -> value.Theme.ThemeSource.Resolve()
            | Error error -> failwith error

        Assert.Equal(Some 0xFF010203u, loadedPalette.SyntaxColors |> Map.tryFind "keyword")
        Assert.Equal(Some 0xFF040506u, loadedPalette.SyntaxColors |> Map.tryFind "function")
