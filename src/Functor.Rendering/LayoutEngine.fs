namespace Functor.Rendering

open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Domain.Diagnostics

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

    let maxVerticalOffset (visibleLineCount: int) (buffer: string list) =
        max 0 (buffer.Length - max 1 visibleLineCount)

    let positionAtPoint
        (metrics: TextMetrics)
        (horizontalOffset: int)
        (verticalOffset: int)
        (buffer: string list)
        (x: float32)
        (y: float32)
        : Position =
        if buffer.IsEmpty then
            { Line = 0; Column = 0 }
        else
            let line =
                verticalOffset
                + max 0 (int (System.Math.Floor(float y / float metrics.LineHeight)))

            let line = max 0 (min (buffer.Length - 1) line)
            let text = buffer.[line]

            let column =
                horizontalOffset
                + max 0 (int (System.Math.Floor(float x / float metrics.DefaultAdvance + 0.5)))

            { Line = line
              Column = max 0 (min text.Length column) }

    /// Layout visible lines into pixel space.
    let layoutLines (metrics: TextMetrics) (horizontalOffset: int) (lines: list<int * string>) : list<LineLayout> =
        lines
        |> List.mapi (fun visibleIndex (lineIndex, text) ->
            { LineIndex = lineIndex
              Text = text
              X = -float32 horizontalOffset * metrics.DefaultAdvance
              Y = float32 visibleIndex * metrics.LineHeight
              Height = metrics.LineHeight })

    /// Layout visible tokens into pixel space.
    let layoutTokens (metrics: TextMetrics) (lines: list<LineLayout>) (tokens: list<Token>) : list<TokenLayout> =
        tokens
        |> List.choose (fun token ->
            lines
            |> List.tryFind (fun line -> line.LineIndex = token.Line)
            |> Option.map (fun line ->
                let startColumn = max 0 (min line.Text.Length token.Column)

                let endColumn =
                    max startColumn (min line.Text.Length (token.Column + max 0 token.Length))

                let tokenRange: Range =
                    { Start =
                        { Line = token.Line
                          Column = startColumn }
                      End =
                        { Line = token.Line
                          Column = endColumn } }

                { LineIndex = token.Line
                  Range = tokenRange
                  Style = { Kind = token.Kind }
                  XStart = line.X + float32 startColumn * metrics.DefaultAdvance
                  XEnd = line.X + float32 endColumn * metrics.DefaultAdvance
                  Y = line.Y }))

    /// Layout selections into pixel rectangles.
    let layoutSelections
        (metrics: TextMetrics)
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
                                { X = float32 (startColumn - horizontalOffset) * metrics.DefaultAdvance
                                  Y = line.Y
                                  Width = float32 (endColumn - startColumn) * metrics.DefaultAdvance
                                  Height = line.Height })

            { Range = selection; Rects = rects })

    /// Layout cursors into pixel geometry.
    let layoutCursors
        (metrics: TextMetrics)
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
                  X = float32 (position.Column - horizontalOffset) * metrics.DefaultAdvance
                  Y = line.Y
                  Height = line.Height
                  Width = max 1.0f (metrics.DefaultAdvance * 0.1f) }))

    /// Layout diagnostics into glyph + underline geometry.
    let layoutDiagnostics
        (metrics: TextMetrics)
        (lines: list<LineLayout>)
        (diagnostics: list<Diagnostic>)
        : list<DiagnosticLayout> =
        diagnostics
        |> List.map (fun diagnostic ->
            let range =
                Range.normalize
                    { Start = diagnostic.RangeStart
                      End = diagnostic.RangeEnd }

            let underline =
                lines
                |> List.choose (fun line ->
                    if line.LineIndex < range.Start.Line || line.LineIndex > range.End.Line then
                        None
                    else
                        let startColumn =
                            if line.LineIndex = range.Start.Line then
                                range.Start.Column
                            else
                                0

                        let endColumn =
                            if line.LineIndex = range.End.Line then
                                range.End.Column
                            else
                                line.Text.Length

                        let startColumn = max 0 (min line.Text.Length startColumn)
                        let endColumn = max startColumn (min line.Text.Length endColumn)

                        if startColumn = endColumn then
                            None
                        else
                            Some
                                { X = line.X + float32 startColumn * metrics.DefaultAdvance
                                  Y = line.Y + line.Height * 0.8f
                                  Width = float32 (endColumn - startColumn) * metrics.DefaultAdvance
                                  Height = max 1.0f (line.Height * 0.08f) })

            let glyph =
                lines
                |> List.tryFind (fun line -> line.LineIndex = range.Start.Line)
                |> Option.map (fun line ->
                    let column = max 0 (min line.Text.Length range.Start.Column)

                    { X = line.X + float32 column * metrics.DefaultAdvance
                      Y = line.Y + line.Height * 0.15f
                      Width = max 1.0f (metrics.DefaultAdvance * 0.2f)
                      Height = max 1.0f (line.Height * 0.2f) })

            { Range = range
              Severity = diagnostic.Severity
              Glyph = glyph
              Underline = underline })

    /// Layout line numbers into gutter geometry.
    let layoutLineNumbers (metrics: TextMetrics) (lines: list<LineLayout>) : list<LineNumber> =
        lines
        |> List.map (fun line ->
            { LineIndex = line.LineIndex
              Text = string (line.LineIndex + 1)
              X = 0.0f
              Y = line.Y })

    /// Run the full layout pipeline.
    let layoutAll
        (metrics: TextMetrics)
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
