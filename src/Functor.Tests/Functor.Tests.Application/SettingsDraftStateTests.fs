namespace Functor.Tests.Application

open Functor.Application
open Xunit

type SettingsDraftStateTests() =
    [<Fact>]
    member _.``from app settings initializes the draft``() =
        let state = SettingsDraftState.fromAppSettings AppSettings.defaults

        Assert.Equal(Some(SettingsForm.fromAppSettings AppSettings.defaults), state.Form)
        Assert.Equal(AppSettings.defaults.Theme.Preset, state.ThemeName)
        Assert.Equal(None, state.SelectedThemeName)

    [<Fact>]
    member _.``editing a named theme switches to Custom``() =
        let state =
            SettingsDraftState.fromAppSettings AppSettings.defaults
            |> SettingsDraftState.selectNamedTheme "Ocean" AppSettings.defaults
            |> SettingsDraftState.updateForm (fun form -> { form with Background = "#FF112233" })

        Assert.Equal(Some ThemePreset.Custom, state.Form |> Option.map (fun form -> form.ThemePreset))
        Assert.Equal(Some "#FF112233", state.Form |> Option.map (fun form -> form.Background))
        Assert.Equal(None, state.SelectedThemeName)

    [<Fact>]
    member _.``selecting a named theme replaces the form and preserves its name``() =
        let state =
            SettingsDraftState.fromAppSettings AppSettings.defaults
            |> SettingsDraftState.selectNamedTheme "Ocean" AppSettings.defaults

        Assert.Equal(Some "Ocean", state.SelectedThemeName)
        Assert.Equal("Ocean", state.ThemeName)
        Assert.Equal(Some AppSettings.defaults.Theme.Preset, state.Form |> Option.map (fun form -> form.ThemePreset))

    [<Fact>]
    member _.``selecting Custom clears the selected theme``() =
        let state =
            SettingsDraftState.fromAppSettings AppSettings.defaults
            |> SettingsDraftState.selectNamedTheme "Ocean" AppSettings.defaults
            |> SettingsDraftState.selectCustom

        Assert.Equal(None, state.SelectedThemeName)
        Assert.Equal(Some ThemePreset.Custom, state.Form |> Option.map (fun form -> form.ThemePreset))

    [<Fact>]
    member _.``applying a preset updates the name and clears the selected theme``() =
        let state =
            SettingsDraftState.fromAppSettings AppSettings.defaults
            |> SettingsDraftState.selectNamedTheme "Ocean" AppSettings.defaults
            |> SettingsDraftState.applyPreset ThemePreset.GraphiteLight

        Assert.Equal(None, state.SelectedThemeName)
        Assert.Equal(ThemePreset.GraphiteLight, state.ThemeName)
        Assert.Equal(Some ThemePreset.GraphiteLight, state.Form |> Option.map (fun form -> form.ThemePreset))
