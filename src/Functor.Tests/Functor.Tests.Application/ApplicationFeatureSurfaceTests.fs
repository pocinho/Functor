namespace Functor.Tests.Application

open Functor.Application
open Xunit

type ApplicationFeatureSurfaceTests() =
    [<Fact>]
    member _.``shell feature exposes command lookup and state operations``() =
        let state = ShellFeature.initialState |> ShellFeature.openCommandPalette

        Assert.True(state.IsCommandPaletteOpen)

        match ShellFeature.resolveCommand "file.save" with
        | CommandFound descriptor -> Assert.Equal("file.save", descriptor.Id)
        | CommandNotFound id -> failwithf "Expected command %s to be found" id

    [<Fact>]
    member _.``settings feature validates and builds application settings``() =
        let form = SettingsFeature.formFromAppSettings AppSettings.defaults

        match SettingsFeature.validateForm form with
        | Ok settings -> Assert.Equal(AppSettings.defaults.Theme.Preset, settings.Theme.Preset)
        | Error error -> failwithf "Expected default settings to validate, but got %s" error

    [<Fact>]
    member _.``theme feature creates preset settings``() =
        let settings =
            ThemeFeature.settingsFromPaletteWithPreset
                ThemePreset.GraphiteLight
                Functor.Rendering.Theme.graphiteLight

        Assert.Equal(ThemePreset.GraphiteLight, settings.Preset)