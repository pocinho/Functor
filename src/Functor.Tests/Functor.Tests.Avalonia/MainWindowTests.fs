namespace Functor.Tests.Avalonia

open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Input
open Avalonia.Interactivity
open Functor.Application
open Functor.Avalonia.Views
open Xunit

module MainWindowTests =
    let private invokeSettingsCommand (window: MainWindow) =
        window.FindControl<Button>("CommandCenterButton").RaiseEvent(RoutedEventArgs(Button.ClickEvent))

        let palette = window.FindControl<CommandPaletteView>("CommandPaletteView")
        let commandList = palette.FindControl<ListBox>("CommandList")

        commandList.SelectedIndex <-
            AppCommandCatalog.all
            |> List.findIndex (fun descriptor -> descriptor.Id = "workbench.settings")

        palette.RaiseEvent(KeyEventArgs(RoutedEvent = InputElement.KeyDownEvent, Key = Key.Enter))

    [<AvaloniaFact>]
    let ``recent document menu follows close and clear lifecycle`` () =
        let window = MainWindow()
        window.Show()

        let editor = window.FindControl<ShellHostView>("ShellHostView").Editor
        let recentMenu = window.FindControl<MenuItem>("RecentDocumentsMenuItem")
        let reopenMenu = window.FindControl<MenuItem>("ReopenClosedTabMenuItem")
        let clearMenu = window.FindControl<MenuItem>("ClearRecentDocumentsMenuItem")

        Assert.False(recentMenu.IsEnabled)
        Assert.False(reopenMenu.IsEnabled)
        Assert.False(clearMenu.IsEnabled)

        editor.DispatchApplicationCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        editor.CloseDocument()

        Assert.Equal(1, recentMenu.Items.Count)
        Assert.True(recentMenu.IsEnabled)
        Assert.True(reopenMenu.IsEnabled)
        Assert.True(clearMenu.IsEnabled)

        clearMenu.RaiseEvent(RoutedEventArgs(MenuItem.ClickEvent))

        Assert.Empty(recentMenu.Items)
        Assert.False(recentMenu.IsEnabled)
        Assert.False(reopenMenu.IsEnabled)
        Assert.False(clearMenu.IsEnabled)

        window.Close()

    [<AvaloniaFact>]
    let ``settings tab visibility follows open and close lifecycle`` () =
        let window = MainWindow()
        window.Show()

        let shellHost = window.FindControl<ShellHostView>("ShellHostView")
        let settingsDocument = shellHost.FindControl<Grid>("SettingsDocument")
        let settingsMenu = window.FindControl<MenuItem>("SettingsMenuItem")

        settingsMenu.RaiseEvent(RoutedEventArgs(MenuItem.ClickEvent))
        Assert.True(settingsDocument.IsVisible)

        let settingsTabButton = shellHost.FindControl<Button>("SettingsTabButton")
        let closeSettingsButton = shellHost.FindControl<Button>("CloseSettingsButton")

        Assert.True(settingsTabButton.IsVisible)

        settingsMenu.RaiseEvent(RoutedEventArgs(MenuItem.ClickEvent))
        Assert.False(settingsDocument.IsVisible)
        Assert.False(settingsTabButton.IsVisible)

        settingsMenu.RaiseEvent(RoutedEventArgs(MenuItem.ClickEvent))
        Assert.True(settingsDocument.IsVisible)

        closeSettingsButton.RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        Assert.False(settingsDocument.IsVisible)
        Assert.False(settingsTabButton.IsVisible)

        window.Close()
