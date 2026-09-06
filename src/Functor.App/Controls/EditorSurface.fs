namespace Functor.App

open Avalonia
open Avalonia.Controls
open Avalonia.Input
open Avalonia.Skia
open SkiaSharp

open Functor.Domain.Core
open Functor.Domain.CoreEvent
open Functor.Rendering

/// EditorSurface is the Avalonia control that hosts the Skia canvas.
/// It connects user input → CoreEvent, and rendering pipeline → Skia drawing.
///
/// IMPORTANT:
/// - EditorSurface does NOT compute geometry.
/// - It does NOT slice tokens.
/// - It does NOT layout text.
/// - It ONLY orchestrates input + pipeline + drawing.
type EditorSurface() as this =
    inherit Control()

    // The current CoreModel (editor state).
    // In the real implementation, this will be injected or bound.
    let mutable coreModel : CoreModel = CoreModel.empty

    // Rendering configuration (font metrics, theme, etc.)
    let mutable renderingConfig : RenderingPipeline.RenderingConfig =
        { Metrics = { LineHeight = 16.0f; CharWidth = 8.0f } }

    // Request a redraw when the model changes.
    member private this.Invalidate() =
        this.InvalidateVisual()

    // Update the CoreModel using CoreEvent and trigger redraw.
    member private this.ApplyEvent(evt: CoreEvent) =
        coreModel <- CoreLogic.update evt coreModel
        this.Invalidate()

    // ─────────────────────────────────────────────────────────────
    // Rendering
    // ─────────────────────────────────────────────────────────────

    override this.Render(context: DrawingContext) =
        match context.PlatformImpl with
        | :? ISkiaDrawingContextImpl as skia ->
            let canvas = skia.SkCanvas

            // 1. Run the rendering pipeline
            let frame = RenderingPipeline.render renderingConfig coreModel

            // 2. Draw the frame
            RenderingSurface.draw canvas frame
        | _ ->
            ()

    // ─────────────────────────────────────────────────────────────
    // Input Handling
    // ─────────────────────────────────────────────────────────────

    override this.OnKeyDown(e: KeyEventArgs) =
        base.OnKeyDown(e)

        // Translate Avalonia key → CoreEvent
        // (Implementation will be added later)
        ()

    override this.OnTextInput(e: TextInputEventArgs) =
        base.OnTextInput(e)

        // Translate typed characters → CoreEvent.InsertChar
        // (Implementation will be added later)
        ()

    override this.OnPointerPressed(e: PointerPressedEventArgs) =
        base.OnPointerPressed(e)

        // Translate pointer → cursor movement / selection start
        // (Implementation will be added later)
        ()

    override this.OnPointerMoved(e: PointerEventArgs) =
        base.OnPointerMoved(e)

        // Translate pointer drag → selection update
        // (Implementation will be added later)
        ()

    override this.OnPointerWheelChanged(e: PointerWheelEventArgs) =
        base.OnPointerWheelChanged(e)

        // Translate scroll → CoreEvent.ScrollVertical / ScrollHorizontal
        // (Implementation will be added later)
        ()

    // ─────────────────────────────────────────────────────────────
    // Focus Handling
    // ─────────────────────────────────────────────────────────────

    override this.OnGotFocus(e: GotFocusEventArgs) =
        base.OnGotFocus(e)
        // Future: IME composition start, cursor blinking, etc.

    override this.OnLostFocus(e: RoutedEventArgs) =
        base.OnLostFocus(e)
        // Future: stop cursor blinking, commit IME, etc.

    // ─────────────────────────────────────────────────────────────
    // Public API
    // ─────────────────────────────────────────────────────────────

    /// Replace the CoreModel (e.g., when loading a file).
    member this.SetModel(model: CoreModel) =
        coreModel <- model
        this.Invalidate()

    /// Replace rendering configuration (theme, font metrics, etc.)
    member this.SetRenderingConfig(config: RenderingPipeline.RenderingConfig) =
        renderingConfig <- config
        this.Invalidate()
