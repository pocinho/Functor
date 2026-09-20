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

    let tabsPanel =
        StackPanel(
            Orientation = Orientation.Horizontal,
            Spacing = (ThemeShapeDensity.fromUiTheme UiThemeDefaults.defaultTheme).DocumentTabSpacing
        )

    let scrollViewer =
        ScrollViewer(
            HorizontalScrollBarVisibility = ScrollBarVisibility.Hidden,
            VerticalScrollBarVisibility = ScrollBarVisibility.Hidden
        )

    let defaultTabNavigationButtonWidth =
        (ThemeShapeDensity.fromUiTheme UiThemeDefaults.defaultTheme).TabNavigationButtonWidth

    let scrollLeftButton =
        Button(Content = "<", Width = defaultTabNavigationButtonWidth)

    let scrollRightButton =
        Button(Content = ">", Width = defaultTabNavigationButtonWidth)

    let documentActivated = Event<DocumentId>()
    let documentCloseRequested = Event<DocumentId>()
    let mutable renderedTabIds: DocumentId list = []
    let mutable tabControls: Map<DocumentId, Button * TextBlock> = Map.empty

    let updateScrollButtons () =
        let maximumOffset =
            max 0.0 (scrollViewer.Extent.Width - scrollViewer.Viewport.Width)

        let currentOffset = scrollViewer.Offset.X
        scrollLeftButton.IsEnabled <- currentOffset > 0.5
        scrollRightButton.IsEnabled <- currentOffset < maximumOffset - 0.5

    let scrollBy direction =
        let step = max 80.0 (scrollViewer.Viewport.Width * 0.8)

        let maximumOffset =
            max 0.0 (scrollViewer.Extent.Width - scrollViewer.Viewport.Width)

        let nextOffset =
            min maximumOffset (max 0.0 (scrollViewer.Offset.X + direction * step))

        scrollViewer.Offset <- Vector(nextOffset, scrollViewer.Offset.Y)
        updateScrollButtons ()

    do
        scrollViewer.Content <- tabsPanel
        scrollLeftButton.Classes.Add("tab-scroll-button")
        scrollRightButton.Classes.Add("tab-scroll-button")
        ToolTip.SetTip(scrollLeftButton, "Scroll tabs left")
        ToolTip.SetTip(scrollRightButton, "Scroll tabs right")
        scrollLeftButton.HorizontalContentAlignment <- HorizontalAlignment.Center
        scrollRightButton.HorizontalContentAlignment <- HorizontalAlignment.Center
        scrollLeftButton.Click.Add(fun _ -> scrollBy -1.0)
        scrollRightButton.Click.Add(fun _ -> scrollBy 1.0)

        scrollViewer.PropertyChanged.Add(fun args ->
            if
                args.Property = ScrollViewer.OffsetProperty
                || args.Property = ScrollViewer.ExtentProperty
                || args.Property = ScrollViewer.ViewportProperty
            then
                updateScrollButtons ())

        let root = Grid(ColumnDefinitions = ColumnDefinitions("Auto,*,Auto"))
        root.Children.Add(scrollLeftButton) |> ignore
        Grid.SetColumn(scrollViewer, 1)
        root.Children.Add(scrollViewer) |> ignore
        Grid.SetColumn(scrollRightButton, 2)
        root.Children.Add(scrollRightButton) |> ignore
        this.Content <- root
        updateScrollButtons ()

    member _.DocumentActivated = documentActivated.Publish

    member _.DocumentCloseRequested = documentCloseRequested.Publish

    member _.TabCount = tabsPanel.Children.Count

    member _.GetTab(index: int) = tabsPanel.Children[index] :?> Button

    member _.ApplyUiTheme(value: UiThemeDefaults) =
        uiTheme <- value
        let shape = ThemeShapeDensity.fromUiTheme uiTheme
        tabsPanel.Spacing <- shape.DocumentTabSpacing
        scrollLeftButton.Width <- shape.TabNavigationButtonWidth
        scrollRightButton.Width <- shape.TabNavigationButtonWidth
        updateScrollButtons ()

        tabControls
        |> Map.iter (fun _ (tabButton, _) ->
            tabButton.Padding <- Thickness(shape.DocumentTabPaddingHorizontal, shape.DocumentTabPaddingVertical)
            tabButton.BorderThickness <- Thickness(shape.BorderWidth)

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
        let shape = ThemeShapeDensity.fromUiTheme uiTheme
        tabsPanel.Spacing <- shape.DocumentTabSpacing
        let tabIds = tabs |> List.map (fun tab -> tab.DocumentId)

        if tabIds <> renderedTabIds then
            tabsPanel.Children.Clear()

            tabControls <-
                tabs
                |> List.map (fun tab ->
                    let tabButton =
                        Button(
                            Padding = Thickness(shape.DocumentTabPaddingHorizontal, shape.DocumentTabPaddingVertical),
                            MinHeight = shape.DocumentTabMinHeight,
                            BorderThickness = Thickness(shape.BorderWidth),
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
                            Width = shape.DocumentTabCloseButtonSize,
                            Height = shape.DocumentTabCloseButtonSize,
                            Padding = Thickness(0)
                        )

                    let content =
                        StackPanel(Orientation = Orientation.Horizontal, Spacing = shape.DocumentTabContentSpacing)

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

        updateScrollButtons ()

        tabs
        |> List.iter (fun tab ->
            match tabControls |> Map.tryFind tab.DocumentId with
            | Some(tabButton, label) ->
                label.Text <- if tab.IsDirty then tab.Name + " *" else tab.Name
                tabButton.Classes.Set("selected", tab.IsActive)
            | None -> ())
