namespace Functor.Rendering

open Functor.Domain.Core
open Functor.Domain.Syntax
open Functor.Domain.Diagnostics

/// The RenderingPipeline orchestrates the full rendering process:
/// CoreModel → SlicedSpans → LayoutResult → RenderingModel.
///
/// It is pure and deterministic. It does not draw anything.
/// RenderingSurface consumes the final RenderingModel.
module RenderingPipeline =

    /// Represents the configuration needed for rendering.
    /// This will later include:
    /// - font metrics
    /// - theme colors
    /// - gutter width
    /// - tab width
    /// - soft-wrap settings
    type RenderingConfig = { Metrics: FontMetrics }

    /// Runs the full rendering pipeline and produces a RenderingModel.
    let render (config: RenderingConfig) (model: CoreModel) : RenderingModel =

        // 1. Slice domain state into visible spans (Position/Range space)
        let sliced = SlicingEngine.sliceAll model

        // 2. Layout spans into pixel geometry
        let layout = LayoutEngine.layoutAll config.Metrics model.Viewport sliced

        // 3. Convert layout result into a RenderingModel
        { VisibleLines =
            layout.Lines
            |> List.map (fun l ->
                { LineIndex = l.LineIndex
                  Text = l.Text
                  Y = l.Y
                  Height = l.Height })

          VisibleTokens =
            layout.Tokens
            |> List.map (fun t ->
                { LineIndex = t.LineIndex
                  Range = t.Range
                  Style = t.Style
                  XStart = t.XStart
                  XEnd = t.XEnd
                  Y = t.Y })

          Selections = layout.Selections |> List.map (fun s -> { Range = s.Range; Rects = s.Rects })

          Cursors =
            layout.Cursors
            |> List.map (fun (c: CursorLayout) ->
                { Position = c.Position
                  X = c.X
                  Y = c.Y
                  Height = c.Height
                  Width = c.Width })

          Diagnostics =
            layout.Diagnostics
            |> List.map (fun d ->
                { Range = d.Range
                  Severity = d.Severity
                  Glyph = d.Glyph
                  Underline = d.Underline })

          LineNumbers = layout.LineNumbers

          ViewportWidth = float32 model.Viewport.Width
          ViewportHeight = float32 model.Viewport.Height }
