namespace Functor.Avalonia.Controls

open Avalonia
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Layout
open Avalonia.Media
open Functor.Application
open Functor.Domain.Document
open Functor.Workspace

type DocumentListView() as this =
    inherit UserControl()

    let mutable uiTheme = UiThemeDefaults.defaultTheme
    let tabsPanel = StackPanel(Orientation = Orientation.Horizontal, Spacing = 2.0)
    let documentActivated = Event<DocumentId>()
    let documentCloseRequested = Event<DocumentId>()
    let mutable renderedTabIds: DocumentId list = []
    let mutable tabControls: Map<DocumentId, Button * TextBlock> = Map.empty

    do
        let scrollViewer =
            ScrollViewer(
                HorizontalScrollBarVisibility = ScrollBarVisibility.Auto,
                VerticalScrollBarVisibility = ScrollBarVisibility.Hidden
            )

        scrollViewer.Content <- tabsPanel
        this.Content <- scrollViewer

    member _.DocumentActivated = documentActivated.Publish

    member _.DocumentCloseRequested = documentCloseRequested.Publish

    member _.TabCount = tabsPanel.Children.Count

    member _.GetTab(index: int) = tabsPanel.Children[index] :?> Button

    member _.ApplyUiTheme(value: UiThemeDefaults) =
        uiTheme <- value

        tabControls
        |> Map.iter (fun _ (tabButton, _) ->
            match tabButton.Content with
            | :? StackPanel as content when content.Children.Count > 1 ->
                match content.Children[1] with
                | :? Button as closeButton ->
                    match closeButton.Content with
                    | :? TextBlock as icon ->
                        icon.FontFamily <- FontFamily(uiTheme.IconFontFamily)
                        icon.FontSize <- uiTheme.TabCloseIconSize
                    | _ -> ()
                | _ -> ()
            | _ -> ())

    member _.ApplyTabs(tabs: WorkspaceTabProjection list) =
        let tabIds = tabs |> List.map (fun tab -> tab.DocumentId)

        if tabIds <> renderedTabIds then
            tabsPanel.Children.Clear()

            tabControls <-
                tabs
                |> List.map (fun tab ->
                    let tabButton =
                        Button(
                            Padding = Thickness(10, 4),
                            MinHeight = UiThemeDefaults.documentTabMinHeight,
                            BorderThickness = Thickness(1),
                            HorizontalContentAlignment = HorizontalAlignment.Stretch
                        )

                    tabButton.Classes.Add("tab")

                    let label = TextBlock()

                    let closeButton =
                        Button(
                            Content =
                                TextBlock(
                                    Text = "\uE8BB",
                                    FontFamily = new FontFamily(uiTheme.IconFontFamily),
                                    FontSize = uiTheme.TabCloseIconSize,
                                    VerticalAlignment = VerticalAlignment.Center,
                                    HorizontalAlignment = HorizontalAlignment.Center
                                ),
                            Width = UiThemeDefaults.documentTabCloseButtonSize,
                            Height = UiThemeDefaults.documentTabCloseButtonSize,
                            Padding = Thickness(0)
                        )

                    let content = StackPanel(Orientation = Orientation.Horizontal, Spacing = 8.0)
                    content.Children.Add(label) |> ignore
                    content.Children.Add(closeButton) |> ignore
                    tabButton.Content <- content

                    tabButton.Click.Add(fun _ -> documentActivated.Trigger(tab.DocumentId))

                    closeButton.Click.Add(fun args ->
                        args.Handled <- true
                        documentCloseRequested.Trigger(tab.DocumentId))

                    tabsPanel.Children.Add(tabButton) |> ignore
                    tab.DocumentId, (tabButton, label))
                |> Map.ofList

            renderedTabIds <- tabIds

        tabs
        |> List.iter (fun tab ->
            match tabControls |> Map.tryFind tab.DocumentId with
            | Some(tabButton, label) ->
                label.Text <- if tab.IsDirty then tab.Name + " *" else tab.Name
                tabButton.Classes.Set("selected", tab.IsActive)
            | None -> ())
