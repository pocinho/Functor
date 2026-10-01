namespace Functor.Application

type SettingsForm = SettingsFormModel

module SettingsForm =
    let paletteForPreset = ThemePreset.palette

    let fromAppSettingsWithPreset = SettingsFormProjection.fromAppSettingsWithPreset

    let fromAppSettings = SettingsFormProjection.fromAppSettings

    let applyPreset = SettingsFormProjection.applyPreset

    let tryBuildAppSettings = SettingsFormValidation.tryBuildAppSettings
