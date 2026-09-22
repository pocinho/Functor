namespace Functor.Avalonia.Controls

open System
open System.Threading
open System.Threading.Tasks
open Avalonia
open Avalonia.Automation
open Avalonia.Controls
open Avalonia.Controls.Primitives
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
    let clearButton = Button(Content = "Clear")
    let replaceButton = Button(Content = "Replace")
    let replaceAllButton = Button(Content = "Replace all")
    let clearHistoryButton = Button(Content = "Clear history")
    let countText = TextBlock()
    let resultsPanel = StackPanel(Spacing = 4.0)

    let resultsScroller =
        ScrollViewer(VerticalScrollBarVisibility = ScrollBarVisibility.Auto)

    let commandRequested = Event<AppCommand>()
    let mutable applying = false
    let mutable queryDebounce = new CancellationTokenSource()
    let mutable renderedMatches: SearchMatch list = []
    let renderedButtons = System.Collections.Generic.Dictionary<SearchMatch, Button>()

    let onUiThread action =
        Dispatcher.UIThread.Post(Action action) |> ignore

    let scheduleQuerySearch query =
        queryDebounce.Cancel()
        queryDebounce.Dispose()
        queryDebounce <- new CancellationTokenSource()
        let cancellationToken = queryDebounce.Token

        Async.StartImmediate(
            async {
                try
                    do! Task.Delay(250, cancellationToken) |> Async.AwaitTask

                    if not cancellationToken.IsCancellationRequested then
                        onUiThread (fun () ->
                            if
                                not cancellationToken.IsCancellationRequested
                                && not applying
                                && not (isNull (TopLevel.GetTopLevel(this)))
                            then
                                commandRequested.Trigger(AppCommand.searchQueryChanged query))
                with :? OperationCanceledException ->
                    ()
            },
            cancellationToken
        )

    let content =
        let buttons = StackPanel(Orientation = Orientation.Horizontal, Spacing = 6.0)
        buttons.Children.Add(clearButton) |> ignore
        buttons.Children.Add(replaceButton) |> ignore
        buttons.Children.Add(replaceAllButton) |> ignore
        buttons.Children.Add(clearHistoryButton) |> ignore

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
        AutomationProperties.SetName(replaceButton, "Replace current search result")
        AutomationProperties.SetName(replaceAllButton, "Replace all search results")
        AutomationProperties.SetName(clearHistoryButton, "Clear search history")

        queryBox.TextChanged.Add(fun args ->
            if not applying then
                onUiThread (fun () ->
                    if not applying && not (isNull (TopLevel.GetTopLevel(this))) then
                        scheduleQuerySearch queryBox.Text))

        caseSensitive.IsCheckedChanged.Add(fun _ ->
            if not applying then
                onUiThread (fun () ->
                    if not applying && not (isNull (TopLevel.GetTopLevel(this))) then
                        commandRequested.Trigger(
                            AppCommand.searchOptionsChanged
                                { Query = queryBox.Text
                                  CaseSensitive = caseSensitive.IsChecked.GetValueOrDefault() }
                        )))

        clearButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.clearSearch))
        replaceButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.replaceCurrentSearch replacementBox.Text))
        replaceAllButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.replaceAllSearch replacementBox.Text))
        clearHistoryButton.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.clearSearchHistory))

        resultsScroller.PropertyChanged.Add(
            (fun args ->
                if
                    args.Property = ScrollViewer.OffsetProperty
                    && resultsScroller.Extent.Height > resultsScroller.Viewport.Height
                    && resultsScroller.Offset.Y + resultsScroller.Viewport.Height
                       >= resultsScroller.Extent.Height - 24.0
                then
                    commandRequested.Trigger(AppCommand.searchWorkspaceMore))
        )

    member _.CommandRequested = commandRequested.Publish

    member _.ApplySearch(search: SearchModel) =
        applying <- true
        queryBox.Text <- search.Query |> Option.defaultValue search.Options.Query
        caseSensitive.IsChecked <- Nullable search.Options.CaseSensitive
        applying <- false

        let current =
            search.Index |> Option.map (fun index -> index + 1) |> Option.defaultValue 0

        countText.Text <- sprintf "%d of %d" current search.Matches.Length
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
