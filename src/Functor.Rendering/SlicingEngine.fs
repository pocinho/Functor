namespace Functor.Rendering

open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Domain.Diagnostics

module SlicingEngine =

    // Helper: get all buffer lines as (index * text)
    let private getBufferLines (model: CoreModel) : string list =
        // Adjust to your actual buffer representation
        model.Editing.Buffer

    let private visibleLineRange visibleLineCount (model: CoreModel) totalLines =
        let firstLine = max 0 model.VerticalOffset
        let lastLine = min (totalLines - 1) (firstLine + max 1 visibleLineCount - 1)
        firstLine, lastLine

    /// Visible lines based on vertical offset and the number of lines that fit in the viewport.
    let sliceLines visibleLineCount (model: CoreModel) : list<int * string> =
        let lines = getBufferLines model
        let totalLines = lines.Length

        if totalLines = 0 then
            []
        else
            let firstLine, lastLine = visibleLineRange visibleLineCount model totalLines

            [ for i in firstLine..lastLine -> i, lines.[i] ]

    /// Visible tokens: intersect tokens with visible line range.
    let sliceTokens visibleLineCount (model: CoreModel) (syntax: SyntaxModel) : list<Token> =
        let firstLine, lastLine =
            visibleLineRange visibleLineCount model model.Editing.Buffer.Length

        syntax.Tokens
        |> List.filter (fun lt -> lt.Line >= firstLine && lt.Line <= lastLine)
        |> List.collect (fun lt -> lt.Tokens)


    /// Visible selections: intersect selection ranges with visible line range.
    let sliceSelections visibleLineCount (model: CoreModel) : list<Range> =
        let firstLine, lastLine =
            visibleLineRange visibleLineCount model model.Editing.Buffer.Length

        model.Editing.Selection
        |> Option.toList
        |> List.map (fun selection ->
            let range: Range =
                { Start = selection.Start
                  End = selection.End }

            Range.normalize range)
        |> List.filter (fun selection -> selection.End.Line >= firstLine && selection.Start.Line <= lastLine)

    /// Visible cursors: keep cursors whose positions fall inside visible lines.
    let sliceCursors visibleLineCount (model: CoreModel) : list<Position> =
        let firstLine, lastLine =
            visibleLineRange visibleLineCount model model.Editing.Buffer.Length

        [ model.Editing.Cursor ]
        |> List.filter (fun p -> p.Line >= firstLine && p.Line <= lastLine)

    /// Visible diagnostics: intersect diagnostic ranges with visible line range.
    let sliceDiagnostics visibleLineCount (model: CoreModel) (diagnostics: DiagnosticsModel) : list<Diagnostic> =
        let firstLine, lastLine =
            visibleLineRange visibleLineCount model model.Editing.Buffer.Length

        diagnostics.All
        |> List.filter (fun d -> d.RangeEnd.Line >= firstLine && d.RangeStart.Line <= lastLine)

    type SlicedSpans =
        { Lines: list<int * string>
          Tokens: list<Token>
          Selections: list<Range>
          Cursors: list<Position>
          Diagnostics: list<Diagnostic> }

    let sliceAll visibleLineCount (model: CoreModel) : SlicedSpans =
        { Lines = sliceLines visibleLineCount model
          Tokens = sliceTokens visibleLineCount model model.Syntax
          Selections = sliceSelections visibleLineCount model
          Cursors = sliceCursors visibleLineCount model
          Diagnostics = sliceDiagnostics visibleLineCount model model.Diagnostics }
