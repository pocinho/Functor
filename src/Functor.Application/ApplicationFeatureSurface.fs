namespace Functor.Application

/// Public entry points for application features used by outer adapters.
/// Feature implementation modules remain private to the application assembly.
module ShellFeature =
    let commands = ShellCommands.all

    let tryFindCommand id = ShellCommands.tryFindById id

    let resolveCommand id = ShellCommands.resolveById id

    let filterCommands query available = ShellCommands.filter query available

    let initialState = ShellState.initial

    let openCommandPalette state = ShellState.openCommandPalette state

    let closeCommandPalette state = ShellState.closeCommandPalette state

    let withSettings settings state = ShellState.withSettings settings state

module SettingsFeature =
    let emptyDraft = SettingsDraftState.empty

    let draftFromAppSettings settings = SettingsDraftState.fromAppSettings settings

    let selectNamedTheme name settings state = SettingsDraftState.selectNamedTheme name settings state

    let selectCustomTheme state = SettingsDraftState.selectCustom state

    let applyThemePreset preset state = SettingsDraftState.applyPreset preset state

    let updateDraft update state = SettingsDraftState.updateForm update state

    let formFromAppSettings settings = SettingsForm.fromAppSettings settings

    let formFromAppSettingsWithPreset preset settings =
        SettingsForm.fromAppSettingsWithPreset preset settings

    let applyPreset preset form = SettingsForm.applyPreset preset form

    let validateForm form =
        match SettingsForm.tryBuildAppSettings form with
        | Ok settings -> Ok settings
        | Error message -> Error message

module ThemeFeature =
    let defaultSettings = ThemeSettings.defaultTheme

    let settingsFromSource source = ThemeSettings.fromSource source

    let settingsFromPalette palette = ThemeSettings.fromPalette palette

    let settingsFromPaletteWithPreset preset palette =
        ThemeSettings.fromPaletteWithPreset preset palette

    let settingsFromPaletteWithPresetAndUi preset palette ui =
        ThemeSettings.fromPaletteWithPresetAndUi preset palette ui

    let presets = ThemePreset.all

    let presetPalette preset = ThemePreset.palette preset

    let presetUi preset = ThemePreset.ui preset