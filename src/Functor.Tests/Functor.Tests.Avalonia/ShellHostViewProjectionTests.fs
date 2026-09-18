namespace Functor.Tests.Avalonia

open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Input
open Avalonia.Interactivity
open Avalonia.Threading
open Functor.Application
open Functor.Avalonia.Controls
open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Avalonia.Views
open Xunit

module ShellHostViewProjectionTests =

    let private withHostedView test =
        let host = ShellHostView()
        let window = Window(Content = host)
        window.Show()

        if host.SessionState.Model.ActiveDocument.IsSome then
            host.Editor.CloseDocument()

        try
            test host
        finally
            window.Close()

    [<AvaloniaFact>]
    let ``an empty untitled document is ready for typing when the editor starts`` () =
        let editor = EditorControl()
        let window = Window(Content = editor)
        window.Show()

        Assert.True(editor.SessionState.Model.ActiveDocument.IsSome)

        editor.RaiseEvent(TextInputEventArgs(RoutedEvent = InputElement.TextInputEvent, Text = "hi"))

        Assert.Equal<string list>([ "hi" ], editor.SessionState.Model.Editing.Buffer)

    [<AvaloniaFact>]
    let ``welcome view replaces the editor after closing the only open document`` () =
        withHostedView (fun view ->
            view.Editor.NewDocument()

            let editorControl = view.FindControl<EditorControl>("EditorControl")
            let welcome = view.FindControl<WelcomeView>("WelcomeView")

            Assert.True(editorControl.IsVisible)
            Assert.False(welcome.IsVisible)

            view.Editor.CloseDocument()

            Assert.False(editorControl.IsVisible)
            Assert.True(welcome.IsVisible)
            Assert.True(view.SessionState.Model.ActiveDocument.IsNone))

    [<AvaloniaFact>]
    let ``choosing New File from the welcome view creates a document and hides the welcome view`` () =
        withHostedView (fun view ->
            view.Editor.NewDocument()
            view.Editor.CloseDocument()

            let editorControl = view.FindControl<EditorControl>("EditorControl")
            let welcome = view.FindControl<WelcomeView>("WelcomeView")
            let newFileButton = welcome.FindControl<Button>("NewFileButton")

            newFileButton.RaiseEvent(RoutedEventArgs(Button.ClickEvent))

            Assert.True(view.SessionState.Model.ActiveDocument.IsSome)
            Assert.True(editorControl.IsVisible)
            Assert.False(welcome.IsVisible))

    [<AvaloniaFact>]
    let ``shell host loads its host controls`` () =
        let view = ShellHostView()
        let window = Window(Content = view)
        window.Show()

        Assert.NotNull(view.FindControl<Control>("EditorControl"))
        Assert.NotNull(view.FindControl<Control>("TabBar"))
        Assert.NotNull(view.FindControl<Control>("TabsPanel"))
        Assert.NotNull(view.FindControl<Control>("StatusBar"))

        window.Close()

    [<AvaloniaFact>]
    let ``tab bar renders one tab per open document`` () =
        withHostedView (fun view ->
            view.Editor.NewDocument()
            view.Editor.NewDocument()
            Dispatcher.UIThread.RunJobs()

            let tabs = view.FindControl<DocumentListView>("TabsPanel")

            Assert.Equal(2, tabs.TabCount))

    [<AvaloniaFact>]
    let ``tab activation updates the active document projection`` () =
        let view = ShellHostView()
        let window = Window(Content = view)
        window.Show()
        view.Editor.NewDocument()
        view.Editor.NewDocument()
        let firstDocumentId = view.SessionState.Workspace.TabOrder.Head
        let secondDocumentId = view.SessionState.Workspace.TabOrder.Tail.Head

        view.Editor.ActivateDocument(firstDocumentId)

        Assert.Equal(Some firstDocumentId, view.SessionState.Workspace.ActiveDocumentId)
        Assert.NotEqual(secondDocumentId, view.SessionState.Workspace.ActiveDocumentId.Value)

        window.Close()

    [<AvaloniaFact>]
    let ``clicking a tab activates its document`` () =
        withHostedView (fun view ->
            view.Editor.NewDocument()
            view.Editor.NewDocument()
            let firstDocumentId = view.SessionState.Workspace.TabOrder.Head
            let tabs = view.FindControl<DocumentListView>("TabsPanel")
            let firstTab = tabs.GetTab(0)

            firstTab.RaiseEvent(RoutedEventArgs(Button.ClickEvent))

            Assert.Equal(Some firstDocumentId, view.SessionState.Workspace.ActiveDocumentId))

    [<AvaloniaFact>]
    let ``dirty state is reflected in the rendered tab label`` () =
        withHostedView (fun view ->
            view.Editor.NewDocument()
            view.Editor.DispatchApplicationCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "changed")))

            let tabs = view.FindControl<DocumentListView>("TabsPanel")
            let tabButton = tabs.GetTab(0)
            let content = tabButton.Content :?> StackPanel
            let label = content.Children[0] :?> TextBlock

            Assert.Equal("untitled *", label.Text))

    [<AvaloniaFact>]
    let ``agent panel choices are restored independently per tab`` () =
        withHostedView (fun view ->
            view.Editor.NewDocument()
            view.Editor.NewDocument()
            let firstId = view.SessionState.Workspace.TabOrder.Head
            let secondId = view.SessionState.Workspace.TabOrder.Tail.Head
            view.Editor.ActivateDocument(secondId)
            Dispatcher.UIThread.RunJobs()
            view.Editor.DispatchApplicationCommand(AppCommand.toggleAgentPanel)
            view.Editor.ActivateDocument(firstId)
            Dispatcher.UIThread.RunJobs()
            view.Editor.DispatchApplicationCommand(AppCommand.toggleAgentPanel)
            Assert.True(view.SessionState.Workspace.Documents[firstId].Auxiliary.Agent.IsOpen)

            view.Editor.ActivateDocument(secondId)
            Assert.True(view.SessionState.Workspace.Documents[secondId].Auxiliary.Agent.IsOpen)

            view.Editor.ActivateDocument(firstId)
            Assert.True(view.SessionState.Workspace.Documents[firstId].Auxiliary.Agent.IsOpen))

    [<AvaloniaFact>]
    let ``status bar reflects messages and dirty state`` () =
        withHostedView (fun view ->
            view.Editor.NewDocument()
            view.Editor.DispatchApplicationCommand(AppCommand.setStatus "Ready")
            view.Editor.DispatchApplicationCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "changed")))

            let message = view.FindControl<TextBlock>("MessageText")
            let dirty = view.FindControl<TextBlock>("DirtyText")

            Assert.Equal("Ready", message.Text)
            Assert.Equal("Modified", dirty.Text))

    [<AvaloniaFact>]
    let ``closing tabs repeatedly updates the tab projection and welcome state`` () =
        withHostedView (fun view ->
            view.Editor.NewDocument()
            view.Editor.NewDocument()

            let tabs = view.FindControl<DocumentListView>("TabsPanel")
            Assert.Equal(2, tabs.TabCount)

            view.Editor.CloseDocument()

            Assert.Equal(1, tabs.TabCount)
            Assert.True(view.SessionState.Model.ActiveDocument.IsSome)

            view.Editor.CloseDocument()

            let welcome = view.FindControl<WelcomeView>("WelcomeView")
            Assert.Equal(0, tabs.TabCount)
            Assert.True(welcome.IsVisible)
            Assert.True(view.SessionState.Model.ActiveDocument.IsNone))
