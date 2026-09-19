namespace Functor.Tests.Avalonia

open System
open System.Threading.Tasks
open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Interactivity
open Avalonia.Threading
open Functor.Avalonia
open Functor.Avalonia.Controls
open Functor.Avalonia.Views
open Functor.Domain.Document
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

        let sidePanel = host.FindControl<SidePanelControl>("SidePanelHost")

        host.FindControl<Button>("WorkspaceToolButton").RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        Assert.True(sidePanel.IsOpen)
        Assert.Equal("Workspace", sidePanel.Title)
        firstEditor.NewDocument()
        Dispatcher.UIThread.RunJobs()
        Assert.True(sidePanel.IsOpen)
        Assert.Equal(500.0, sidePanel.PanelWidth)
        Assert.Equal("Workspace", sidePanel.Title)

        Assert.False(
            WorkspaceModel.activeDocument host.SessionState.Workspace
            |> Option.get
            |> fun document -> document.Auxiliary.Agent.IsOpen
        )

        Assert.Same(firstEditor, host.Editor)

        window.Close()

    [<AvaloniaFact>]
    member _.``switching documents retains the workspace tree control``() =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        host.FindControl<Button>("WorkspaceToolButton").RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        let sidePanel = host.FindControl<SidePanelControl>("SidePanelHost")
        let workspaceView = sidePanel.PanelContent :?> WorkspaceDocumentControl

        host.Editor.NewDocument()
        host.Editor.NewDocument()
        Dispatcher.UIThread.RunJobs()
        let content = workspaceView.FindControl<StackPanel>("ContentPanel")
        let treePanel = content.Children[2] :?> StackPanel
        let renderedTree = treePanel.Children[0]

        host.Editor.ActivateDocument(host.SessionState.Workspace.TabOrder.Head)
        Dispatcher.UIThread.RunJobs()

        Assert.Same(workspaceView, sidePanel.PanelContent)
        Assert.Same(renderedTree, treePanel.Children[0])

        window.Close()

    [<AvaloniaFact>]
    member _.``opening a file reuses the rendered workspace tree``() =
        let workspaceView = WorkspaceDocumentControl()
        let window = Window(Content = workspaceView)
        window.Show()

        let filePath = "C:\\work\\opened.fs"
        let closedFile =
            { Key = "file:" + filePath
              Name = "opened.fs"
              Path = filePath
              IsDirectory = false
              DocumentId = None
              IsDirty = false
              Children = [] }

        let openFile = { closedFile with DocumentId = Some(Guid.NewGuid()) }
        let root =
            { Key = "workspace:test"
              Name = "Workspace"
              Path = ""
              IsDirectory = true
              DocumentId = None
              IsDirty = false
              Children = [ closedFile ] }

        workspaceView.ApplyWorkspace(root, [])
        let content = workspaceView.FindControl<StackPanel>("ContentPanel")
        let treePanel = content.Children[2] :?> StackPanel
        let renderedTree = treePanel.Children[0]

        workspaceView.ApplyWorkspace({ root with Children = [ openFile ] }, [])

        Assert.Same(renderedTree, treePanel.Children[0])

        window.Close()

    [<AvaloniaFact>]
    member _.``shell host restores persisted layout``() =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        host.ApplyLayout
            { SidePanelWidth = 480.0
              ActiveToolId = Some "search"
              IsToolPanelOpen = true }

        let layout = host.Layout
        Assert.Equal(480.0, layout.SidePanelWidth)
        Assert.Equal(Some "search", layout.ActiveToolId)
        Assert.True(layout.IsToolPanelOpen)

        window.Close()

    [<AvaloniaFact>]
    member _.``shell host marshals background dispatches to the UI thread``() =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        host.FindControl<Button>("WorkspaceToolButton").RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        Task.Run(fun () -> host.Dispatch(SetSidePanelWidth 500.0)).GetAwaiter().GetResult()
        Dispatcher.UIThread.RunJobs()

        let sidePanel = host.FindControl<SidePanelControl>("SidePanelHost")
        Assert.Equal(500.0, sidePanel.PanelWidth)

        window.Close()

    [<AvaloniaFact>]
    member _.``rapid side panel resizing keeps the final width stable``() =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        host.FindControl<Button>("WorkspaceToolButton").RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        let sidePanel = host.FindControl<SidePanelControl>("SidePanelHost")

        for index in 0..99 do
            host.Dispatch(SetSidePanelWidth(240.0 + float index * 4.0))

        Dispatcher.UIThread.RunJobs()

        Assert.Equal(636.0, sidePanel.PanelWidth)

        window.Close()

    [<AvaloniaFact>]
    member _.``open editor rows use the same child indentation as the explorer tree``() =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        host.FindControl<Button>("WorkspaceToolButton").RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        host.Editor.NewDocument()
        Dispatcher.UIThread.RunJobs()

        let workspaceView = host.FindControl<SidePanelControl>("SidePanelHost").PanelContent :?> WorkspaceDocumentControl
        let content = workspaceView.FindControl<StackPanel>("ContentPanel")
        let openEditorsContainer = content.Children[1] :?> StackPanel
        let editors = openEditorsContainer.Children[1] :?> StackPanel
        let openEditorRow = editors.Children[0] :?> Button
        let tree = content.Children[2] :?> StackPanel
        let emptyWorkspaceRow = tree.Children[0] :?> Button

        Assert.Equal(14.0, openEditorRow.Margin.Left)
        Assert.Equal(0.0, emptyWorkspaceRow.Margin.Left)

        window.Close()
