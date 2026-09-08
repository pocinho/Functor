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

    /// Visible lines based on vertical offset + viewport height (in lines).
    let sliceLines (model: CoreModel) : list<int * string> =
        let lines = getBufferLines model
        let totalLines = lines.Length

        // Assume viewport height is expressed in lines for now
        let firstLine = max 0 model.VerticalOffset
        if firstLine >= totalLines then
            []
        else
            let lastLine =
                firstLine + 50
                |> min (totalLines - 1)

            [ for i in firstLine .. lastLine ->
                i, lines.[i] ]

    /// Visible tokens: intersect tokens with visible line range.
    let sliceTokens (model: CoreModel) (syntax: SyntaxModel) : list<Token> =
        let firstLine = max 0 model.VerticalOffset
        let lastLine = firstLine + 50 // temporary until LayoutEngine provides line height

        syntax.Tokens
        |> List.filter (fun lt ->
            lt.Line >= firstLine &&
            lt.Line <= lastLine)
        |> List.collect (fun lt -> lt.Tokens)


        // Horizontal slicing can be added later once LayoutEngine is in place.

    /// Visible selections: intersect selection ranges with visible line range.
    let sliceSelections (model: CoreModel) : list<Range> =
        let firstLine = max 0 model.VerticalOffset
        let lastLine =
            firstLine + 50

        model.Editing.Selection
        |> Option.toList
        |> List.filter (fun selection ->
            selection.End.Line >= firstLine &&
            selection.Start.Line <= lastLine)
        |> List.map (fun selection ->
            {
                Start = selection.Start
                End = selection.End
            })

    /// Visible cursors: keep cursors whose positions fall inside visible lines.
    let sliceCursors (model: CoreModel) : list<Position> =
        let firstLine = max 0 model.VerticalOffset
        let lastLine =
            firstLine + 50

        [ model.Editing.Cursor ]
        |> List.filter (fun p ->
            p.Line >= firstLine && p.Line <= lastLine)

    /// Visible diagnostics: intersect diagnostic ranges with visible line range.
    let sliceDiagnostics (model: CoreModel) (diagnostics: DiagnosticsModel) : list<Diagnostic> =
        let firstLine = max 0 model.VerticalOffset
        let lastLine =
            firstLine + 50

        diagnostics.All
        |> List.filter (fun d ->
            d.RangeEnd.Line >= firstLine &&
            d.RangeStart.Line <= lastLine)

    type SlicedSpans =
        {
            Lines : list<int * string>
            Tokens : list<Token>
            Selections : list<Range>
            Cursors : list<Position>
            Diagnostics : list<Diagnostic>
        }

    let sliceAll (model: CoreModel) : SlicedSpans =
        {
            Lines = sliceLines model
            Tokens = sliceTokens model model.Syntax
            Selections = sliceSelections model
            Cursors = sliceCursors model
            Diagnostics = sliceDiagnostics model model.Diagnostics
        }
