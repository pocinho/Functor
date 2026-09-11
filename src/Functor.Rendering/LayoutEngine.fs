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
    { LineIndex: int; Range: Range; Style: TextStyle; XStart: float32; XEnd: float32; Y: float32 }

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
    { Lines: list<LineLayout>; Tokens: list<TokenLayout>; Selections: list<SelectionLayout>; Cursors: list<CursorLayout>; Diagnostics: list<DiagnosticLayout>; LineNumbers: list<LineNumber> }

/// The LayoutEngine is responsible for converting sliced spans
/// (still in Position/Range space) into pixel geometry using font metrics
/// and viewport information.
///
/// It does NOT draw anything; it only computes geometry.
module LayoutEngine =

    let private normalizePosition (measurer: TextMeasurer) (lines: list<LineLayout>) (position: Position) =
        lines
        |> List.tryFind (fun line -> line.LineIndex = position.Line)
        |> Option.map (fun line ->
            { position with Column = TextMeasurer.normalizeColumn line.Text position.Column })
        |> Option.defaultValue position

    let gutterWidth (measurer: TextMeasurer) (lineCount: int) =
        let numberText = string (max 1 lineCount)
        measurer.MeasureRange numberText 0 numberText.Length + 16.0f

    let maxVerticalOffset (visibleLineCount: int) (buffer: string list) =
        max 0 (buffer.Length - max 1 visibleLineCount)

    let maxHorizontalOffset
        (measurer: TextMeasurer)
        (viewportWidth: float32)
        (gutterWidth: float32)
        (buffer: string list)
        =
        let availableWidth = max 0.0f (viewportWidth - gutterWidth)
        let maxOffsetForLine text =
            let widths = measurer.MeasurePrefix text
            let targetWidth = max 0.0f (widths.[text.Length] - availableWidth)
            let mutable low = 0
            let mutable high = widths.Length

            while low < high do
                let middle = low + ((high - low) / 2)

                if widths.[middle] <= targetWidth then
                    low <- middle + 1
                else
                    high <- middle

            TextMeasurer.normalizeColumn text (low - 1)

        buffer |> List.map maxOffsetForLine |> List.fold max 0

    let positionAtPoint
        (measurer: TextMeasurer)
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
                + max 0 (int (System.Math.Floor(float y / float measurer.Metrics.LineHeight)))

            let line = max 0 (min (buffer.Length - 1) line)
            let text = buffer.[line]

            let lineX = -measurer.MeasureRange text 0 horizontalOffset
            let column = measurer.HitTestColumn text (x - lineX)

            { Line = line
              Column = max 0 (min text.Length column) }

    let positionAtPointWithGutter
        (measurer: TextMeasurer)
        (gutterWidth: float32)
        (horizontalOffset: int)
        (verticalOffset: int)
        (buffer: string list)
        (x: float32)
        (y: float32)
        : Position =
        positionAtPoint measurer horizontalOffset verticalOffset buffer (x - gutterWidth) y

    /// Layout visible lines into pixel space.
    let layoutLinesWithGutter
        (measurer: TextMeasurer)
        (gutterWidth: float32)
        (horizontalOffset: int)
        (lines: list<int * string>)
        : list<LineLayout> =
        lines
        |> List.mapi (fun visibleIndex (lineIndex, text) ->
            { LineIndex = lineIndex
              Text = text
              X = gutterWidth - measurer.MeasureRange text 0 horizontalOffset
              Y = float32 visibleIndex * measurer.Metrics.LineHeight
              Height = measurer.Metrics.LineHeight })

    let layoutLines (measurer: TextMeasurer) (horizontalOffset: int) (lines: list<int * string>) : list<LineLayout> =
        layoutLinesWithGutter measurer 0.0f horizontalOffset lines

    /// Layout visible tokens into pixel space.
    let layoutTokens (measurer: TextMeasurer) (lines: list<LineLayout>) (tokens: list<Token>) : list<TokenLayout> =
        tokens
        |> List.choose (fun token ->
            lines
            |> List.tryFind (fun line -> line.LineIndex = token.Line)
            |> Option.map (fun line ->
                let startColumn, endColumn = TextMeasurer.normalizeRange line.Text token.Column token.Length

                let tokenRange: Range =
                    { Start =
                        { Line = token.Line
                          Column = startColumn }
                      End =
                        { Line = token.Line
                          Column = endColumn } }

                { LineIndex = token.Line
                  Range = tokenRange
                  Style = TextStyle.forTokenKind token.Kind
                  XStart = line.X + measurer.MeasureRange line.Text 0 startColumn
                  XEnd = line.X + measurer.MeasureRange line.Text 0 endColumn
                  Y = line.Y }))

    let layoutTextRuns
        (measurer: TextMeasurer)
        (lines: list<LineLayout>)
        (tokens: list<TokenLayout>)
        : list<VisibleTextRun> =
        let visualColumn text column =
            if measurer.Metrics.DefaultAdvance <= 0.0f then
                0
            else
                int (measurer.MeasureRange text 0 column / measurer.Metrics.DefaultAdvance)

        let createRun (line: LineLayout) startColumn endColumn (style: TextStyle) : VisibleTextRun =
            { LineIndex = line.LineIndex
              Text = line.Text.Substring(startColumn, endColumn - startColumn)
              Range =
                { Start = { Line = line.LineIndex; Column = startColumn }
                  End = { Line = line.LineIndex; Column = endColumn } }
              X = line.X + measurer.MeasureRange line.Text 0 startColumn
              Y = line.Y
              Height = line.Height
              StartVisualColumn = visualColumn line.Text startColumn
              Style = style }

        lines
        |> List.collect (fun (line: LineLayout) ->
            let lineTokens =
                tokens
                |> List.filter (fun token -> token.LineIndex = line.LineIndex)
                |> List.sortBy (fun token -> token.Range.Start.Column, token.Range.End.Column)

            let runs, column =
                lineTokens
                |> List.fold (fun (runs, column) token ->
                    let startColumn = max column token.Range.Start.Column
                    let endColumn = max startColumn token.Range.End.Column
                    let runs =
                        if column < startColumn then
                            createRun line column startColumn TextStyle.defaultStyle :: runs
                        else
                            runs

                    if startColumn < endColumn then
                        createRun line startColumn endColumn token.Style :: runs, endColumn
                    else
                        runs, column) ([], 0)

            let runs =
                if column < line.Text.Length then
                    createRun line column line.Text.Length TextStyle.defaultStyle :: runs
                elif line.Text.Length = 0 then
                    createRun line 0 0 TextStyle.defaultStyle :: runs
                else
                    runs

            List.rev runs)

    /// Layout selections into pixel rectangles.
    let layoutSelections
        (measurer: TextMeasurer)
        (horizontalOffset: int)
        (lines: list<LineLayout>)
        (selections: list<Range>)
        : list<SelectionLayout> =
        selections
        |> List.map (fun selection ->
            let selection: Range =
                Range.normalize selection
                |> fun range ->
                    { Start = normalizePosition measurer lines range.Start
                      End = normalizePosition measurer lines range.End }

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

                        let startColumn =
                            if line.LineIndex = selection.Start.Line then
                                TextMeasurer.normalizeColumn line.Text startColumn
                            else
                                0

                        let endColumn =
                            if line.LineIndex = selection.End.Line then
                                snd (TextMeasurer.normalizeRange line.Text 0 endColumn)
                            else
                                line.Text.Length

                        let startColumn = max 0 (min line.Text.Length startColumn)
                        let endColumn = max startColumn (min line.Text.Length endColumn)

                        if startColumn = endColumn then
                            None
                        else
                            Some
                                { X = line.X + measurer.MeasureRange line.Text 0 startColumn
                                  Y = line.Y
                                  Width = measurer.MeasureRange line.Text startColumn (endColumn - startColumn)
                                  Height = line.Height })

            { Range = selection; Rects = rects })

    /// Layout cursors into pixel geometry.
    let layoutCursors
        (measurer: TextMeasurer)
        (horizontalOffset: int)
        (lines: list<LineLayout>)
        (cursors: list<Position>)
        : list<CursorLayout> =
        cursors
        |> List.choose (fun position ->
            lines
            |> List.tryFind (fun line -> line.LineIndex = position.Line)
            |> Option.map (fun line ->
                let column = TextMeasurer.normalizeColumn line.Text position.Column

                { Position = { position with Column = column }
                  X = line.X + measurer.MeasureRange line.Text 0 column
                  Y = line.Y
                  Height = line.Height
                  Width = max 1.0f (measurer.Metrics.DefaultAdvance * 0.1f) }))

    /// Layout diagnostics into glyph + underline geometry.
    let layoutDiagnostics
        (measurer: TextMeasurer)
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

                        let startColumn =
                            if line.LineIndex = range.Start.Line then
                                TextMeasurer.normalizeColumn line.Text startColumn
                            else
                                0

                        let endColumn =
                            if line.LineIndex = range.End.Line then
                                snd (TextMeasurer.normalizeRange line.Text 0 endColumn)
                            else
                                line.Text.Length

                        let startColumn = max 0 (min line.Text.Length startColumn)
                        let endColumn = max startColumn (min line.Text.Length endColumn)

                        if startColumn = endColumn then
                            None
                        else
                            Some
                                { X = line.X + measurer.MeasureRange line.Text 0 startColumn
                                  Y = line.Y + line.Height * 0.8f
                                  Width = measurer.MeasureRange line.Text startColumn (endColumn - startColumn)
                                  Height = max 1.0f (line.Height * 0.08f) })

            let glyph =
                lines
                |> List.tryFind (fun line -> line.LineIndex = range.Start.Line)
                |> Option.map (fun line ->
                    let column = TextMeasurer.normalizeColumn line.Text range.Start.Column

                    { X = line.X + measurer.MeasureRange line.Text 0 column
                      Y = line.Y + line.Height * 0.15f
                      Width = max 1.0f (measurer.Metrics.DefaultAdvance * 0.2f)
                      Height = max 1.0f (line.Height * 0.2f) })

            { Range = range
              Severity = diagnostic.Severity
              Glyph = glyph
              Underline = underline })

    /// Layout line numbers into gutter geometry.
    let layoutLineNumbersWithGutter
        (measurer: TextMeasurer)
        (gutterWidth: float32)
        (lines: list<LineLayout>)
        : list<LineNumber> =
        lines
        |> List.map (fun line ->
            let text = string (line.LineIndex + 1)

            { LineIndex = line.LineIndex
              Text = text
              X = gutterWidth - measurer.MeasureRange text 0 text.Length - 4.0f
              Y = line.Y })

    let layoutLineNumbers (measurer: TextMeasurer) (lines: list<LineLayout>) : list<LineNumber> =
        layoutLineNumbersWithGutter measurer 0.0f lines

    /// Run the full layout pipeline.
    let layoutAll
        (measurer: TextMeasurer)
        (viewport: Viewport)
        (gutterWidth: float32)
        (horizontalOffset: int)
        (sliced: SlicingEngine.SlicedSpans)
        : LayoutResult =
        let lines = layoutLinesWithGutter measurer gutterWidth horizontalOffset sliced.Lines
        let tokens = layoutTokens measurer lines sliced.Tokens
        let selections = layoutSelections measurer horizontalOffset lines sliced.Selections
        let cursors = layoutCursors measurer horizontalOffset lines sliced.Cursors
        let diagnostics = layoutDiagnostics measurer lines sliced.Diagnostics
        let lineNumbers = layoutLineNumbersWithGutter measurer gutterWidth lines

        { Lines = lines
          Tokens = tokens
          Selections = selections
          Cursors = cursors
          Diagnostics = diagnostics
          LineNumbers = lineNumbers }
