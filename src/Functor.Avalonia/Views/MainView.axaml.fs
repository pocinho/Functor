namespace Functor.Avalonia.Views

open System
open Avalonia
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Layout
open Avalonia.Media
open Avalonia.Markup.Xaml
open Functor.Application
open Functor.Avalonia.Controls
open Functor.Domain.Document
open Functor.Workspace

type MainView() as this =
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
    let tabsPanel = lazy (this.FindControl<StackPanel>("TabsPanel"))
    let mutable confirmationOpen = false

    let colorFromArgb (argb: uint32) =
        Color.FromArgb(byte (argb >>> 24), byte (argb >>> 16), byte (argb >>> 8), byte argb)

    let updateEmptyState (state: AppSessionState) =
        let hasActiveDocument = state.Model.ActiveDocument.IsSome
        editor.Value.IsVisible <- hasActiveDocument
        welcomeView.Value.IsVisible <- not hasActiveDocument

    let updateScrollBar () =
        let editor = editor.Value
        let verticalScrollBar = verticalScrollBar.Value
        let horizontalScrollBar = horizontalScrollBar.Value
        let maximum = float editor.VerticalScrollMaximum

        verticalScrollBar.Maximum <- maximum
        verticalScrollBar.ViewportSize <- float editor.VerticalScrollViewport
        verticalScrollBar.LargeChange <- max 1.0 (float editor.VerticalScrollViewport)
        verticalScrollBar.IsEnabled <- maximum > 0.0
        verticalScrollBar.Value <- float editor.VerticalOffset
        horizontalScrollBar.Maximum <- float editor.HorizontalScrollMaximum
        horizontalScrollBar.ViewportSize <- editor.HorizontalScrollViewport
        horizontalScrollBar.LargeChange <- max 1.0 editor.HorizontalScrollViewport
        horizontalScrollBar.IsEnabled <- editor.HorizontalScrollMaximum > 0
        horizontalScrollBar.Value <- float editor.HorizontalOffset

    let updateEditorStatus (status: Functor.Application.EditorStatus) =
        positionText.Value.Text <- sprintf "Ln %d, Col %d" status.Line status.Column
        fileTypeText.Value.Text <- status.FileType
        messageText.Value.Text <- status.Error |> Option.orElse status.Message |> Option.defaultValue ""
        fileNameText.Value.Text <- status.FileName
        dirtyText.Value.Text <- if status.IsDirty then "Modified" else ""

    let updateTabs (state: AppSessionState) =
        let panel = tabsPanel.Value
        panel.Children.Clear()

        let palette = editor.Value.ThemeSettings.ThemeSource.Resolve()
        let foreground = SolidColorBrush(colorFromArgb palette.Foreground)
        let border = SolidColorBrush(colorFromArgb (palette.GutterSeparator |> Option.defaultValue palette.Foreground))
        let selected = SolidColorBrush(colorFromArgb palette.Selection)
        let background = SolidColorBrush(colorFromArgb palette.GutterBackground)

        WorkspaceProjection.tabs state.Workspace
        |> List.iter (fun tab ->
            let tabButton =
                Button(
                    Padding = Thickness(10, 4),
                    MinHeight = 28.0,
                    BorderThickness = Thickness(1),
                    BorderBrush = border,
                    Background = (if tab.IsActive then selected else background),
                    Foreground = foreground,
                    HorizontalContentAlignment = HorizontalAlignment.Stretch
                )

            let label = TextBlock(Text = (if tab.IsDirty then tab.Name + " *" else tab.Name))
            let closeButton = Button(Content = "x", Width = 22.0, Height = 22.0, Padding = Thickness(0))
            let content = StackPanel(Orientation = Orientation.Horizontal, Spacing = 8.0)
            content.Children.Add(label) |> ignore
            content.Children.Add(closeButton) |> ignore
            tabButton.Content <- content

            tabButton.Click.Add(fun _ -> editor.Value.ActivateDocument(tab.DocumentId))
            closeButton.Click.Add(fun args ->
                args.Handled <- true
                editor.Value.ActivateDocument(tab.DocumentId)
                editor.Value.CloseDocument())
            panel.Children.Add(tabButton) |> ignore)

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
        | _ ->
            editor.Value.CancelPendingOperation()

    do
        this.InitializeComponent()

        let editor = editor.Value
        let verticalScrollBar = verticalScrollBar.Value
        let horizontalScrollBar = horizontalScrollBar.Value

        editor.StatusChanged.Add(fun status ->
            if status.PendingAction.IsSome && not confirmationOpen then
                showDiscardDialog ())
        editor.EditorStatusChanged.Add(updateEditorStatus)
        editor.StateChanged.Add(updateTabs)
        editor.StateChanged.Add(updateEmptyState)

        editor.ScrollStateChanged.Add(fun () -> updateScrollBar ())

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

    member _.Editor = editor.Value

    member _.SessionState = editor.Value.SessionState

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

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
