namespace Functor.Tests.Application

open Functor.Application
open Xunit

type SettingsDraftTests() =
    [<Fact>]
    member _.``round trips default settings``() =
        let result =
            AppSettings.defaults
            |> SettingsDraft.fromSettings
            |> SettingsDraft.tryCreateSettings

        match result with
        | Ok settings ->
            Assert.Equal(AppSettings.defaults.Theme.Preset, settings.Theme.Preset)

            Assert.Equal(
                AppSettings.defaults.Theme.ThemeSource.Resolve().Background,
                settings.Theme.ThemeSource.Resolve().Background
            )
        | Error error -> failwith error

    [<Fact>]
    member _.``rejects invalid colors``() =
        let draft =
            AppSettings.defaults
            |> SettingsDraft.fromSettings
            |> fun value ->
                { value with
                    Background = "not-a-color" }

        Assert.Equal(Error "Background must be a 6- or 8-digit hex color.", SettingsDraft.tryCreateSettings draft)

    [<Fact>]
    member _.``rejects colors with unsupported hex lengths``() =
        let draft =
            AppSettings.defaults
            |> SettingsDraft.fromSettings
            |> fun value -> { value with Background = "1" }

        Assert.Equal(Error "Background must be a 6- or 8-digit hex color.", SettingsDraft.tryCreateSettings draft)

    [<Fact>]
    member _.``rejects non-finite editor border widths``() =
        let draft =
            AppSettings.defaults
            |> SettingsDraft.fromSettings
            |> fun value ->
                { value with
                    EditorBorderWidth = "Infinity" }

        Assert.Equal(Error "Editor border width must be a non-negative number.", SettingsDraft.tryCreateSettings draft)

    [<Fact>]
    member _.``rejects non-positive tab close icon sizes``() =
        let draft =
            AppSettings.defaults
            |> SettingsDraft.fromSettings
            |> fun value -> { value with TabCloseIconSize = "0" }

        Assert.Equal(Error "Tab close icon size must be a positive number.", SettingsDraft.tryCreateSettings draft)

    [<Fact>]
    member _.``applying a preset replaces palette values``() =
        let draft = AppSettings.defaults |> SettingsDraft.fromSettings
        let updated = SettingsDraft.applyPreset "Graphite Light" draft

        Assert.Equal("Graphite Light", updated.ThemePreset)
        Assert.Equal("#FFE7E5EA", updated.Background)

    [<Fact>]
    member _.``round trips custom UI settings``() =
        let draft =
            AppSettings.defaults
            |> SettingsDraft.fromSettings
            |> fun value ->
                { value with
                    EditorFontFamily = "Cascadia Code"
                    UiFontFamily = "Segoe UI Variable"
                    EditorLineHeight = "18"
                    EditorTabSize = "2"
                    CursorWidth = "2.5"
                    GutterSeparatorWidth = "1.75"
                    TabCloseIconSize = "11"
                    ControlCornerRadius = "4"
                    WorkspaceSeparatorColor = "#FF123456"
                    SyntaxKeyword = "#FF654321" }

        let result = SettingsDraft.tryCreateSettings draft

        match result with
        | Ok settings ->
            Assert.Equal("Cascadia Code", settings.Theme.Ui.EditorFontFamily)
            Assert.Equal("Segoe UI Variable", settings.Theme.Ui.UiFontFamily)
            Assert.Equal(18.0, settings.Theme.Ui.EditorLineHeight)
            Assert.Equal(2, settings.Theme.Ui.EditorTabSize)
            Assert.Equal(2.5, settings.Theme.Ui.CursorWidth)
            Assert.Equal(1.75, settings.Theme.Ui.GutterSeparatorWidth)
            Assert.Equal(11.0, settings.Theme.Ui.TabCloseIconSize)
            Assert.Equal(4.0, settings.Theme.Ui.ControlCornerRadius)
            Assert.Equal(0xFF123456u, settings.Theme.Ui.WorkspaceSeparatorColor)
            Assert.Equal(Some 0xFF654321u, settings.Theme.ThemeSource.Resolve().SyntaxColors |> Map.tryFind "keyword")
        | Error error -> failwith error
