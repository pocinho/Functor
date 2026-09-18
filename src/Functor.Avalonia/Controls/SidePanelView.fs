namespace Functor.Avalonia.Controls

open Avalonia
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Layout
open Avalonia.Media

type SidePanelView() as this =
    inherit UserControl()

    let closeRequested = Event<unit>()
    let resizeRequested = Event<double>()
    let title = TextBlock(FontWeight = FontWeight.Bold)
    let content = ContentControl()
    let closeButton = Button(Content = "Close")

    let resizeHandle =
        Border(Width = 4.0, HorizontalAlignment = HorizontalAlignment.Right)

    let host = Border(BorderThickness = Thickness(1, 0, 0, 0), Padding = Thickness(12))
    let mutable resizeStartX = 0.0
    let mutable resizeStartWidth = 0.0
    let mutable isResizing = false

    do
        let header = Grid(ColumnDefinitions = ColumnDefinitions.Parse("*,Auto"))
        Grid.SetColumn(title, 0)
        Grid.SetColumn(closeButton, 1)
        header.Children.Add(title) |> ignore
        header.Children.Add(closeButton) |> ignore

        let panel = StackPanel(Spacing = 12.0)
        panel.Children.Add(header) |> ignore
        panel.Children.Add(content) |> ignore
        let surface = Grid(ColumnDefinitions = ColumnDefinitions.Parse("*,4"))
        Grid.SetColumn(panel, 0)
        Grid.SetColumn(resizeHandle, 1)
        surface.Children.Add(panel) |> ignore
        surface.Children.Add(resizeHandle) |> ignore
        host.Child <- surface
        this.Content <- host

        closeButton.Click.Add(fun _ -> closeRequested.Trigger())

        resizeHandle.PointerPressed.Add(fun args ->
            if this.IsResizable then
                resizeStartX <- args.GetPosition(this).X
                resizeStartWidth <- this.Width
                isResizing <- true
                args.Pointer.Capture(resizeHandle) |> ignore
                args.Handled <- true)

        resizeHandle.PointerMoved.Add(fun args ->
            if isResizing then
                let currentX = args.GetPosition(this).X
                resizeRequested.Trigger(resizeStartWidth + currentX - resizeStartX)
                args.Handled <- true)

        resizeHandle.PointerReleased.Add(fun args ->
            if isResizing then
                isResizing <- false
                args.Pointer.Capture(null) |> ignore
                args.Handled <- true)

    member _.CloseRequested = closeRequested.Publish

    member _.ResizeRequested = resizeRequested.Publish

    member _.IsResizable
        with get () = resizeHandle.IsVisible
        and set value = resizeHandle.IsVisible <- value

    member _.Title
        with get () = title.Text
        and set value = title.Text <- value

    member _.PanelContent
        with get () = content.Content
        and set value = content.Content <- value

    member _.IsOpen
        with get () = this.IsVisible
        and set value = this.IsVisible <- value

    member _.PanelWidth
        with get () = this.Width
        and set value = this.Width <- value
