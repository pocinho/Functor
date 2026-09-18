namespace Functor.Tests.Avalonia

open System.Threading.Tasks
open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Interactivity
open Avalonia.Threading
open Functor.Avalonia
open Functor.Avalonia.Controls
open Functor.Avalonia.Views
open Functor.Workspace
open Xunit

type ShellHostViewTests() =
    [<AvaloniaFact>]
    member _.``shell host retains one editor control``() =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        let firstEditor = host.Editor
        Assert.Same(firstEditor, host.Editor)

        let tabs = host.FindControl<DocumentListView>("TabsPanel")
        let initialTabCount = tabs.TabCount
        firstEditor.NewDocument()

        Assert.Equal(initialTabCount + 1, tabs.TabCount)

        host.Dispatch(SetSidePanelWidth 500.0)

        host.Dispatch(SelectPanel(Some Agent))

        let sidePanel = host.FindControl<SidePanelView>("SidePanelHost")
        Assert.True(sidePanel.IsOpen)
        Assert.Equal(500.0, sidePanel.PanelWidth)
        Assert.Equal("Agent", sidePanel.Title)
        Assert.Equal("Agent", (sidePanel.PanelContent :?> TextBlock).Text)

        host.Dispatch(SelectPanel None)
        Assert.False(sidePanel.IsOpen)
        Assert.Equal(0.0, sidePanel.PanelWidth)

        host.FindControl<Button>("NotebookPanelButton").RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        Assert.True(sidePanel.IsOpen)
        Assert.Equal("Notebook", sidePanel.Title)

        Assert.Same(firstEditor, host.Editor)

        window.Close()

    [<AvaloniaFact>]
    member _.``shell host restores persisted layout``() =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        host.ApplyLayout
            { IsSidePanelOpen = false
              SidePanelWidth = 480.0
              ActivePanel = Some "agent" }

        Assert.Equal(480.0, host.Layout.SidePanelWidth)
        Assert.Equal(Some "agent", host.Layout.ActivePanel)
        Assert.False(host.Layout.IsSidePanelOpen)

        window.Close()

    [<AvaloniaFact>]
    member _.``shell host marshals background dispatches to the UI thread``() =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        Task.Run(fun () -> host.Dispatch(SelectPanel(Some Agent))).GetAwaiter().GetResult()
        Dispatcher.UIThread.RunJobs()

        let sidePanel = host.FindControl<SidePanelView>("SidePanelHost")
        Assert.True(sidePanel.IsOpen)
        Assert.Equal("Agent", sidePanel.Title)

        window.Close()
