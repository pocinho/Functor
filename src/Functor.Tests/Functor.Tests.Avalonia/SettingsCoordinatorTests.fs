namespace Functor.Tests.Avalonia

open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Threading
open Functor.Application
open Functor.Avalonia.Views
open Xunit

module SettingsCoordinatorTests =
    let private createCoordinator () =
        let view = SettingsView()
        let window = Window(Content = view)
        window.Show()

        let coordinator =
            SettingsCoordinator(view, Button(), Grid(), (fun () -> true), (fun _ -> ()), (fun _ -> ()), ignore)

        coordinator.Configure(AppSettings.defaults, ignore, fun _ -> Ok())
        view, coordinator

    [<AvaloniaFact>]
    let ``invalid settings are reported without invoking save`` () =
        let view, coordinator = createCoordinator ()
        let saveCalls = ref 0

        coordinator.Configure(
            AppSettings.defaults,
            ignore,
            fun _ ->
                saveCalls.Value <- saveCalls.Value + 1
                Ok()
        )

        view.FindControl<TextBox>("Background").Text <- "not-a-color"
        Dispatcher.UIThread.RunJobs()
        coordinator.TryApply(true)

        Assert.Equal(0, saveCalls.Value)
        Assert.Equal("Background must be a 6- or 8-digit hex color.", view.FindControl<TextBlock>("ErrorText").Text)

    [<AvaloniaFact>]
    let ``save failures are surfaced in the settings view`` () =
        let view, coordinator = createCoordinator ()
        coordinator.Configure(AppSettings.defaults, ignore, fun _ -> Error "settings could not be saved")

        coordinator.TryApply(true)

        Assert.Equal("settings could not be saved", view.FindControl<TextBlock>("ErrorText").Text)

    [<AvaloniaFact>]
    let ``successful apply invokes callback and clears previous error`` () =
        let view, coordinator = createCoordinator ()
        let applied = ref None
        coordinator.Configure(AppSettings.defaults, (fun settings -> applied.Value <- Some settings), fun _ -> Ok())
        view.SetError("previous error")
        view.FindControl<TextBox>("Background").Text <- "#FF112233"
        Dispatcher.UIThread.RunJobs()

        coordinator.TryApply(false)

        Assert.True(applied.Value.IsSome)
        Assert.Equal(0xFF112233u, coordinator.CurrentSettings.Theme.ThemeSource.Resolve().Background)
        Assert.Equal("", view.FindControl<TextBlock>("ErrorText").Text)
