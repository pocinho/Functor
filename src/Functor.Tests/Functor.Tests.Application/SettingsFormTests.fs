namespace Functor.Tests.Application

open Functor.Application
open Xunit

type SettingsFormTests() =
    [<Fact>]
    member _.``round trips default settings``() =
        let result =
            AppSettings.defaults
            |> SettingsForm.fromAppSettings
            |> SettingsForm.tryBuildAppSettings

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
        let form =
            AppSettings.defaults
            |> SettingsForm.fromAppSettings
            |> fun value ->
                { value with
                    Background = "not-a-color" }

        Assert.Equal(Error "Background must be a 6- or 8-digit hex color.", SettingsForm.tryBuildAppSettings form)

    [<Fact>]
    member _.``rejects colors with unsupported hex lengths``() =
        let draft =
            AppSettings.defaults
            |> SettingsForm.fromAppSettings
            |> fun value -> { value with Background = "1" }

        Assert.Equal(Error "Background must be a 6- or 8-digit hex color.", SettingsForm.tryBuildAppSettings draft)

    [<Fact>]
    member _.``rejects non-finite editor border widths``() =
        let draft =
            AppSettings.defaults
            |> SettingsForm.fromAppSettings
            |> fun value ->
                { value with
                    EditorBorderWidth = "Infinity" }

        Assert.Equal(Error "Editor border width must be a non-negative number.", SettingsForm.tryBuildAppSettings draft)

    [<Fact>]
    member _.``rejects non-positive tab close icon sizes``() =
        let draft =
            AppSettings.defaults
            |> SettingsForm.fromAppSettings
            |> fun value -> { value with TabCloseIconSize = "0" }

        Assert.Equal(Error "Tab close icon size must be a positive number.", SettingsForm.tryBuildAppSettings draft)

    [<Fact>]
    member _.``rejects non-positive command palette width``() =
        let draft =
            AppSettings.defaults
            |> SettingsForm.fromAppSettings
            |> fun value -> { value with CommandPaletteWidth = "0" }

        Assert.Equal(Error "Command palette width must be a positive number.", SettingsForm.tryBuildAppSettings draft)

    [<Fact>]
    member _.``applying a preset replaces palette values``() =
        let draft = AppSettings.defaults |> SettingsForm.fromAppSettings
        let updated = SettingsForm.applyPreset "Graphite Light" draft

        Assert.Equal("Graphite Light", updated.ThemePreset)
        Assert.Equal("#FFE7E5EA", updated.Background)
        Assert.Equal("#FF4E3A78", updated.ResizeHandleColor)

    [<Fact>]
    member _.``applying Graphite Dark uses its built-in resize handle color``() =
        let draft = AppSettings.defaults |> SettingsForm.fromAppSettings
        let updated = SettingsForm.applyPreset "Graphite Dark" draft

        Assert.Equal("#FF333333", updated.ResizeHandleColor)

    [<Fact>]
    member _.``round trips custom UI settings``() =
        let draft =
            AppSettings.defaults
            |> SettingsForm.fromAppSettings
            |> fun value ->
                { value with
                    EditorFontFamily = "Cascadia Code"
                    UiFontFamily = "Segoe UI Variable"
                    EditorFontSize = "15"
                    EditorLineHeight = "18"
                    EditorTabSize = "2"
                    CursorWidth = "2.5"
                    GutterSeparatorWidth = "1.75"
                    TabCloseIconSize = "11"
                    ControlCornerRadius = "4"
                    CommandPaletteWidth = "640"
                    CommandPaletteTopMargin = "12"
                    CommandPalettePadding = "14"
                    CommandPaletteMaxHeight = "320"
                    CommandPaletteItemMarginHorizontal = "10"
                    CommandPaletteItemMarginVertical = "7"
                    CommandPaletteGestureMargin = "20"
                    WorkspaceSeparatorColor = "#FF123456"
                    SyntaxKeyword = "#FF654321" }

        let result = SettingsForm.tryBuildAppSettings draft

        match result with
        | Ok settings ->
            Assert.Equal("Cascadia Code", settings.Theme.Ui.EditorFontFamily)
            Assert.Equal("Segoe UI Variable", settings.Theme.Ui.UiFontFamily)
            Assert.Equal(15.0, settings.Theme.Ui.EditorFontSize)
            Assert.Equal(18.0, settings.Theme.Ui.EditorLineHeight)
            Assert.Equal(2, settings.Theme.Ui.EditorTabSize)
            Assert.Equal(2.5, settings.Theme.Ui.CursorWidth)
            Assert.Equal(1.75, settings.Theme.Ui.GutterSeparatorWidth)
            Assert.Equal(11.0, settings.Theme.Ui.TabCloseIconSize)
            Assert.Equal(4.0, settings.Theme.Ui.ControlCornerRadius)
            Assert.Equal(640.0, settings.Theme.Ui.CommandPaletteWidth)
            Assert.Equal(12.0, settings.Theme.Ui.CommandPaletteTopMargin)
            Assert.Equal(14.0, settings.Theme.Ui.CommandPalettePadding)
            Assert.Equal(320.0, settings.Theme.Ui.CommandPaletteMaxHeight)
            Assert.Equal(10.0, settings.Theme.Ui.CommandPaletteItemMarginHorizontal)
            Assert.Equal(7.0, settings.Theme.Ui.CommandPaletteItemMarginVertical)
            Assert.Equal(20.0, settings.Theme.Ui.CommandPaletteGestureMargin)
            Assert.Equal(0xFF123456u, settings.Theme.Ui.WorkspaceSeparatorColor)
            Assert.Equal(Some 0xFF654321u, settings.Theme.ThemeSource.Resolve().SyntaxColors |> Map.tryFind "keyword")
        | Error error -> failwith error
