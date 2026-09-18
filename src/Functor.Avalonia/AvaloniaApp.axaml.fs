namespace Functor.Avalonia

open Avalonia
open Avalonia.Controls.ApplicationLifetimes
open Avalonia.Data.Core
open Avalonia.Data.Core.Plugins
open Avalonia.Markup.Xaml
open Functor.Avalonia.Views

type AvaloniaApp() =
    inherit Application()

    override this.Initialize() = AvaloniaXamlLoader.Load(this)

    override this.OnFrameworkInitializationCompleted() =
        match this.ApplicationLifetime with
        | :? IClassicDesktopStyleApplicationLifetime as desktopLifetime -> desktopLifetime.MainWindow <- MainWindow()
        | :? IActivityApplicationLifetime as singleViewFactoryApplicationLifetime ->
            singleViewFactoryApplicationLifetime.MainViewFactory <- fun () -> ShellHostView()
        | :? ISingleViewApplicationLifetime as singleViewLifetime -> singleViewLifetime.MainView <- ShellHostView()
        | _ -> ()

        base.OnFrameworkInitializationCompleted()
