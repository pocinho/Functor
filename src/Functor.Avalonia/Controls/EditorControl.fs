namespace Functor.Avalonia.Controls

open System
open System.Threading
open System.Threading.Tasks
open Avalonia
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Input
open Avalonia.Layout
open Avalonia.Media
open Avalonia.Skia

open Functor.Domain.Editing
open Functor.Domain.Document
open Functor.Application
open Functor.Syntax
open Functor.Domain.Core
open Functor.Avalonia
open Functor.Avalonia.Rendering
open Functor.Avalonia.Services
open Functor.Rendering

type private EditorRenderSurface() =
    inherit Control()

    let mutable renderCallback: DrawingContext -> unit = ignore

    member this.SetRenderCallback(callback: DrawingContext -> unit) = renderCallback <- callback

    override this.Render(context: DrawingContext) =
        base.Render(context)
        renderCallback context

type private EditorMiniMap() =
    inherit Control()

    let mutable linesProvider: unit -> string list = fun () -> []
    let mutable viewportProvider: unit -> int * int = fun () -> 0, 1
    let mutable scrollCallback: float -> unit = ignore
    let mutable palette = Theme.defaultPalette
    let mutable isDragging = false

    let colorFromArgb (argb: uint32) =
        let a = byte ((argb >>> 24) &&& 0xFFu)
        let r = byte ((argb >>> 16) &&& 0xFFu)
        let g = byte ((argb >>> 8) &&& 0xFFu)
        let b = byte (argb &&& 0xFFu)
        Color.FromArgb(a, r, g, b)

    member this.SetProviders(lines: unit -> string list, viewport: unit -> int * int, scroll: float -> unit) =
        linesProvider <- lines
        viewportProvider <- viewport
        scrollCallback <- scroll

    member this.SetPalette(value: ThemePalette) = palette <- value

    member private this.ScrollToPoint(point: Point) =
        let lineCount = max 1 (linesProvider ()).Length
        let visibleLines = max 1 (snd (viewportProvider ()))
        let lineHeight = min 3.0 (this.Bounds.Height / float lineCount)
        let contentHeight = max 1.0 (lineHeight * float lineCount)
        let ratio = max 0.0 (min 1.0 (point.Y / contentHeight))
        let targetLine = ratio * float lineCount - (float visibleLines / 2.0)
        scrollCallback (max 0.0 (min (float (max 0 (lineCount - 1))) targetLine))

    override this.OnPointerPressed(e: PointerPressedEventArgs) =
        base.OnPointerPressed(e)
        isDragging <- true
        e.Pointer.Capture(this) |> ignore
        this.ScrollToPoint(e.GetCurrentPoint(this).Position)
        e.Handled <- true

    override this.OnPointerMoved(e: PointerEventArgs) =
        base.OnPointerMoved(e)

        if isDragging then
            this.ScrollToPoint(e.GetCurrentPoint(this).Position)
            e.Handled <- true

    override this.OnPointerReleased(e: PointerReleasedEventArgs) =
        base.OnPointerReleased(e)

        if isDragging then
            isDragging <- false
            e.Pointer.Capture(null) |> ignore
            e.Handled <- true

    override this.Render(context: DrawingContext) =
        base.Render(context)

        let bounds = Rect(0.0, 0.0, this.Bounds.Width, this.Bounds.Height)
        let lineList = linesProvider ()
        let lineCount = max 1 lineList.Length
        let lineHeight = min 3.0 (bounds.Height / float lineCount)
        let contentHeight = max 1.0 (lineHeight * float lineCount)
        let foreground = SolidColorBrush(colorFromArgb (palette.Foreground &&& 0x80FFFFFFu))
        let background = SolidColorBrush(colorFromArgb palette.GutterBackground)

        let viewportBrush =
            SolidColorBrush(colorFromArgb (palette.Selection &&& 0x30FFFFFFu))

        context.FillRectangle(background, bounds)

        lineList
        |> List.iteri (fun index line ->
            let widthRatio = min 1.0 (float (max 1 line.Length) / 120.0)
            let width = max 2.0 ((bounds.Width - 8.0) * widthRatio)
            context.FillRectangle(foreground, Rect(4.0, float index * lineHeight, width, max 1.0 (lineHeight * 0.55))))

        let offset, visibleLines = viewportProvider ()
        let viewportTop = float offset * lineHeight
        let viewportHeight = max 3.0 (float visibleLines * lineHeight)
        context.FillRectangle(viewportBrush, Rect(0.0, viewportTop, bounds.Width, min contentHeight viewportHeight))

type EditorControl() as this =
    inherit Panel()

    // ------------------------------------------------------------
    // Internal state
    // ------------------------------------------------------------

    let session = EditorSession()

    let mutable services: EditorServices option = None
    let mutable effectInterpreter: AppEffectInterpreter option = None

    let configureComposition (composition: AvaloniaComposition) =
        services <- Some composition.EditorServices
        effectInterpreter <- Some(AppEffectInterpreter(composition.EditorServices, session.DispatchCommand))

    do
        session.EffectsRequested.Add(fun effects ->
            match effectInterpreter with
            | Some interpreter -> effects |> List.iter (fun effect -> Async.StartImmediate(interpreter.Execute effect))
            | None -> ())

    let mutable isPointerSelecting = false
    let scrollStateChanged = Event<unit>()
    let workspaceStructureChanged = Event<unit>()

    let verticalScrollBar =
        ScrollBar(
            Orientation = Orientation.Vertical,
            Minimum = 0.0,
            Maximum = 0.0,
            ViewportSize = 1.0,
            IsEnabled = false,
            ZIndex = 1
        )

    let horizontalScrollBar =
        ScrollBar(
            Orientation = Orientation.Horizontal,
            Minimum = 0.0,
            Maximum = 0.0,
            ViewportSize = 1.0,
            IsEnabled = false,
            ZIndex = 1
        )

    let miniMap = EditorMiniMap(Width = 72.0, ZIndex = 1)
    let renderSurface = EditorRenderSurface()

    do
        miniMap.Classes.Add("editor-minimap")
        verticalScrollBar.Classes.Add("editor-scrollbar")
        horizontalScrollBar.Classes.Add("editor-scrollbar")
        this.Children.Add(renderSurface) |> ignore
        this.Children.Add(miniMap) |> ignore
        this.Children.Add(verticalScrollBar) |> ignore
        this.Children.Add(horizontalScrollBar) |> ignore

        miniMap.SetProviders(
            (fun () -> session.Model.Editing.Buffer),
            (fun () -> session.Model.View.VerticalOffset, this.VisibleLineCount),
            (fun offset -> this.ScrollVerticalTo(int (Math.Round(offset))))
        )

        renderSurface.SetRenderCallback(fun context -> this.RenderEditor(context))

    let invalidateEditorSurface () = renderSurface.InvalidateVisual()

    let mutable previousDocumentPaths =
        session.State.Workspace.Documents
        |> Map.map (fun _ documentState -> documentState.Document.Metadata.Path)

    do
        session.StateChanged.Add(fun state ->
            invalidateEditorSurface ()
            this.UpdateScrollBars()
            this.UpdateMiniMap()
            scrollStateChanged.Trigger()

            let pathChanged =
                state.Workspace.Documents
                |> Map.exists (fun documentId documentState ->
                    previousDocumentPaths
                    |> Map.tryFind documentId
                    |> Option.exists (fun previousPath -> previousPath <> documentState.Document.Metadata.Path))

            if pathChanged then
                workspaceStructureChanged.Trigger()

            previousDocumentPaths <-
                state.Workspace.Documents
                |> Map.map (fun _ documentState -> documentState.Document.Metadata.Path))

    let mutable themeSettings = ThemeSettings.defaultTheme

    let createRenderingConfig (uiTheme: UiThemeDefaults) =
        let lineHeight = float32 uiTheme.EditorLineHeight

        let measurer =
            TextMeasurer.create (
                TextMetrics.createWithGraphemeAdvance
                    lineHeight
                    (RenderingSurface.measureDefaultAdvance lineHeight)
                    uiTheme.EditorTabSize
                    (RenderingSurface.measureGraphemeAdvance lineHeight)
            )

        RenderingPipeline.createConfig measurer (float32 uiTheme.GutterPadding) (float32 uiTheme.GutterMinimumWidth)

    let mutable renderingConfig = createRenderingConfig themeSettings.Ui

    let renderBackend: IRenderBackend<DrawingContext, Avalonia.Rect, RenderingSurface.ThemeSnapshot> =
        AvaloniaRenderBackend()

    member this.ThemeSettings
        with get () = themeSettings
        and set (value: ThemeSettings) =
            themeSettings <- value
            RenderingSurface.setUiTheme value.Ui
            renderingConfig <- createRenderingConfig value.Ui
            miniMap.SetPalette(value.ThemeSource.Resolve())
            miniMap.InvalidateVisual()
            invalidateEditorSurface ()

    member this.ThemeSource
        with get () = themeSettings.ThemeSource
        and set (value: ThemeSource) =
            themeSettings <-
                { themeSettings with
                    ThemeSource = value }

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
        max 1 (int (Math.Ceiling(float this.ContentHeight / themeSettings.Ui.EditorLineHeight)))

    member private this.PositionAtPoint(point: Point) =
        LayoutEngine.positionAtPointWithGutter
            renderingConfig.Measurer
            (LayoutEngine.gutterWidthWithMetrics
                renderingConfig.Measurer
                session.Model.Editing.Buffer.Length
                renderingConfig.GutterPadding
                renderingConfig.GutterMinimumWidth)
            session.Model.View.HorizontalOffset
            session.Model.View.VerticalOffset
            session.Model.Editing.Buffer
            (float32 point.X - this.BorderInset)
            (float32 point.Y - this.BorderInset)

    member private this.NotifyScrollStateChanged() = scrollStateChanged.Trigger()

    member private this.ClipboardService: IClipboardService =
        services
        |> Option.defaultWith (fun () -> invalidOp "Editor composition has not been configured.")
        |> fun configured -> configured.Clipboard

    member private this.ApplyEditingEvent(event: EditingEvent) =
        session.Dispatch(CoreEvent.ApplyEditingEvent event)
        invalidateEditorSurface ()
        this.NotifyScrollStateChanged()

    member private this.CopySelection() =
        match EditingLogic.selectedText session.Model.Editing with
        | Some text -> Async.StartImmediate(async { do! this.ClipboardService.SetText(text, CancellationToken.None) })
        | _ -> ()

    member private this.CutSelection() =
        match EditingLogic.selectedText session.Model.Editing with
        | Some text ->
            Async.StartImmediate(
                async {
                    do! this.ClipboardService.SetText(text, CancellationToken.None)
                    this.ApplyEditingEvent(EditingEvent.TextInput TextInputEvent.DeleteSelection)
                }
            )
        | _ -> ()

    member private this.Paste() =
        Async.StartImmediate(
            async {
                let! text = this.ClipboardService.GetText(CancellationToken.None)

                match text with
                | Some value -> this.ApplyEditingEvent(EditingEvent.TextInput(TextInputEvent.InsertString value))
                | None -> ()
            }
        )

    member this.ScrollStateChanged = scrollStateChanged.Publish

    member this.StatusChanged = session.StatusChanged

    member this.StateChanged = session.StateChanged

    member this.WorkspaceStructureChanged = workspaceStructureChanged.Publish

    member this.SessionState = session.State

    member _.ConfigureComposition(composition: AvaloniaComposition) =
        configureComposition composition

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
        this.Focus() |> ignore

    member this.DispatchApplicationCommand(command: AppCommand) = session.DispatchCommand(command)

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
            (LayoutEngine.gutterWidthWithMetrics
                renderingConfig.Measurer
                session.Model.Editing.Buffer.Length
                renderingConfig.GutterPadding
                renderingConfig.GutterMinimumWidth)
            session.Model.Editing.Buffer

    member this.VerticalScrollViewport = this.VisibleLineCount

    member this.HorizontalScrollViewport =
        let availableWidth =
            max
                1.0f
                (this.ContentWidth
                 - LayoutEngine.gutterWidthWithMetrics
                     renderingConfig.Measurer
                     session.Model.Editing.Buffer.Length
                     renderingConfig.GutterPadding
                     renderingConfig.GutterMinimumWidth)

        max 1.0 (Math.Floor(float availableWidth / float renderingConfig.Measurer.Metrics.DefaultAdvance))

    member this.DocumentPositionAtPoint(point: Point) = this.PositionAtPoint(point)

    member this.ScrollVerticalTo(offset: int) =
        let clamped = max 0 (min this.VerticalScrollMaximum offset)
        session.Dispatch(CoreEvent.ScrollVerticalTo clamped)
        invalidateEditorSurface ()
        this.NotifyScrollStateChanged()

    member this.ScrollHorizontalTo(offset: int) =
        let clamped = max 0 (min this.HorizontalScrollMaximum offset)
        session.Dispatch(CoreEvent.ScrollHorizontalTo clamped)
        invalidateEditorSurface ()
        this.NotifyScrollStateChanged()

    member private this.UpdateScrollBars() =
        verticalScrollBar.Maximum <- float this.VerticalScrollMaximum
        verticalScrollBar.ViewportSize <- float this.VerticalScrollViewport
        verticalScrollBar.LargeChange <- float this.VerticalScrollViewport
        verticalScrollBar.IsEnabled <- this.VerticalScrollMaximum > 0
        verticalScrollBar.Value <- float this.VerticalOffset
        horizontalScrollBar.Maximum <- float this.HorizontalScrollMaximum
        horizontalScrollBar.ViewportSize <- this.HorizontalScrollViewport
        horizontalScrollBar.LargeChange <- this.HorizontalScrollViewport
        horizontalScrollBar.IsEnabled <- this.HorizontalScrollMaximum > 0
        horizontalScrollBar.Value <- float this.HorizontalOffset

    member private this.UpdateMiniMap() =
        miniMap.SetPalette(themeSettings.ThemeSource.Resolve())
        miniMap.IsVisible <- true
        miniMap.IsHitTestVisible <- session.Model.Editing.Buffer.Length > 1
        miniMap.InvalidateVisual()

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
            invalidateEditorSurface ()
            this.UpdateScrollBars()
            this.UpdateMiniMap()
            this.NotifyScrollStateChanged()
        elif e.Property = Visual.IsVisibleProperty && this.IsVisible then
            // A hidden control is never laid out, so Bounds can still be stale/zero here.
            this.UpdateViewport()
            invalidateEditorSurface ()
            this.UpdateScrollBars()
            this.UpdateMiniMap()
            this.NotifyScrollStateChanged()

    override this.MeasureOverride(availableSize) =
        miniMap.Measure(availableSize)
        verticalScrollBar.Measure(availableSize)
        horizontalScrollBar.Measure(availableSize)
        availableSize

    override this.ArrangeOverride(finalSize) =
        let verticalWidth = verticalScrollBar.DesiredSize.Width
        let horizontalHeight = horizontalScrollBar.DesiredSize.Height
        let miniMapWidth = miniMap.DesiredSize.Width

        renderSurface.Arrange(Rect(0.0, 0.0, finalSize.Width, finalSize.Height))

        miniMap.Arrange(Rect(finalSize.Width - verticalWidth - miniMapWidth, 0.0, miniMapWidth, finalSize.Height))

        verticalScrollBar.Arrange(
            Rect(finalSize.Width - verticalWidth, 0.0, verticalWidth, max 0.0 (finalSize.Height - horizontalHeight))
        )

        horizontalScrollBar.Arrange(
            Rect(
                0.0,
                finalSize.Height - horizontalHeight,
                max 0.0 (finalSize.Width - verticalWidth - miniMapWidth),
                horizontalHeight
            )
        )

        finalSize

    override this.OnAttachedToVisualTree(e: VisualTreeAttachmentEventArgs) =
        base.OnAttachedToVisualTree(e)

        if effectInterpreter.IsNone then
            configureComposition (EditorServicesFactory.create (fun () -> TopLevel.GetTopLevel(this) |> Option.ofObj))

        verticalScrollBar.ValueChanged.Add(fun args ->
            let offset = int (Math.Round(args.NewValue))

            if offset <> this.VerticalOffset then
                this.ScrollVerticalTo(offset))

        horizontalScrollBar.ValueChanged.Add(fun args ->
            let offset = int (Math.Round(args.NewValue))

            if offset <> this.HorizontalOffset then
                this.ScrollHorizontalTo(offset))

        if session.Model.ActiveDocument.IsNone then
            this.NewDocument()

        this.UpdateScrollBars()
        this.UpdateMiniMap()
        this.Focus() |> ignore

    override this.OnGotFocus(e: FocusChangedEventArgs) =
        base.OnGotFocus(e)
        invalidateEditorSurface ()

    override this.OnLostFocus(e: FocusChangedEventArgs) =
        base.OnLostFocus(e)
        invalidateEditorSurface ()

    override this.OnTextInput(e: TextInputEventArgs) =
        base.OnTextInput(e)

        match Functor.Avalonia.InputAdapter.textInput e.Text with
        | Some input ->
            this.ApplyEditingEvent(EditingEvent.TextInput(TextInputEvent.InsertString input.Text))
            e.Handled <- true
        | None -> ()

    override this.OnKeyDown(e: KeyEventArgs) =
        base.OnKeyDown(e)

        match Functor.Avalonia.InputAdapter.editorAction e.Key e.KeyModifiers with
        | Some Functor.Input.EditorAction.Copy ->
            this.CopySelection()
            e.Handled <- true
        | Some Functor.Input.EditorAction.Cut ->
            this.CutSelection()
            e.Handled <- true
        | Some Functor.Input.EditorAction.Paste ->
            this.Paste()
            e.Handled <- true
        | Some Functor.Input.EditorAction.OpenFile ->
            this.OpenFile()
            e.Handled <- true
        | Some Functor.Input.EditorAction.SaveFileAs ->
            this.SaveFileAs()
            e.Handled <- true
        | Some Functor.Input.EditorAction.SaveFile ->
            this.SaveFile()
            e.Handled <- true
        | Some Functor.Input.EditorAction.NewDocument ->
            this.NewDocument()
            e.Handled <- true
        | Some Functor.Input.EditorAction.CloseDocument ->
            this.CloseDocument()
            e.Handled <- true
        | Some Functor.Input.EditorAction.ReopenClosedTab ->
            this.ReopenClosedTab()
            e.Handled <- true
        | None -> ()

        match Functor.Avalonia.InputAdapter.editingEvent e.Key e.KeyModifiers with
        | Some event ->
            if e.KeyModifiers.HasFlag(KeyModifiers.Shift) then
                if session.Model.Editing.Selection.IsNone then
                    this.ApplyEditingEvent(EditingEvent.Selection SelectionEvent.StartSelection)

                this.ApplyEditingEvent(event)
                this.ApplyEditingEvent(EditingEvent.Selection SelectionEvent.UpdateSelection)
            else
                this.ApplyEditingEvent(event)

                match event with
                | EditingEvent.Cursor(CursorEvent.MoveLeft)
                | EditingEvent.Cursor(CursorEvent.MoveRight)
                | EditingEvent.Cursor(CursorEvent.MoveUp)
                | EditingEvent.Cursor(CursorEvent.MoveDown)
                | EditingEvent.Cursor(CursorEvent.MoveWordLeft)
                | EditingEvent.Cursor(CursorEvent.MoveWordRight)
                | EditingEvent.Cursor(CursorEvent.MoveToLineStart)
                | EditingEvent.Cursor(CursorEvent.MoveToLineEnd)
                | EditingEvent.Cursor(CursorEvent.MoveToDocumentStart)
                | EditingEvent.Cursor(CursorEvent.MoveToDocumentEnd) ->
                    this.ApplyEditingEvent(EditingEvent.Selection SelectionEvent.ClearSelection)
                | _ -> ()

            e.Handled <- true
        | None -> ()

    override this.OnPointerPressed(e: PointerPressedEventArgs) =
        base.OnPointerPressed(e)

        let point = e.GetCurrentPoint(this)

        let input =
            Functor.Avalonia.InputAdapter.pointerInput
                point.Position
                (Some Avalonia.Input.MouseButton.Left)
                point.Properties.IsLeftButtonPressed
                e.KeyModifiers

        if input.Button = Some Functor.Input.PointerButton.Left && input.IsPressed then
            this.Focus() |> ignore
            isPointerSelecting <- true
            e.Pointer.Capture(this) |> ignore

            let position =
                this.PositionAtPoint(Avalonia.Point(input.Position.X, input.Position.Y))

            this.ApplyEditingEvent(EditingEvent.Cursor(CursorEvent.SetCursor position))
            this.ApplyEditingEvent(EditingEvent.Selection SelectionEvent.StartSelection)
            e.Handled <- true

    override this.OnPointerMoved(e: PointerEventArgs) =
        base.OnPointerMoved(e)

        if isPointerSelecting && e.GetCurrentPoint(this).Properties.IsLeftButtonPressed then
            let point = e.GetCurrentPoint(this)

            let input =
                Functor.Avalonia.InputAdapter.pointerInput
                    point.Position
                    (Some Avalonia.Input.MouseButton.Left)
                    point.Properties.IsLeftButtonPressed
                    e.KeyModifiers

            let position =
                this.PositionAtPoint(Avalonia.Point(input.Position.X, input.Position.Y))

            this.ApplyEditingEvent(EditingEvent.Cursor(CursorEvent.SetCursor position))
            this.ApplyEditingEvent(EditingEvent.Selection SelectionEvent.UpdateSelection)
            e.Handled <- true

    override this.OnPointerReleased(e: PointerReleasedEventArgs) =
        base.OnPointerReleased(e)

        if isPointerSelecting then
            let point = e.GetCurrentPoint(this)

            let input =
                Functor.Avalonia.InputAdapter.pointerInput
                    point.Position
                    (Some Avalonia.Input.MouseButton.Left)
                    false
                    e.KeyModifiers

            let position =
                this.PositionAtPoint(Avalonia.Point(input.Position.X, input.Position.Y))

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

    member private this.RenderEditor(context: DrawingContext) =

        let frame: RenderingModel =
            RenderingPipeline.render renderingConfig session.Model
            |> fun frame ->
                if this.IsFocused then
                    frame
                else
                    { frame with Cursors = [] }

        let bounds = Avalonia.Rect(0.0, 0.0, this.Bounds.Width, this.Bounds.Height)

        let theme: RenderingSurface.ThemeSnapshot =
            { Ui = themeSettings.Ui
              Palette = themeSettings.ThemeSource.Resolve() }

        RenderBackend.drawFrame renderBackend context bounds frame theme
