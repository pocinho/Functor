namespace Functor.App.Controls

open System
open Avalonia
open Avalonia.Controls
open Avalonia.Input
open Avalonia.Media
open Avalonia.Skia

open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.App.Rendering
open Functor.Rendering

type EditorControl() =
    inherit Control()

    // ------------------------------------------------------------
    // Internal state
    // ------------------------------------------------------------

    let mutable coreModel: CoreModel = CoreModel.empty
    let mutable isPointerSelecting = false
    let scrollStateChanged = Event<unit>()

    let lineHeight = 16.0f

    let renderingConfig =
        { RenderingPipeline.Metrics =
            { LineHeight = lineHeight
              DefaultAdvance = RenderingSurface.measureDefaultAdvance lineHeight
              TabWidth = 4 } }

    member private this.VisibleLineCount =
        max 1 (int (Math.Ceiling(this.Bounds.Height / float lineHeight)))

    member private this.PositionAtPoint(point: Point) =
        LayoutEngine.positionAtPoint
            renderingConfig.Metrics
            coreModel.HorizontalOffset
            coreModel.VerticalOffset
            coreModel.Editing.Buffer
            (float32 point.X)
            (float32 point.Y)

    member private this.NotifyScrollStateChanged() = scrollStateChanged.Trigger()

    member private this.ApplyEditingEvent(event: EditingEvent) =
        coreModel <- CoreLogic.update (CoreEvent.ApplyEditingEvent event) coreModel
        this.InvalidateVisual()
        this.NotifyScrollStateChanged()

    member this.ScrollStateChanged = scrollStateChanged.Publish

    member this.VerticalOffset = coreModel.VerticalOffset

    member this.VerticalScrollMaximum =
        LayoutEngine.maxVerticalOffset this.VisibleLineCount coreModel.Editing.Buffer

    member this.VerticalScrollViewport = this.VisibleLineCount

    member this.ScrollTo(offset: int) =
        let clamped = max 0 (min this.VerticalScrollMaximum offset)
        coreModel <- CoreLogic.update (CoreEvent.ScrollTo clamped) coreModel
        this.InvalidateVisual()
        this.NotifyScrollStateChanged()

    // ------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------

    member private this.UpdateViewport() =
        let b = this.Bounds

        coreModel <-
            { coreModel with
                Viewport =
                    { Width = int b.Width
                      Height = int b.Height } }

    // ------------------------------------------------------------
    // Avalonia overrides
    // ------------------------------------------------------------

    override this.OnPropertyChanged(e) =
        base.OnPropertyChanged(e)

        if e.Property = Control.BoundsProperty then
            this.UpdateViewport()
            this.InvalidateVisual()
            this.NotifyScrollStateChanged()

    override this.OnAttachedToVisualTree(e: VisualTreeAttachmentEventArgs) =
        base.OnAttachedToVisualTree(e)
        this.Focus() |> ignore

    override this.OnTextInput(e: TextInputEventArgs) =
        base.OnTextInput(e)

        if not (String.IsNullOrEmpty(e.Text)) then
            this.ApplyEditingEvent(EditingEvent.InsertString e.Text)
            e.Handled <- true

    override this.OnKeyDown(e: KeyEventArgs) =
        base.OnKeyDown(e)

        let editingEvent =
            match e.Key with
            | Key.Left -> Some EditingEvent.MoveLeft
            | Key.Right -> Some EditingEvent.MoveRight
            | Key.Up -> Some EditingEvent.MoveUp
            | Key.Down -> Some EditingEvent.MoveDown
            | Key.Back -> Some EditingEvent.Backspace
            | Key.Enter -> Some EditingEvent.InsertNewLine
            | _ -> None

        match editingEvent with
        | Some event ->
            this.ApplyEditingEvent(event)
            e.Handled <- true
        | None -> ()

    override this.OnPointerPressed(e: PointerPressedEventArgs) =
        base.OnPointerPressed(e)

        let point = e.GetCurrentPoint(this)

        if point.Properties.IsLeftButtonPressed then
            this.Focus() |> ignore
            isPointerSelecting <- true
            e.Pointer.Capture(this) |> ignore

            let position = this.PositionAtPoint(point.Position)
            this.ApplyEditingEvent(EditingEvent.SetCursor position)
            this.ApplyEditingEvent(EditingEvent.StartSelection)
            e.Handled <- true

    override this.OnPointerMoved(e: PointerEventArgs) =
        base.OnPointerMoved(e)

        if isPointerSelecting && e.GetCurrentPoint(this).Properties.IsLeftButtonPressed then
            let position = this.PositionAtPoint(e.GetPosition(this))
            this.ApplyEditingEvent(EditingEvent.SetCursor position)
            this.ApplyEditingEvent(EditingEvent.UpdateSelection)
            e.Handled <- true

    override this.OnPointerReleased(e: PointerReleasedEventArgs) =
        base.OnPointerReleased(e)

        if isPointerSelecting then
            let position = this.PositionAtPoint(e.GetPosition(this))
            this.ApplyEditingEvent(EditingEvent.SetCursor position)
            this.ApplyEditingEvent(EditingEvent.UpdateSelection)
            isPointerSelecting <- false
            e.Pointer.Capture(null) |> ignore
            e.Handled <- true

    override this.OnPointerWheelChanged(e: PointerWheelEventArgs) =
        base.OnPointerWheelChanged(e)

        if e.Delta.Y <> 0.0 then
            let delta = -int(Math.Round(e.Delta.Y))
            this.ScrollTo(this.VerticalOffset + delta)
            e.Handled <- true

    override this.Render(context: DrawingContext) =
        base.Render(context)

        let frame: RenderingModel = RenderingPipeline.render renderingConfig coreModel

        let bounds = Rect(0.0, 0.0, this.Bounds.Width, this.Bounds.Height)

        RenderingSurface.draw context bounds frame
