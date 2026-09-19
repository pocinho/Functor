namespace Functor.Avalonia.Controls

open System
open Avalonia
open Avalonia.Controls
open Avalonia.Input
open Avalonia.Markup.Xaml

type SidePanelControl() as this =
    inherit UserControl()

    let resizeRequested = Event<double>()
    let mutable resizeStartX = 0.0
    let mutable resizeStartWidth = 0.0
    let mutable isResizing = false
    let mutable panelWidth = 0.0
    let mutable title = ""

    let content = lazy (this.FindControl<ContentControl>("PanelContentControl"))
    let resizeHandle = lazy (this.FindControl<Border>("ResizeHandle"))

    do
        this.InitializeComponent()

        resizeHandle.Value.PointerPressed.Add(fun args ->
            if this.IsResizable then
                let parent = this.Parent :?> Control
                resizeStartX <- args.GetPosition(parent).X
                resizeStartWidth <- if panelWidth > 0.0 then panelWidth else this.Bounds.Width
                isResizing <- true
                args.Pointer.Capture(resizeHandle.Value) |> ignore
                args.Handled <- true)

        resizeHandle.Value.PointerMoved.Add(fun args ->
            if isResizing then
                let parent = this.Parent :?> Control
                let currentX = args.GetPosition(parent).X
                let width = max 160.0 (min 720.0 (resizeStartWidth + currentX - resizeStartX))
                panelWidth <- width
                this.Width <- width
                resizeRequested.Trigger(width)
                args.Handled <- true)

        resizeHandle.Value.PointerReleased.Add(fun args ->
            if isResizing then
                isResizing <- false
                args.Pointer.Capture(null) |> ignore
                args.Handled <- true)

    member _.ResizeRequested = resizeRequested.Publish

    member _.Title
        with get () = title
        and set value = title <- value

    member _.IsResizable
        with get () = resizeHandle.Value.IsVisible
        and set value = resizeHandle.Value.IsVisible <- value

    member _.PanelContent
        with get () = content.Value.Content
        and set value = content.Value.Content <- value

    member _.IsOpen
        with get () = this.IsVisible
        and set value = this.IsVisible <- value

    member _.PanelWidth
        with get () = panelWidth
        and set value =
            panelWidth <- value
            this.Width <- value

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
