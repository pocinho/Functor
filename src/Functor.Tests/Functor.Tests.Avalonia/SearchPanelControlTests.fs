namespace Functor.Tests.Avalonia

open System
open Avalonia.Controls
open Avalonia.Interactivity
open Functor.Avalonia.Controls
open Functor.Application
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Search
open Xunit

type SearchPanelControlTests() =
    [<Fact>]
    member _.``open files button dispatches open documents search``() =
        let panel = SearchPanelControl()
        let commands = ResizeArray<AppCommand>()
        panel.CommandRequested.Add(commands.Add)

        let content = panel.Content :?> StackPanel
        let buttons = content.Children[2] :?> StackPanel
        let openDocumentsButton = buttons.Children[2] :?> Button
        openDocumentsButton.RaiseEvent(RoutedEventArgs(Button.ClickEvent))

        Assert.Contains(AppCommand.searchOpenDocuments, commands)

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

        let content = panel.Content :?> StackPanel
        let results = content.Children[4] :?> StackPanel
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
