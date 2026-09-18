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

        let sidePanel = host.FindControl<SidePanelView>("SidePanelHost")

        host.FindControl<Button>("NotebookToggleButton").RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        Assert.True(sidePanel.IsOpen)
        Assert.Equal("Workspace", sidePanel.Title)

        Assert.True(
            WorkspaceModel.activeDocument host.SessionState.Workspace
            |> Option.get
            |> fun document -> document.Auxiliary.Notebook.IsOpen
        )

        Assert.Same(firstEditor, host.Editor)

        window.Close()

    [<AvaloniaFact>]
    member _.``shell host restores persisted layout``() =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        host.ApplyLayout { SidePanelWidth = 480.0 }

        Assert.Equal(480.0, host.Layout.SidePanelWidth)

        window.Close()

    [<AvaloniaFact>]
    member _.``shell host marshals background dispatches to the UI thread``() =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        host.FindControl<Button>("NotebookToggleButton").RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        Task.Run(fun () -> host.Dispatch(SetSidePanelWidth 500.0)).GetAwaiter().GetResult()
        Dispatcher.UIThread.RunJobs()

        let sidePanel = host.FindControl<SidePanelView>("SidePanelHost")
        Assert.Equal(500.0, sidePanel.PanelWidth)

        window.Close()
