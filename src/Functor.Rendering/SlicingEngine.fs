namespace Functor.Rendering

open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Domain.Diagnostics

type RenderInput =
    { Buffer: string list
      View: ViewState
      Editing: EditingModel
      Syntax: SyntaxModel
      Diagnostics: DiagnosticsModel }

module RenderInput =
    let fromCoreModel (model: CoreModel) : RenderInput =
        { Buffer = model.Editing.Buffer
          View = model.View
          Editing = model.Editing
          Syntax = model.Syntax
          Diagnostics = model.Diagnostics }

module SlicingEngine =

    // Helper: get all buffer lines as (index * text)
    let private getBufferLines (input: RenderInput) : string list =
        input.Buffer

    let private visibleLineRange visibleLineCount (input: RenderInput) totalLines =
        let firstLine = max 0 input.View.VerticalOffset
        let lastLine = min (totalLines - 1) (firstLine + max 1 visibleLineCount - 1)
        firstLine, lastLine

    /// Visible lines based on vertical offset and the number of lines that fit in the viewport.
    let sliceLines visibleLineCount (input: RenderInput) : list<int * string> =
        let lines = getBufferLines input
        let totalLines = lines.Length

        if totalLines = 0 then
            []
        else
            let firstLine, lastLine = visibleLineRange visibleLineCount input totalLines

            [ for i in firstLine..lastLine -> i, lines.[i] ]

    /// Visible tokens: intersect tokens with visible line range.
    let sliceTokens visibleLineCount (input: RenderInput) : list<Token> =
        let firstLine, lastLine =
            visibleLineRange visibleLineCount input input.Buffer.Length

        input.Syntax.Tokens
        |> List.filter (fun lt -> lt.Line >= firstLine && lt.Line <= lastLine)
        |> List.collect (fun lt -> lt.Tokens)

    /// Visible selections: intersect selection ranges with visible line range.
    let sliceSelections visibleLineCount (input: RenderInput) : list<Range> =
        let firstLine, lastLine =
            visibleLineRange visibleLineCount input input.Buffer.Length

        input.Editing.Selection
        |> Option.toList
        |> List.map (fun selection ->
            let range: Range =
                { Start = selection.Start
                  End = selection.End }

            Range.normalize range)
        |> List.filter (fun selection -> selection.End.Line >= firstLine && selection.Start.Line <= lastLine)

    /// Visible cursors: keep cursors whose positions fall inside visible lines.
    let sliceCursors visibleLineCount (input: RenderInput) : list<Position> =
        let firstLine, lastLine =
            visibleLineRange visibleLineCount input input.Buffer.Length

        [ input.Editing.Cursor ]
        |> List.filter (fun p -> p.Line >= firstLine && p.Line <= lastLine)

    /// Visible diagnostics: intersect diagnostic ranges with visible line range.
    let sliceDiagnostics visibleLineCount (input: RenderInput) : list<Diagnostic> =
        let firstLine, lastLine =
            visibleLineRange visibleLineCount input input.Buffer.Length

        input.Diagnostics.All
        |> List.filter (fun d -> d.RangeEnd.Line >= firstLine && d.RangeStart.Line <= lastLine)

    type SlicedSpans =
        { Lines: list<int * string>
          Tokens: list<Token>
          Selections: list<Range>
          Cursors: list<Position>
          Diagnostics: list<Diagnostic> }

    let sliceAll visibleLineCount (input: RenderInput) : SlicedSpans =
        { Lines = sliceLines visibleLineCount input
          Tokens = sliceTokens visibleLineCount input
          Selections = sliceSelections visibleLineCount input
          Cursors = sliceCursors visibleLineCount input
          Diagnostics = sliceDiagnostics visibleLineCount input }
