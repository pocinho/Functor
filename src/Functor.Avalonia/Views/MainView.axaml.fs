namespace Functor.Avalonia.Views

open System
open Avalonia
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Layout
open Avalonia.Markup.Xaml
open Functor.Application
open Functor.Avalonia.Controls

type MainView() as this =
    inherit UserControl()

    let editor = lazy (this.FindControl<EditorControl>("EditorControl"))
    let verticalScrollBar = lazy (this.FindControl<ScrollBar>("VerticalScrollBar"))
    let horizontalScrollBar = lazy (this.FindControl<ScrollBar>("HorizontalScrollBar"))
    let positionText = lazy (this.FindControl<TextBlock>("PositionText"))
    let fileTypeText = lazy (this.FindControl<TextBlock>("FileTypeText"))
    let messageText = lazy (this.FindControl<TextBlock>("MessageText"))
    let fileNameText = lazy (this.FindControl<TextBlock>("FileNameText"))
    let dirtyText = lazy (this.FindControl<TextBlock>("DirtyText"))
    let mutable confirmationOpen = false

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

        editor.ScrollStateChanged.Add(fun () -> updateScrollBar ())

        verticalScrollBar.ValueChanged.Add(fun args ->
            let offset = int (Math.Round(args.NewValue))

            if offset <> editor.VerticalOffset then
                editor.ScrollVerticalTo(offset))

        horizontalScrollBar.ValueChanged.Add(fun args ->
            let offset = int (Math.Round(args.NewValue))

            if offset <> editor.HorizontalOffset then
                editor.ScrollHorizontalTo(offset))

        updateScrollBar ()
        updateEditorStatus editor.EditorStatus

    member _.Editor = editor.Value

    member _.SessionState = editor.Value.SessionState

    member _.ApplySettings(settings: AppSettings) =
        editor.Value.ThemeSettings <- settings.Theme

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
