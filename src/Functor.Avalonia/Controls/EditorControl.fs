namespace Functor.Avalonia.Controls

open System
open System.Threading.Tasks
open Avalonia
open Avalonia.Controls
open Avalonia.Input
open Avalonia.Media
open Avalonia.Skia

open Functor.Domain.Editing
open Functor.Domain.Document
open Functor.Application
open Functor.Domain.Core
open Functor.Avalonia.Rendering
open Functor.Avalonia.Services
open Functor.Platform
open Functor.Rendering

type EditorControl() as this =
    inherit Control()

    // ------------------------------------------------------------
    // Internal state
    // ------------------------------------------------------------

    let session = EditorSession()
    let clipboardService: IClipboardService =
        AvaloniaClipboardService(fun () -> TopLevel.GetTopLevel(this) |> Option.ofObj)
    let fileService: IFileService = FileService()
    let dialogService: IDialogService =
        AvaloniaDialogService(fun () -> TopLevel.GetTopLevel(this) |> Option.ofObj)
    let tokenizerService: ITokenizerService = DefaultTokenizerService()
    let effectInterpreter =
        AppEffectInterpreter(
            clipboardService,
            fileService,
            dialogService,
            session.DispatchCommand,
            tokenizerService = tokenizerService
        )

    do
        session.EffectsRequested.Add(fun effects ->
            effects
            |> List.iter (fun effect -> Async.StartImmediate(effectInterpreter.Execute effect)))

    let mutable isPointerSelecting = false
    let scrollStateChanged = Event<unit>()

    do
        session.StateChanged.Add(fun _ ->
            this.InvalidateVisual()
            scrollStateChanged.Trigger())

    let lineHeight = 16.0f

    let renderingConfig =
        { RenderingPipeline.Measurer =
            TextMeasurer.create
                (TextMetrics.createWithGraphemeAdvance
                    lineHeight
                    (RenderingSurface.measureDefaultAdvance lineHeight)
                    4
                    (RenderingSurface.measureGraphemeAdvance lineHeight)) }

    let renderBackend: IRenderBackend<DrawingContext, Avalonia.Rect, ThemePalette> = AvaloniaRenderBackend()
    let mutable themeSettings = ThemeSettings.defaultTheme

    member this.ThemeSettings
        with get () = themeSettings
        and set (value: ThemeSettings) =
            themeSettings <- value
            this.InvalidateVisual()

    member this.ThemeSource
        with get () = themeSettings.ThemeSource
        and set (value: ThemeSource) = themeSettings <- { themeSettings with ThemeSource = value }

    member private this.BorderInset =
        let theme = themeSettings.ThemeSource.Resolve()

        match theme.EditorBorder with
        | Some _ -> max 0.0f theme.EditorBorderWidth
        | None -> 0.0f

    member private this.ContentWidth =
        max 1.0f (float32 this.Bounds.Width - (this.BorderInset * 2.0f))

    member private this.ContentHeight =
        max 1.0f (float32 this.Bounds.Height - (this.BorderInset * 2.0f))

    member private this.VisibleLineCount =
        max 1 (int (Math.Ceiling(float this.ContentHeight / float lineHeight)))

    member private this.PositionAtPoint(point: Point) =
        LayoutEngine.positionAtPointWithGutter
            renderingConfig.Measurer
            (LayoutEngine.gutterWidth renderingConfig.Measurer session.Model.Editing.Buffer.Length)
            session.Model.View.HorizontalOffset
            session.Model.View.VerticalOffset
            session.Model.Editing.Buffer
            (float32 point.X - this.BorderInset)
            (float32 point.Y - this.BorderInset)

    member private this.NotifyScrollStateChanged() = scrollStateChanged.Trigger()

    member private this.ClipboardService : IClipboardService =
        clipboardService

    member private this.ApplyEditingEvent(event: EditingEvent) =
        session.Dispatch(CoreEvent.ApplyEditingEvent event)
        this.InvalidateVisual()
        this.NotifyScrollStateChanged()

    member private this.CopySelection() =
        match EditingLogic.selectedText session.Model.Editing with
        | Some text ->
            Async.StartImmediate(async { do! this.ClipboardService.SetText text })
        | _ -> ()

    member private this.CutSelection() =
        match EditingLogic.selectedText session.Model.Editing with
        | Some text ->
            Async.StartImmediate(
                async {
                    do! this.ClipboardService.SetText text
                    this.ApplyEditingEvent(EditingEvent.DeleteSelection)
                }
            )
        | _ -> ()

    member private this.Paste() =
        Async.StartImmediate(
            async {
                let! text = this.ClipboardService.GetText()

                match text with
                | Some value -> this.ApplyEditingEvent(EditingEvent.InsertString value)
                | None -> ()
            }
        )

    member this.ScrollStateChanged = scrollStateChanged.Publish

    member this.StatusChanged = session.StatusChanged

    member this.StateChanged = session.StateChanged

    member this.SessionState = session.State

    member this.EditorStatus = session.EditorStatus

    member this.EditorStatusChanged = session.EditorStatusChanged

    member this.NewDocument() =
        session.DispatchCommand(AppCommand.newDocument)

    member this.OpenFile() =
        session.DispatchCommand(AppCommand.openFile)

    member this.SaveFile() =
        session.DispatchCommand(AppCommand.saveFile)

    member this.SaveFileAs() =
        session.DispatchCommand(AppCommand.saveFileAs)

    member this.CloseDocument() =
        session.DispatchCommand(AppCommand.closeDocument)

    member this.ReopenClosedTab() =
        session.DispatchCommand(AppCommand.reopenClosedTab)

    member this.ActivateDocument(documentId: DocumentId) =
        session.Dispatch(CoreEvent.SwitchDocument documentId)

    member this.DispatchApplicationCommand(command: AppCommand) =
        session.DispatchCommand(command)

    member this.ConfirmDiscardChanges() =
        session.DispatchCommand(AppCommand.confirmDiscardChanges)

    member this.CancelPendingOperation() =
        session.DispatchCommand(AppCommand.cancelPendingOperation)

    member this.VerticalOffset = session.Model.View.VerticalOffset

    member this.HorizontalOffset = session.Model.View.HorizontalOffset

    member this.VerticalScrollMaximum =
        LayoutEngine.maxVerticalOffset this.VisibleLineCount session.Model.Editing.Buffer

    member this.HorizontalScrollMaximum =
        LayoutEngine.maxHorizontalOffset
            renderingConfig.Measurer
            this.ContentWidth
            (LayoutEngine.gutterWidth renderingConfig.Measurer session.Model.Editing.Buffer.Length)
            session.Model.Editing.Buffer

    member this.VerticalScrollViewport = this.VisibleLineCount

    member this.HorizontalScrollViewport =
        let availableWidth =
            max 1.0f (
                this.ContentWidth
                - LayoutEngine.gutterWidth renderingConfig.Measurer session.Model.Editing.Buffer.Length
            )

        max 1.0 (Math.Floor(float availableWidth / float renderingConfig.Measurer.Metrics.DefaultAdvance))

    member this.DocumentPositionAtPoint(point: Point) = this.PositionAtPoint(point)

    member this.ScrollVerticalTo(offset: int) =
        let clamped = max 0 (min this.VerticalScrollMaximum offset)
        session.Dispatch(CoreEvent.ScrollVerticalTo clamped)
        this.InvalidateVisual()
        this.NotifyScrollStateChanged()

    member this.ScrollHorizontalTo(offset: int) =
        let clamped = max 0 (min this.HorizontalScrollMaximum offset)
        session.Dispatch(CoreEvent.ScrollHorizontalTo clamped)
        this.InvalidateVisual()
        this.NotifyScrollStateChanged()

    // ------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------

    member private this.UpdateViewport() =
        session.Dispatch(CoreEvent.ResizeViewport(int this.ContentWidth, int this.ContentHeight))

    // ------------------------------------------------------------
    // Avalonia overrides
    // ------------------------------------------------------------

    override this.OnPropertyChanged(e) =
        base.OnPropertyChanged(e)

        if e.Property = Control.BoundsProperty then
            this.UpdateViewport()
            this.InvalidateVisual()
            this.NotifyScrollStateChanged()
        elif e.Property = Visual.IsVisibleProperty && this.IsVisible then
            // A hidden control is never laid out, so Bounds can still be stale/zero here.
            this.UpdateViewport()
            this.InvalidateVisual()
            this.NotifyScrollStateChanged()

    override this.OnAttachedToVisualTree(e: VisualTreeAttachmentEventArgs) =
        base.OnAttachedToVisualTree(e)

        if session.Model.ActiveDocument.IsNone then
            this.NewDocument()

        this.Focus() |> ignore

    override this.OnTextInput(e: TextInputEventArgs) =
        base.OnTextInput(e)

        if not (String.IsNullOrEmpty(e.Text)) then
            this.ApplyEditingEvent(EditingEvent.InsertString e.Text)
            e.Handled <- true

    override this.OnKeyDown(e: KeyEventArgs) =
        base.OnKeyDown(e)

        let commandModifier =
            e.KeyModifiers.HasFlag(KeyModifiers.Control)
            || e.KeyModifiers.HasFlag(KeyModifiers.Meta)

        if commandModifier then
            match e.Key with
            | Key.C ->
                this.CopySelection()
                e.Handled <- true
            | Key.X ->
                this.CutSelection()
                e.Handled <- true
            | Key.V ->
                this.Paste()
                e.Handled <- true
            | Key.O ->
                this.OpenFile()
                e.Handled <- true
            | Key.S when e.KeyModifiers.HasFlag(KeyModifiers.Shift) ->
                this.SaveFileAs()
                e.Handled <- true
            | Key.S ->
                this.SaveFile()
                e.Handled <- true
            | Key.N ->
                this.NewDocument()
                e.Handled <- true
            | Key.W when not (e.KeyModifiers.HasFlag(KeyModifiers.Shift)) ->
                this.CloseDocument()
                e.Handled <- true
            | Key.T when e.KeyModifiers.HasFlag(KeyModifiers.Shift) ->
                this.ReopenClosedTab()
                e.Handled <- true
            | _ -> ()

        let editingEvent =
            match e.Key with
            | Key.Left -> Some EditingEvent.MoveLeft
            | Key.Right -> Some EditingEvent.MoveRight
            | Key.Up -> Some EditingEvent.MoveUp
            | Key.Down -> Some EditingEvent.MoveDown
            | Key.Back -> Some EditingEvent.Backspace
            | Key.Delete -> Some EditingEvent.Delete
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
            if e.KeyModifiers.HasFlag(KeyModifiers.Shift) then
                this.ScrollHorizontalTo(this.HorizontalOffset + delta)
            else
                this.ScrollVerticalTo(this.VerticalOffset + delta)
            e.Handled <- true

    override this.Render(context: DrawingContext) =
        base.Render(context)

        let frame: RenderingModel = RenderingPipeline.render renderingConfig session.Model
        let bounds = Avalonia.Rect(0.0, 0.0, this.Bounds.Width, this.Bounds.Height)
        let theme = themeSettings.ThemeSource.Resolve()

        RenderBackend.drawFrame renderBackend context bounds frame theme
