namespace Functor.App.Controls

open Avalonia
open Avalonia.Controls
open Avalonia.Input
open Avalonia.Media
open Avalonia.Skia

open Functor.Domain.Core
open Functor.App.Rendering
open Functor.Rendering

type EditorControl() =
    inherit Control()

    // ------------------------------------------------------------
    // Internal state
    // ------------------------------------------------------------

    let mutable coreModel: CoreModel = CoreModel.empty

    let renderingConfig =
        { RenderingPipeline.Metrics = { LineHeight = 16.0f; CharWidth = 8.0f } }

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

    override this.OnKeyDown(e: KeyEventArgs) =
        base.OnKeyDown(e)

        let evt =
            match e.Key with
            | Key.Left -> CoreEvent.MoveCursorLeft
            | Key.Right -> CoreEvent.MoveCursorRight
            | Key.Up -> CoreEvent.MoveCursorUp
            | Key.Down -> CoreEvent.MoveCursorDown
            | Key.Back -> CoreEvent.Backspace
            | Key.Enter -> CoreEvent.InsertNewline
            | _ ->
                if e.Key.ToString().Length = 1 then
                    CoreEvent.InsertChar(e.Key.ToString()[0])
                else
                    CoreEvent.NoOp

        coreModel <- CoreLogic.update evt coreModel
        this.InvalidateVisual()

    override this.Render(context: DrawingContext) =
        base.Render(context)

        let frame: RenderingModel = RenderingPipeline.render renderingConfig coreModel

        let bounds = Rect(0.0, 0.0, this.Bounds.Width, this.Bounds.Height)

        RenderingSurface.draw context bounds frame
