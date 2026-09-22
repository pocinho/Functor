namespace Functor.Avalonia.Controls

open System
open Avalonia.Automation
open Avalonia.Controls
open Avalonia.Layout
open Avalonia.Threading
open Functor.Application
open Functor.Domain.Search

/// Search controls that translate user input into application commands.
type SearchPanelControl() as this =
    inherit UserControl()

    let queryBox = TextBox(PlaceholderText = "Search")
    let replacementBox = TextBox(PlaceholderText = "Replace")
    let caseSensitive = CheckBox(Content = "Case sensitive")
    let previousButton = Button(Content = "Previous")
    let nextButton = Button(Content = "Next")
    let openDocumentsButton = Button(Content = "Open files")
    let clearButton = Button(Content = "Clear")
    let replaceButton = Button(Content = "Replace")
    let replaceAllButton = Button(Content = "Replace all")
    let clearHistoryButton = Button(Content = "Clear history")
    let countText = TextBlock()
    let resultsPanel = StackPanel(Spacing = 4.0)
    let commandRequested = Event<AppCommand>()
    let mutable applying = false

    let onUiThread action =
        Dispatcher.UIThread.Post(Action action) |> ignore

    let content =
        let buttons = StackPanel(Orientation = Orientation.Horizontal, Spacing = 6.0)
        buttons.Children.Add(previousButton) |> ignore
        buttons.Children.Add(nextButton) |> ignore
        buttons.Children.Add(openDocumentsButton) |> ignore
        buttons.Children.Add(clearButton) |> ignore
        buttons.Children.Add(replaceButton) |> ignore
        buttons.Children.Add(replaceAllButton) |> ignore
        buttons.Children.Add(clearHistoryButton) |> ignore

        let panel = StackPanel(Spacing = 8.0)
        panel.Children.Add(queryBox) |> ignore
        panel.Children.Add(replacementBox) |> ignore
        panel.Children.Add(caseSensitive) |> ignore
        panel.Children.Add(buttons) |> ignore
        panel.Children.Add(countText) |> ignore
        panel.Children.Add(resultsPanel) |> ignore
        panel

    do
        this.Content <- content
        this.Focusable <- true
        this.IsHitTestVisible <- true
        queryBox.Focusable <- true
        queryBox.IsHitTestVisible <- true
        replacementBox.IsHitTestVisible <- true
        caseSensitive.IsHitTestVisible <- true

        AutomationProperties.SetName(queryBox, "Search query")
        AutomationProperties.SetName(replacementBox, "Replacement text")
        AutomationProperties.SetName(caseSensitive, "Case sensitive search")
        AutomationProperties.SetName(previousButton, "Previous search result")
        AutomationProperties.SetName(nextButton, "Next search result")
        AutomationProperties.SetName(openDocumentsButton, "Search open documents")
        AutomationProperties.SetName(clearButton, "Clear search")
        AutomationProperties.SetName(replaceButton, "Replace current search result")
        AutomationProperties.SetName(replaceAllButton, "Replace all search results")
        AutomationProperties.SetName(clearHistoryButton, "Clear search history")

        queryBox.TextChanged.Add(fun args ->
            if not applying then
                onUiThread (fun () ->
                    if not applying && not (isNull (TopLevel.GetTopLevel(this))) then
                        commandRequested.Trigger(AppCommand.searchQueryChanged queryBox.Text)))

        caseSensitive.IsCheckedChanged.Add(fun _ ->
            if not applying then
                onUiThread (fun () ->
                    if not applying && not (isNull (TopLevel.GetTopLevel(this))) then
                        commandRequested.Trigger(
                            AppCommand.searchOptionsChanged
                                { Query = queryBox.Text
                                  CaseSensitive = caseSensitive.IsChecked.GetValueOrDefault() }
                        )))

        previousButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.previousSearchResult))
        nextButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.nextSearchResult))
        openDocumentsButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.searchOpenDocuments))
        clearButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.clearSearch))
        replaceButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.replaceCurrentSearch replacementBox.Text))
        replaceAllButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.replaceAllSearch replacementBox.Text))
        clearHistoryButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.clearSearchHistory))

    member _.CommandRequested = commandRequested.Publish

    member _.ApplySearch(search: SearchModel) =
        applying <- true
        queryBox.Text <- search.Query |> Option.defaultValue search.Options.Query
        caseSensitive.IsChecked <- Nullable search.Options.CaseSensitive
        applying <- false

        let current =
            search.Index |> Option.map (fun index -> index + 1) |> Option.defaultValue 0

        countText.Text <- sprintf "%d of %d" current search.Matches.Length
        previousButton.IsEnabled <- search.Matches.Length > 0
        nextButton.IsEnabled <- search.Matches.Length > 0
        clearButton.IsEnabled <- search.Query.IsSome

        resultsPanel.Children.Clear()

        search.Matches
        |> List.iteri (fun index matchValue ->
            let path = matchValue.Path |> Option.defaultValue matchValue.Name

            let label =
                sprintf "%s:%d:%d  %s" path (matchValue.Line + 1) (matchValue.Column + 1) matchValue.Preview

            let resultButton =
                Button(Content = label, HorizontalContentAlignment = HorizontalAlignment.Left)

            resultButton.Classes.Set("selected", search.Index = Some index)
            resultButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.activateSearchResult matchValue))
            resultsPanel.Children.Add(resultButton) |> ignore)
