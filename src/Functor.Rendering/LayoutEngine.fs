namespace Functor.Rendering

open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Domain.Diagnostics

/// Represents font and layout metrics used by the layout engine.
type FontMetrics =
    { LineHeight: float32
      CharWidth: float32
    /// Optional: more advanced metrics later (ascender, descender, etc.)
    }

/// Represents a laid-out line in pixel space.
type LineLayout =
    { LineIndex: int
      Text: string
      X: float32
      Y: float32
      Height: float32 }

/// Represents a laid-out token in pixel space.
type TokenLayout =
    { LineIndex: int
      Range: Range
      Style: SyntaxStyle
      XStart: float32
      XEnd: float32
      Y: float32 }

/// Represents laid-out selection geometry (rectangles in pixel space).
type SelectionLayout = { Range: Range; Rects: list<Rect> }

/// Represents laid-out cursor geometry.
type CursorLayout =
    { Position: Position
      X: float32
      Y: float32
      Height: float32
      Width: float32 }

/// Represents laid-out diagnostics geometry.
type DiagnosticLayout =
    { Range: Range
      Severity: DiagnosticSeverity
      Glyph: Rect option
      Underline: list<Rect> }

/// Aggregated layout result, ready to be turned into a RenderingModel.
type LayoutResult =
    { Lines: list<LineLayout>
      Tokens: list<TokenLayout>
      Selections: list<SelectionLayout>
      Cursors: list<CursorLayout>
      Diagnostics: list<DiagnosticLayout>
      LineNumbers: list<LineNumber> }

/// The LayoutEngine is responsible for converting sliced spans
/// (still in Position/Range space) into pixel geometry using font metrics
/// and viewport information.
///
/// It does NOT draw anything; it only computes geometry.
module LayoutEngine =

    /// Layout visible lines into pixel space.
    let layoutLines (metrics: FontMetrics) (horizontalOffset: int) (lines: list<int * string>) : list<LineLayout> =
        lines
        |> List.mapi (fun visibleIndex (lineIndex, text) ->
            { LineIndex = lineIndex
              Text = text
              X = -float32 horizontalOffset * metrics.CharWidth
              Y = float32 visibleIndex * metrics.LineHeight
              Height = metrics.LineHeight })

    /// Layout visible tokens into pixel space.
    let layoutTokens (metrics: FontMetrics) (lines: list<LineLayout>) (tokens: list<Token>) : list<TokenLayout> =
        // Implementation will be added later.
        []

    /// Layout selections into pixel rectangles.
    let layoutSelections
        (metrics: FontMetrics)
        (horizontalOffset: int)
        (lines: list<LineLayout>)
        (selections: list<Range>)
        : list<SelectionLayout> =
        selections
        |> List.map (fun selection ->
            let selection = Range.normalize selection

            let rects =
                lines
                |> List.choose (fun line ->
                    if line.LineIndex < selection.Start.Line || line.LineIndex > selection.End.Line then
                        None
                    else
                        let startColumn =
                            if line.LineIndex = selection.Start.Line then
                                selection.Start.Column
                            else
                                0

                        let endColumn =
                            if line.LineIndex = selection.End.Line then
                                selection.End.Column
                            else
                                line.Text.Length

                        let startColumn = max 0 (min line.Text.Length startColumn)
                        let endColumn = max startColumn (min line.Text.Length endColumn)

                        if startColumn = endColumn then
                            None
                        else
                            Some
                                { X = float32 (startColumn - horizontalOffset) * metrics.CharWidth
                                  Y = line.Y
                                  Width = float32 (endColumn - startColumn) * metrics.CharWidth
                                  Height = line.Height })

            { Range = selection; Rects = rects })

    /// Layout cursors into pixel geometry.
    let layoutCursors
        (metrics: FontMetrics)
        (horizontalOffset: int)
        (lines: list<LineLayout>)
        (cursors: list<Position>)
        : list<CursorLayout> =
        cursors
        |> List.choose (fun position ->
            lines
            |> List.tryFind (fun line -> line.LineIndex = position.Line)
            |> Option.map (fun line ->
                { Position = position
                  X = float32 (position.Column - horizontalOffset) * metrics.CharWidth
                  Y = line.Y
                  Height = line.Height
                  Width = max 1.0f (metrics.CharWidth * 0.1f) }))

    /// Layout diagnostics into glyph + underline geometry.
    let layoutDiagnostics
        (metrics: FontMetrics)
        (lines: list<LineLayout>)
        (diagnostics: list<Diagnostic>)
        : list<DiagnosticLayout> =
        // Implementation will be added later.
        []

    /// Layout line numbers into gutter geometry.
    let layoutLineNumbers (metrics: FontMetrics) (lines: list<LineLayout>) : list<LineNumber> =
        // Implementation will be added later.
        []

    /// Run the full layout pipeline.
    let layoutAll
        (metrics: FontMetrics)
        (viewport: Viewport)
        (horizontalOffset: int)
        (sliced: SlicingEngine.SlicedSpans)
        : LayoutResult =
        let lines = layoutLines metrics horizontalOffset sliced.Lines
        let tokens = layoutTokens metrics lines sliced.Tokens
        let selections = layoutSelections metrics horizontalOffset lines sliced.Selections
        let cursors = layoutCursors metrics horizontalOffset lines sliced.Cursors
        let diagnostics = layoutDiagnostics metrics lines sliced.Diagnostics
        let lineNumbers = layoutLineNumbers metrics lines

        { Lines = lines
          Tokens = tokens
          Selections = selections
          Cursors = cursors
          Diagnostics = diagnostics
          LineNumbers = lineNumbers }
