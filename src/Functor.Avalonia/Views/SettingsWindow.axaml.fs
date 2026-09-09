namespace Functor.Avalonia.Views

open Avalonia.Controls
open Avalonia.Markup.Xaml
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

    let tryApply closeAfterApply save =
        match viewModel.TryCreateSettings() with
        | Error error -> viewModel.ErrorMessage <- error
        | Ok settings ->
            match if save then saveSettings settings else Ok(applySettings settings) with
            | Ok () ->
                viewModel.ErrorMessage <- ""

                if closeAfterApply then
                    this.Close()
            | Error error -> viewModel.ErrorMessage <- error

    do
        this.InitializeComponent()
        settingsView.Value.DataContext <- viewModel
        applyButton.Value.Click.Add(fun _ -> tryApply false false)
        saveButton.Value.Click.Add(fun _ -> tryApply true true)
        cancelButton.Value.Click.Add(fun _ -> this.Close())

    new() = SettingsWindow(AppSettings.defaults, ignore, fun _ -> Ok())

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)