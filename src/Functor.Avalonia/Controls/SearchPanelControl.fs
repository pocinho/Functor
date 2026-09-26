namespace Functor.Avalonia.Controls

open System
open Avalonia
open Avalonia.Automation
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Layout
open Functor.Application
open Functor.Domain.Search

/// Search controls that translate user input into application commands.
type SearchPanelControl() as this =
    inherit UserControl()

    let queryBox = TextBox(PlaceholderText = "Search")
    let replacementBox = TextBox(PlaceholderText = "Replace")
    let caseSensitive = CheckBox(Content = "Case sensitive")
    let clearButton = Button(Content = "Clear")
    let findAllButton = Button(Content = "Find All")
    let findNextButton = Button(Content = "Find Next")
    let replaceButton = Button(Content = "Replace")
    let replaceAllButton = Button(Content = "Replace all")
    let countText = TextBlock()
    let resultsPanel = StackPanel(Spacing = 4.0)

    let resultsScroller =
        ScrollViewer(VerticalScrollBarVisibility = ScrollBarVisibility.Auto)

    let commandRequested = Event<AppCommand>()
    let mutable applying = false
    let mutable renderedMatches: SearchMatch list = []
    let renderedButtons = System.Collections.Generic.Dictionary<SearchMatch, Button>()
    let mutable currentSearch = SearchModel.create ()
    let mutable currentStatus: EditorStatus option = None

    let content =
        let buttons = StackPanel(Orientation = Orientation.Horizontal, Spacing = 6.0)
        buttons.Children.Add(clearButton) |> ignore
        buttons.Children.Add(findAllButton) |> ignore
        buttons.Children.Add(findNextButton) |> ignore
        buttons.Children.Add(replaceButton) |> ignore
        buttons.Children.Add(replaceAllButton) |> ignore

        let panel = Grid(RowSpacing = 8.0)
        panel.RowDefinitions.Add(RowDefinition(GridLength.Auto))
        panel.RowDefinitions.Add(RowDefinition(GridLength.Auto))
        panel.RowDefinitions.Add(RowDefinition(GridLength.Auto))
        panel.RowDefinitions.Add(RowDefinition(GridLength.Auto))
        panel.RowDefinitions.Add(RowDefinition(GridLength.Auto))
        panel.RowDefinitions.Add(RowDefinition(GridLength(1.0, GridUnitType.Star)))

        panel.Children.Add(queryBox) |> ignore
        Grid.SetRow(replacementBox, 1)
        panel.Children.Add(replacementBox) |> ignore
        Grid.SetRow(caseSensitive, 2)
        panel.Children.Add(caseSensitive) |> ignore
        Grid.SetRow(buttons, 3)
        panel.Children.Add(buttons) |> ignore
        Grid.SetRow(countText, 4)
        panel.Children.Add(countText) |> ignore
        resultsScroller.Content <- resultsPanel
        Grid.SetRow(resultsScroller, 5)
        panel.Children.Add(resultsScroller) |> ignore
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
        AutomationProperties.SetName(clearButton, "Clear search")
        AutomationProperties.SetName(findAllButton, "Find all search results")
        AutomationProperties.SetName(findNextButton, "Find next search result")
        AutomationProperties.SetName(replaceButton, "Replace current search result")
        AutomationProperties.SetName(replaceAllButton, "Replace all search results")

        findAllButton.Click.Add(fun _ ->
            commandRequested.Trigger(
                AppCommand.searchOptionsChanged
                    { Query = queryBox.Text
                      CaseSensitive = caseSensitive.IsChecked.GetValueOrDefault() }
            ))

        findNextButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.nextSearchResult))

        clearButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.clearSearch))
        replaceButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.replaceCurrentSearch replacementBox.Text))
        replaceAllButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.replaceAllSearch replacementBox.Text))

    member _.CommandRequested = commandRequested.Publish

    member _.ApplySearch(search: SearchModel) =
        currentSearch <- search
        applying <- true
        queryBox.Text <- search.Query |> Option.defaultValue search.Options.Query
        caseSensitive.IsChecked <- Nullable search.Options.CaseSensitive
        applying <- false

        let current =
            search.Index |> Option.map (fun index -> index + 1) |> Option.defaultValue 0

        countText.Text <-
            match search.Query, search.Matches with
            | None, _ -> ""
            | Some _, [] -> "No results"
            | Some _, _ -> sprintf "%d of %d" current search.Matches.Length

        clearButton.IsEnabled <- search.Query.IsSome

        let canReuse =
            search.Matches.Length >= renderedMatches.Length
            && List.forall2 (=) renderedMatches (search.Matches |> List.take renderedMatches.Length)

        if not canReuse then
            resultsPanel.Children.Clear()
            renderedButtons.Clear()
            renderedMatches <- []

        search.Matches
        |> List.skip renderedMatches.Length
        |> List.iter (fun matchValue ->
            let path = matchValue.Path |> Option.defaultValue matchValue.Name

            let label =
                sprintf "%s:%d:%d  %s" path (matchValue.Line + 1) (matchValue.Column + 1) matchValue.Preview

            let resultButton =
                Button(Content = label, HorizontalContentAlignment = HorizontalAlignment.Left)

            resultButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.activateSearchResult matchValue))
            renderedButtons[matchValue] <- resultButton
            resultsPanel.Children.Add(resultButton) |> ignore)

        renderedMatches <- search.Matches

        search.Matches
        |> List.iteri (fun index matchValue ->
            renderedButtons[matchValue].Classes.Set("selected", search.Index = Some index))

        match currentStatus with
        | Some status when status.Error.IsSome || status.Message.IsSome ->
            countText.Text <- status.Error |> Option.orElse status.Message |> Option.get
        | Some _ when search.IsDirty && search.Query.IsSome -> countText.Text <- "Searching..."
        | _ -> ()

    member _.ApplyStatus(status: EditorStatus) =
        currentStatus <- Some status
        let statusMessage = status.Error |> Option.orElse status.Message

        match statusMessage, currentSearch.Query, currentSearch.IsDirty, currentSearch.Matches with
        | Some error, _, _, _ -> countText.Text <- error
        | None, Some _, true, _ -> countText.Text <- "Searching..."
        | None, Some _, false, [] -> countText.Text <- "No results"
        | None, None, _, _ -> countText.Text <- ""
        | None, Some _, false, _ ->
            let current =
                currentSearch.Index
                |> Option.map (fun index -> index + 1)
                |> Option.defaultValue 0

            countText.Text <- sprintf "%d of %d" current currentSearch.Matches.Length

    member _.FocusQuery() = queryBox.Focus() |> ignore

    member _.FocusReplacement() = replacementBox.Focus() |> ignore
