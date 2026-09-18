namespace Functor.Avalonia.Views

open System
open Avalonia
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Layout
open Avalonia.Media
open Avalonia.Markup.Xaml
open Avalonia.Threading
open Functor.Application
open Functor.Avalonia
open Functor.Avalonia.Controls
open Functor.Domain.Document
open Functor.Workspace

type ShellHostView() as this =
    inherit UserControl()

    let editor = lazy (this.FindControl<EditorControl>("EditorControl"))
    let welcomeView = lazy (this.FindControl<WelcomeView>("WelcomeView"))
    let verticalScrollBar = lazy (this.FindControl<ScrollBar>("VerticalScrollBar"))
    let horizontalScrollBar = lazy (this.FindControl<ScrollBar>("HorizontalScrollBar"))
    let statusBar = lazy (this.FindControl<Border>("StatusBar"))
    let positionText = lazy (this.FindControl<TextBlock>("PositionText"))
    let fileTypeText = lazy (this.FindControl<TextBlock>("FileTypeText"))
    let messageText = lazy (this.FindControl<TextBlock>("MessageText"))
    let fileNameText = lazy (this.FindControl<TextBlock>("FileNameText"))
    let dirtyText = lazy (this.FindControl<TextBlock>("DirtyText"))
    let tabBar = lazy (this.FindControl<Border>("TabBar"))
    let tabsPanel = lazy (this.FindControl<DocumentListView>("TabsPanel"))
    let sidePanelHost = lazy (this.FindControl<SidePanelView>("SidePanelHost"))

    let auxiliaryPanelHost =
        lazy (this.FindControl<SidePanelView>("AuxiliaryPanelHost"))

    let mutable model = ShellModel.initial
    let mutable subscriptions: IDisposable list = []
    let mutable confirmationOpen = false

    let colorFromArgb (argb: uint32) =
        Color.FromArgb(byte (argb >>> 24), byte (argb >>> 16), byte (argb >>> 8), byte argb)

    let updateEmptyStateFromProjection hasActiveDocument =
        editor.Value.IsVisible <- hasActiveDocument
        welcomeView.Value.IsVisible <- not hasActiveDocument

    let updateScrollBarFromProjection (scroll: ShellScrollPresentation) =
        let verticalScrollBar = verticalScrollBar.Value
        let horizontalScrollBar = horizontalScrollBar.Value

        verticalScrollBar.Maximum <- scroll.VerticalMaximum
        verticalScrollBar.ViewportSize <- scroll.VerticalViewport
        verticalScrollBar.LargeChange <- max 1.0 scroll.VerticalViewport
        verticalScrollBar.IsEnabled <- scroll.VerticalMaximum > 0.0
        verticalScrollBar.Value <- scroll.VerticalOffset
        horizontalScrollBar.Maximum <- scroll.HorizontalMaximum
        horizontalScrollBar.ViewportSize <- scroll.HorizontalViewport
        horizontalScrollBar.LargeChange <- max 1.0 scroll.HorizontalViewport
        horizontalScrollBar.IsEnabled <- scroll.HorizontalMaximum > 0.0
        horizontalScrollBar.Value <- scroll.HorizontalOffset

    let updateScrollBar () =
        updateScrollBarFromProjection (ShellProjection.fromEditor editor.Value).Scroll

    let updateEditorStatus (status: Functor.Application.EditorStatus) =
        positionText.Value.Text <- sprintf "Ln %d, Col %d" status.Line status.Column
        fileTypeText.Value.Text <- status.FileType
        messageText.Value.Text <- status.Error |> Option.orElse status.Message |> Option.defaultValue ""
        fileNameText.Value.Text <- status.FileName
        dirtyText.Value.Text <- if status.IsDirty then "Modified" else ""

    let updateTabsFromProjection tabs =
        let palette = editor.Value.ThemeSettings.ThemeSource.Resolve()
        let foreground = SolidColorBrush(colorFromArgb palette.Foreground)

        let border =
            SolidColorBrush(colorFromArgb (palette.GutterSeparator |> Option.defaultValue palette.Foreground))

        let selected = SolidColorBrush(colorFromArgb palette.Selection)
        let background = SolidColorBrush(colorFromArgb palette.GutterBackground)

        tabsPanel.Value.ApplyTabs tabs foreground border selected background

    let updateTabs (state: AppSessionState) =
        updateTabsFromProjection (WorkspaceProjection.tabs state.Workspace)

    let updateEmptyState (state: AppSessionState) =
        updateEmptyStateFromProjection state.Model.ActiveDocument.IsSome

    let updateToolRail () =
        let activeTool = model.Layout.ActiveTool
        let isPanelOpen = model.Layout.IsToolPanelOpen

        this
            .FindControl<Button>("WorkspaceToolButton")
            .Classes.Set("selected", isPanelOpen && activeTool = Some WorkspaceTool)

        this
            .FindControl<Button>("SearchToolButton")
            .Classes.Set("selected", isPanelOpen && activeTool = Some SearchTool)

    let updateToolRailMetadata () =
        let applyDescriptor controlName descriptor =
            let button = this.FindControl<Button>(controlName)
            button.Content <- descriptor.Glyph
            button.SetValue(ToolTip.TipProperty, descriptor.AccessibilityName)

        applyDescriptor "WorkspaceToolButton" (ToolDescriptor.get WorkspaceTool)
        |> ignore

        applyDescriptor "SearchToolButton" (ToolDescriptor.get SearchTool) |> ignore

    let showDiscardDialog () =
        match TopLevel.GetTopLevel(this) with
        | :? Window as owner ->
            confirmationOpen <- true

            let dialog =
                Window(
                    Title = "Unsaved changes",
                    Width = 420.0,
                    Height = 160.0,
                    CanResize = false,
                    WindowStartupLocation = WindowStartupLocation.CenterOwner
                )

            let message = TextBlock(Text = "This document has unsaved changes. Discard them?")
            let discardButton = Button(Content = "Discard")
            let cancelButton = Button(Content = "Cancel")
            let buttons = StackPanel(Orientation = Orientation.Horizontal, Spacing = 8.0)
            let content = StackPanel(Spacing = 16.0, Margin = Thickness(16.0))

            buttons.Children.Add(discardButton) |> ignore
            buttons.Children.Add(cancelButton) |> ignore
            content.Children.Add(message) |> ignore
            content.Children.Add(buttons) |> ignore
            dialog.Content <- content

            discardButton.Click.Add(fun _ ->
                editor.Value.ConfirmDiscardChanges()
                dialog.Close())

            cancelButton.Click.Add(fun _ ->
                editor.Value.CancelPendingOperation()
                dialog.Close())

            dialog.Closed.Add(fun _ -> confirmationOpen <- false)
            dialog.ShowDialog(owner) |> ignore
        | _ -> editor.Value.CancelPendingOperation()

    let refresh () =
        let input = ShellProjection.fromEditor editor.Value

        ShellView.applyModel
            this
            sidePanelHost.Value
            auxiliaryPanelHost.Value
            model
            input
            (fun documentId -> editor.Value.ActivateDocument(documentId))
            (fun path -> editor.Value.DispatchApplicationCommand(AppCommand.openDocument path))
        |> ignore

        updateToolRail ()

    let refreshOnUiThread () =
        if Dispatcher.UIThread.CheckAccess() then
            refresh ()
        else
            Dispatcher.UIThread.Post(Action refresh) |> ignore

    let disposeSubscriptions () =
        subscriptions |> List.iter (fun subscription -> subscription.Dispose())
        subscriptions <- []

    let attachSubscriptions () =
        if subscriptions.IsEmpty then
            let editorControl = editor.Value

            subscriptions <-
                [ editorControl.StateChanged.Subscribe(fun _ -> refreshOnUiThread ())
                  editorControl.EditorStatusChanged.Subscribe(fun _ -> refreshOnUiThread ())
                  editorControl.ScrollStateChanged.Subscribe(fun _ -> refreshOnUiThread ()) ]

            refreshOnUiThread ()

    do
        this.InitializeComponent()

        let editor = editor.Value
        let verticalScrollBar = verticalScrollBar.Value
        let horizontalScrollBar = horizontalScrollBar.Value

        editor.StatusChanged.Add(fun status ->
            if status.PendingAction.IsSome && not confirmationOpen then
                showDiscardDialog ())

        verticalScrollBar.ValueChanged.Add(fun args ->
            let offset = int (Math.Round(args.NewValue))

            if offset <> editor.VerticalOffset then
                editor.ScrollVerticalTo(offset))

        horizontalScrollBar.ValueChanged.Add(fun args ->
            let offset = int (Math.Round(args.NewValue))

            if offset <> editor.HorizontalOffset then
                editor.ScrollHorizontalTo(offset))

        welcomeView.Value.NewFileRequested.Add(fun _ -> editor.NewDocument())
        welcomeView.Value.OpenFileRequested.Add(fun _ -> editor.OpenFile())
        welcomeView.Value.OpenFolderRequested.Add(fun _ -> editor.DispatchApplicationCommand(AppCommand.openFolder))

        updateScrollBar ()
        updateEditorStatus editor.EditorStatus
        updateTabs editor.SessionState
        updateEmptyState editor.SessionState
        updateToolRailMetadata ()

        this.FindControl<Button>("WorkspaceToolButton").Click.Add(fun _ -> this.Dispatch(ToggleTool WorkspaceTool))

        this.FindControl<Button>("SearchToolButton").Click.Add(fun _ -> this.Dispatch(ToggleTool SearchTool))

        tabsPanel.Value.DocumentActivated.Add(fun documentId -> editor.ActivateDocument(documentId))

        tabsPanel.Value.DocumentCloseRequested.Add(fun documentId ->
            editor.ActivateDocument(documentId)
            editor.CloseDocument())

        sidePanelHost.Value.CloseRequested.Add(fun _ -> this.Dispatch(CloseToolPanel))

        sidePanelHost.Value.ResizeRequested.Add(fun width -> this.Dispatch(SetSidePanelWidth width))

        auxiliaryPanelHost.Value.CloseRequested.Add(fun _ ->
            match editor.SessionState.Workspace.ActiveDocumentId with
            | Some documentId -> editor.DispatchApplicationCommand(AppCommand.setAgentOpen documentId false)
            | None -> ())

        this.AttachedToVisualTree.Add(fun _ -> attachSubscriptions ())
        this.DetachedFromVisualTree.Add(fun _ -> disposeSubscriptions ())

    member _.Editor = editor.Value

    member _.SessionState = editor.Value.SessionState

    member _.Layout: WorkspaceLayout =
        { SidePanelWidth = model.Layout.SidePanelWidth
          ActiveToolId =
            model.Layout.ActiveTool
            |> Option.map (ToolDescriptor.get >> fun descriptor -> descriptor.Id)
          IsToolPanelOpen = model.Layout.IsToolPanelOpen }

    member _.ApplyLayout(layout: WorkspaceLayout) =
        let widthMessage = SetSidePanelWidth layout.SidePanelWidth

        let toolMessage =
            layout.ActiveToolId
            |> Option.bind (fun toolId -> ToolDescriptor.all |> List.tryFind (fun descriptor -> descriptor.Id = toolId))
            |> Option.map (fun descriptor -> descriptor.Kind)
            |> fun tool -> RestoreTool(tool, layout.IsToolPanelOpen)

        model <-
            [ widthMessage; toolMessage ]
            |> List.fold (fun current message -> ShellUpdate.update message current |> fst) model

        refreshOnUiThread ()

    member _.ApplySettings(settings: AppSettings) =
        editor.Value.ThemeSettings <- settings.Theme

        let palette = settings.Theme.ThemeSource.Resolve()
        let foreground = SolidColorBrush(colorFromArgb palette.Foreground)
        let background = SolidColorBrush(colorFromArgb palette.Background)
        let borderColor = palette.GutterSeparator |> Option.defaultValue palette.Foreground

        this.Background <- background
        this.Foreground <- foreground
        statusBar.Value.Background <- SolidColorBrush(colorFromArgb palette.GutterBackground)
        statusBar.Value.BorderBrush <- SolidColorBrush(colorFromArgb borderColor)
        tabBar.Value.Background <- SolidColorBrush(colorFromArgb palette.GutterBackground)
        updateTabs editor.Value.SessionState

    interface IShellProjectionTarget with
        member _.ApplyShellInput(input) =
            updateTabsFromProjection input.Tabs
            updateEmptyStateFromProjection input.HasActiveDocument
            updateEditorStatus input.Status
            updateScrollBarFromProjection input.Scroll

    member _.Model = model

    member _.Dispatch(message: ShellMsg) =
        let updatedModel, effects = ShellUpdate.update message model
        model <- updatedModel

        effects
        |> List.iter (fun effect ->
            match effect with
            | DispatchAppCommand command -> editor.Value.DispatchApplicationCommand(command))

        refreshOnUiThread ()

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
