namespace Functor.Avalonia.Views

open Avalonia.Controls
open Avalonia.Markup.Xaml
open Avalonia.Media
open Avalonia.Styling
open Functor.Application
open Functor.Avalonia.ViewModels

type SettingsWindow(
    initialSettings: AppSettings,
    applySettings: AppSettings -> unit,
    saveSettings: AppSettings -> Result<unit, string>
) as this =
    inherit Window()

    let viewModel = SettingsViewModel(initialSettings)
    let settingsView = lazy (this.FindControl<SettingsView>("SettingsView"))
    let applyButton = lazy (this.FindControl<Button>("ApplyButton"))
    let saveButton = lazy (this.FindControl<Button>("SaveButton"))
    let cancelButton = lazy (this.FindControl<Button>("CancelButton"))
    let settingsFooter = lazy (this.FindControl<Border>("SettingsFooter"))

    let applyTheme settings =
        let palette = settings.Theme.ThemeSource.Resolve()
        let colorFromArgb (argb: uint32) =
            Color.FromArgb(byte (argb >>> 24), byte (argb >>> 16), byte (argb >>> 8), byte argb)

        this.RequestedThemeVariant <-
            if settings.Theme.Preset = "Graphite Light" then ThemeVariant.Light else ThemeVariant.Dark

        this.Background <- SolidColorBrush(colorFromArgb palette.Background)
        this.Foreground <- SolidColorBrush(colorFromArgb palette.Foreground)
        settingsView.Value.ApplyTheme settings.Theme
        settingsFooter.Value.BorderBrush <- SolidColorBrush(colorFromArgb (palette.GutterSeparator |> Option.defaultValue palette.Foreground))

    let tryApply closeAfterApply save =
        match viewModel.TryCreateSettings() with
        | Error error -> viewModel.ErrorMessage <- error
        | Ok settings ->
            match if save then saveSettings settings else Ok(applySettings settings) with
            | Ok () ->
                applyTheme settings
                viewModel.ErrorMessage <- ""

                if closeAfterApply then
                    this.Close()
            | Error error -> viewModel.ErrorMessage <- error

    do
        this.InitializeComponent()
        settingsView.Value.DataContext <- viewModel
        applyTheme initialSettings
        applyButton.Value.Click.Add(fun _ -> tryApply false false)
        saveButton.Value.Click.Add(fun _ -> tryApply true true)
        cancelButton.Value.Click.Add(fun _ -> this.Close())

    new() = SettingsWindow(AppSettings.defaults, ignore, fun _ -> Ok())

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)