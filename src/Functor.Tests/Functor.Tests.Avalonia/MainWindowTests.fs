namespace Functor.Tests.Avalonia

open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Interactivity
open Functor.Application
open Functor.Avalonia.Views
open Xunit

module MainWindowTests =
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
