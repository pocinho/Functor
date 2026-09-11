namespace Functor.Tests.Avalonia

open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Input
open Avalonia.Interactivity
open Functor.Application
open Functor.Avalonia.Controls
open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Avalonia.Views
open Xunit

module MainViewTests =

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
        let view = MainView()
        view.Editor.NewDocument()

        let editorControl = view.FindControl<EditorControl>("EditorControl")
        let welcome = view.FindControl<WelcomeView>("WelcomeView")

        Assert.True(editorControl.IsVisible)
        Assert.False(welcome.IsVisible)

        view.Editor.CloseDocument()

        Assert.False(editorControl.IsVisible)
        Assert.True(welcome.IsVisible)
        Assert.True(view.SessionState.Model.ActiveDocument.IsNone)

    [<AvaloniaFact>]
    let ``choosing New File from the welcome view creates a document and hides the welcome view`` () =
        let view = MainView()
        view.Editor.NewDocument()
        view.Editor.CloseDocument()

        let editorControl = view.FindControl<EditorControl>("EditorControl")
        let welcome = view.FindControl<WelcomeView>("WelcomeView")
        let newFileButton = welcome.FindControl<Button>("NewFileButton")

        newFileButton.RaiseEvent(RoutedEventArgs(Button.ClickEvent))

        Assert.True(view.SessionState.Model.ActiveDocument.IsSome)
        Assert.True(editorControl.IsVisible)
        Assert.False(welcome.IsVisible)

    [<AvaloniaFact>]
    let ``main view loads its host controls`` () =
        let view = MainView()

        Assert.NotNull(view.FindControl<Control>("EditorControl"))
        Assert.NotNull(view.FindControl<Control>("TabBar"))
        Assert.NotNull(view.FindControl<Control>("TabsPanel"))
        Assert.NotNull(view.FindControl<Control>("StatusBar"))

    [<AvaloniaFact>]
    let ``tab bar renders one tab per open document`` () =
        let view = MainView()
        view.Editor.NewDocument()
        view.Editor.NewDocument()

        let tabs = view.FindControl<StackPanel>("TabsPanel")

        Assert.Equal(2, tabs.Children.Count)

    [<AvaloniaFact>]
    let ``tab activation updates the active document projection`` () =
        let view = MainView()
        view.Editor.NewDocument()
        view.Editor.NewDocument()
        let firstDocumentId = view.SessionState.Workspace.TabOrder.Head
        let secondDocumentId = view.SessionState.Workspace.TabOrder.Tail.Head

        view.Editor.ActivateDocument(firstDocumentId)

        Assert.Equal(Some firstDocumentId, view.SessionState.Workspace.ActiveDocumentId)
        Assert.NotEqual(secondDocumentId, view.SessionState.Workspace.ActiveDocumentId.Value)

    [<AvaloniaFact>]
    let ``clicking a tab activates its document`` () =
        let view = MainView()
        view.Editor.NewDocument()
        view.Editor.NewDocument()
        let firstDocumentId = view.SessionState.Workspace.TabOrder.Head
        let tabs = view.FindControl<StackPanel>("TabsPanel")
        let firstTab = tabs.Children[0] :?> Button

        firstTab.RaiseEvent(RoutedEventArgs(Button.ClickEvent))

        Assert.Equal(Some firstDocumentId, view.SessionState.Workspace.ActiveDocumentId)

    [<AvaloniaFact>]
    let ``dirty state is reflected in the rendered tab label`` () =
        let view = MainView()
        view.Editor.NewDocument()
        view.Editor.DispatchApplicationCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "changed")))

        let tabs = view.FindControl<StackPanel>("TabsPanel")
        let tabButton = tabs.Children[0] :?> Button
        let content = tabButton.Content :?> StackPanel
        let label = content.Children[0] :?> TextBlock

        Assert.Equal("untitled *", label.Text)
