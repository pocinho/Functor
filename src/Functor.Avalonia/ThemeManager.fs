namespace Functor.Avalonia

open Avalonia
open Avalonia.Controls
open Avalonia.Styling
open Functor.Application
open Functor.Rendering

module ThemeManager =
    let private applyResources (application: Application) (settings: AppSettings) =
        let resources = application.Resources
        ThemeResources.applyUi resources settings.Theme.Ui
        ThemeResources.applyPalette resources settings.Theme.Ui (settings.Theme.ThemeSource.Resolve())

    let apply (application: Application) (window: Window) (settings: AppSettings) =
        applyResources application settings

        window.RequestedThemeVariant <-
            if settings.Theme.Preset = ThemePreset.GraphiteLight then
                ThemeVariant.Light
            else
                ThemeVariant.Dark

    let applyDefaults (application: Application) =
        let resources = application.Resources
        ThemeResources.applyUi resources UiThemeDefaults.defaultTheme
        ThemeResources.applyPalette resources UiThemeDefaults.defaultTheme Theme.defaultPalette
