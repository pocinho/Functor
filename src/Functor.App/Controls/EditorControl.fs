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

    let renderingConfig =
        { RenderingPipeline.Metrics = { LineHeight = 16.0f; CharWidth = 8.0f } }

    member private this.ApplyEditingEvent(event: EditingEvent) =
        coreModel <- CoreLogic.update (CoreEvent.ApplyEditingEvent event) coreModel
        this.InvalidateVisual()

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

    override this.Render(context: DrawingContext) =
        base.Render(context)

        let frame: RenderingModel = RenderingPipeline.render renderingConfig coreModel

        let bounds = Rect(0.0, 0.0, this.Bounds.Width, this.Bounds.Height)

        RenderingSurface.draw context bounds frame
