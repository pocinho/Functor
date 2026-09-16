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
    member _.``applying a preset replaces palette values``() =
        let draft = AppSettings.defaults |> SettingsDraft.fromSettings
        let updated = SettingsDraft.applyPreset "Graphite Light" draft

        Assert.Equal("Graphite Light", updated.ThemePreset)
        Assert.Equal("#FFE7E5EA", updated.Background)
