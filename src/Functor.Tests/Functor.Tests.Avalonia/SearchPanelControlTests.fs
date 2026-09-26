namespace Functor.Tests.Avalonia

open System
open System.Threading.Tasks
open Avalonia.Automation
open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Interactivity
open Avalonia.Threading
open Functor.Avalonia.Controls
open Functor.Application
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Search
open Xunit

type SearchPanelControlTests() =
    [<AvaloniaFact>]
    member _.``query changes do not dispatch a search command``() =
        let panel = SearchPanelControl()
        let window = Window(Content = panel)
        let commands = ResizeArray<AppCommand>()
        panel.CommandRequested.Add(commands.Add)
        window.Show()

        let content = panel.Content :?> Grid
        let queryBox = content.Children[0] :?> TextBox
        queryBox.Text <- "needle"

        task {
            do! Task.Delay(350)
            Dispatcher.UIThread.RunJobs()

            Assert.Empty(commands)
            window.Close()
        }

    [<AvaloniaFact>]
    member _.``find all dispatches the current search options``() =
        let panel = SearchPanelControl()
        let window = Window(Content = panel)
        let commands = ResizeArray<AppCommand>()
        panel.CommandRequested.Add(commands.Add)
        window.Show()

        let content = panel.Content :?> Grid
        let queryBox = content.Children[0] :?> TextBox
        let caseSensitive = content.Children[2] :?> CheckBox
        queryBox.Text <- "Needle"
        Dispatcher.UIThread.RunJobs()
        caseSensitive.IsChecked <- Nullable true
        Dispatcher.UIThread.RunJobs()

        let buttons = content.Children[3] :?> StackPanel
        (buttons.Children[1] :?> Button).RaiseEvent(RoutedEventArgs(Button.ClickEvent))

        Assert.Contains(
            AppCommand.searchOptionsChanged
                { Query = "Needle"
                  CaseSensitive = true },
            commands
        )

        window.Close()

    [<Fact>]
    member _.``search panel distinguishes empty search from no results``() =
        let panel = SearchPanelControl()
        let content = panel.Content :?> Grid
        let countText = content.Children[4] :?> TextBlock

        panel.ApplySearch(SearchModel.create ())
        Assert.Equal("", countText.Text)

        panel.ApplySearch
            { SearchModel.create () with
                Query = Some "missing" }

        Assert.Equal("No results", countText.Text)

    [<Fact>]
    member _.``search panel presents searching and error states``() =
        let panel = SearchPanelControl()
        let content = panel.Content :?> Grid
        let countText = content.Children[4] :?> TextBlock

        panel.ApplySearch
            { SearchModel.create () with
                Query = Some "term"
                IsDirty = true }

        panel.ApplyStatus
            { Line = 1
              Column = 1
              FileName = "untitled"
              FileType = "Plain Text"
              IsDirty = false
              Message = None
              Error = None }

        Assert.Equal("Searching...", countText.Text)

        panel.ApplyStatus
            { Line = 1
              Column = 1
              FileName = "untitled"
              FileType = "Plain Text"
              IsDirty = false
              Message = Some "Unable to read workspace file."
              Error = None }

        Assert.Equal("Unable to read workspace file.", countText.Text)

    [<Fact>]
    member _.``search actions expose find and replacement commands``() =
        let panel = SearchPanelControl()
        let content = panel.Content :?> Grid
        let buttons = content.Children[3] :?> StackPanel

        Assert.Equal(5, buttons.Children.Count)

    [<Fact>]
    member _.``search controls expose accessible names``() =
        let panel = SearchPanelControl()
        let content = panel.Content :?> Grid
        let buttons = content.Children[3] :?> StackPanel

        Assert.Equal("Search query", AutomationProperties.GetName(content.Children[0]))
        Assert.Equal("Replacement text", AutomationProperties.GetName(content.Children[1]))
        Assert.Equal("Case sensitive search", AutomationProperties.GetName(content.Children[2]))
        Assert.Equal("Clear search", AutomationProperties.GetName(buttons.Children[0]))
        Assert.Equal("Find all search results", AutomationProperties.GetName(buttons.Children[1]))
        Assert.Equal("Find next search result", AutomationProperties.GetName(buttons.Children[2]))

    [<Fact>]
    member _.``replacement buttons dispatch replacement commands``() =
        let panel = SearchPanelControl()
        let commands = ResizeArray<AppCommand>()
        panel.CommandRequested.Add(commands.Add)

        let content = panel.Content :?> Grid
        let replacementBox = content.Children[1] :?> TextBox
        replacementBox.Text <- "word"
        let buttons = content.Children[3] :?> StackPanel
        (buttons.Children[3] :?> Button).RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        (buttons.Children[4] :?> Button).RaiseEvent(RoutedEventArgs(Button.ClickEvent))

        Assert.Contains(AppCommand.replaceCurrentSearch "word", commands)
        Assert.Contains(AppCommand.replaceAllSearch "word", commands)

    [<Fact>]
    member _.``search result activation dispatches the selected match``() =
        let panel = SearchPanelControl()
        let documentId = Guid.NewGuid()

        let matchValue =
            { DocumentId = documentId
              Path = Some "C:\work\result.fs"
              Name = "result.fs"
              Line = 2
              Column = 3
              Length = 4
              Range =
                { Start = { Line = 2; Column = 3 }
                  End = { Line = 2; Column = 7 } }
              Preview = "needle" }

        let search =
            { SearchModel.create () with
                Query = Some "needle"
                Options =
                    { Query = "needle"
                      CaseSensitive = false }
                Matches = [ matchValue ]
                Index = Some 0 }

        let commands = ResizeArray<AppCommand>()
        panel.CommandRequested.Add(commands.Add)
        panel.ApplySearch search

        let content = panel.Content :?> Grid
        let resultsScroller = content.Children[5] :?> ScrollViewer
        let results = resultsScroller.Content :?> StackPanel
        let resultButton = results.Children[0] :?> Button
        resultButton.RaiseEvent(RoutedEventArgs(Button.ClickEvent))

        match
            commands
            |> Seq.tryFind (function
                | SearchResultActivated _ -> true
                | _ -> false)
        with
        | Some(SearchResultActivated result) -> Assert.Equal(matchValue, result)
        | _ -> Assert.True(false, "Expected search result activation command.")

    [<Fact>]
    member _.``reuses result controls when only the active match changes``() =
        let panel = SearchPanelControl()

        let firstMatch =
            { DocumentId = Guid.NewGuid()
              Path = Some "C:\work\first.fs"
              Name = "first.fs"
              Line = 0
              Column = 0
              Length = 4
              Range =
                { Start = { Line = 0; Column = 0 }
                  End = { Line = 0; Column = 4 } }
              Preview = "term" }

        let secondMatch =
            { firstMatch with
                DocumentId = Guid.NewGuid()
                Path = Some "C:\work\second.fs"
                Name = "second.fs" }

        let options =
            { Query = "term"
              CaseSensitive = false }

        let initialSearch =
            { SearchModel.create () with
                Query = Some "term"
                Options = options
                Matches = [ firstMatch; secondMatch ]
                Index = Some 0 }

        let updatedSearch = { initialSearch with Index = Some 1 }

        panel.ApplySearch initialSearch
        let content = panel.Content :?> Grid
        let results = (content.Children[5] :?> ScrollViewer).Content :?> StackPanel
        let firstButton = results.Children[0]
        let secondButton = results.Children[1]

        Assert.True(firstButton.Classes.Contains("selected"))
        Assert.False(secondButton.Classes.Contains("selected"))

        panel.ApplySearch updatedSearch

        Assert.Same(firstButton, results.Children[0])
        Assert.False(firstButton.Classes.Contains("selected"))
        Assert.True(secondButton.Classes.Contains("selected"))
