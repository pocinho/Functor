namespace Functor.Tests.Rendering

open Functor.Domain.Diagnostics
open Functor.Domain.Core
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Navigation
open Functor.Domain.Syntax
open Functor.Rendering
open Functor.Domain.Search
open Xunit
open TestFixtures

type LayoutEngineTests() =
    [<Fact>]
    member _.``vertical offset can scroll the final line to the top``() =
        Assert.Equal(99, LayoutEngine.maxVerticalOffset 10 (List.replicate 100 "line"))
        Assert.Equal(4, LayoutEngine.maxVerticalOffset 10 (List.replicate 5 "line"))

    [<Fact>]
    member _.``horizontal offset is bounded by measured content and gutter``() =
        let measurer = createMeasurer ()
        let gutter = LayoutEngine.gutterWidth measurer 10

        let maximum =
            LayoutEngine.maxHorizontalOffset measurer 80.0f gutter [ "1234567890" ]

        Assert.True(maximum > 0)
        Assert.Equal(0, LayoutEngine.maxHorizontalOffset measurer 200.0f gutter [ "short" ])

    [<Fact>]
    member _.``pointer coordinates map to document position``() =
        let measurer = createMeasurer ()
        let buffer = [ "first"; "second"; "third"; "fourth"; "fifth" ]
        let position = LayoutEngine.positionAtPoint measurer 2 3 buffer 20.0f 17.0f

        Assert.Equal({ Line = 4; Column = 5 }, position)

    [<Fact>]
    member _.``pointer coordinates clamp to document bounds``() =
        let measurer = createMeasurer ()
        let position = LayoutEngine.positionAtPoint measurer 0 0 [ "abc" ] -20.0f 1000.0f

        Assert.Equal({ Line = 0; Column = 0 }, position)

    [<Fact>]
    member _.``tokens are converted to styled pixel ranges``() =
        let measurer = createMeasurer ()
        let lines = LayoutEngine.layoutLines measurer 1 [ 2, "let value" ]

        let token =
            { Kind = "keyword"
              Line = 2
              Column = 0
              Length = 3 }

        let layout = LayoutEngine.layoutTokens measurer lines [ token ] |> List.exactlyOne

        Assert.Equal(2, layout.LineIndex)
        Assert.Equal(SyntaxForeground "keyword", layout.Style.Foreground)
        Assert.Equal(TextWeight.Bold, layout.Style.Weight)
        Assert.Equal(0, layout.Range.Start.Column)
        Assert.Equal(3, layout.Range.End.Column)
        Assert.Equal(-8.0f, layout.XStart)
        Assert.Equal(16.0f, layout.XEnd)

    [<Fact>]
    member _.``styled runs cover tokenized and unstyled text``() =
        let measurer = createMeasurer ()
        let lines = LayoutEngine.layoutLines measurer 0 [ 0, "let value" ]

        let token =
            { Kind = "keyword"
              Line = 0
              Column = 0
              Length = 3 }

        let tokens = LayoutEngine.layoutTokens measurer lines [ token ]
        let runs = LayoutEngine.layoutTextRuns measurer lines tokens

        Assert.Equal(2, runs.Length)
        Assert.Equal("let", runs.[0].Text)
        Assert.Equal(SyntaxForeground "keyword", runs.[0].Style.Foreground)
        Assert.Equal(" value", runs.[1].Text)
        Assert.Equal(EditorForeground, runs.[1].Style.Foreground)

    [<Fact>]
    member _.``token ranges use utf16 offsets and tab visual columns``() =
        let measurer = createMeasurer ()
        let lines = LayoutEngine.layoutLines measurer 0 [ 0, "😀\tlet" ]

        let token =
            { Kind = "keyword"
              Line = 0
              Column = 3
              Length = 3 }

        let tokens = LayoutEngine.layoutTokens measurer lines [ token ]

        let run =
            LayoutEngine.layoutTextRuns measurer lines tokens
            |> List.find (fun item -> item.Text = "let")

        Assert.Equal(3, run.Range.Start.Column)
        Assert.Equal(6, run.Range.End.Column)
        Assert.Equal(4, run.StartVisualColumn)
        Assert.Equal(32.0f, run.X)

    [<Fact>]
    member _.``cursor and selection geometry use backend grapheme advances``() =
        let measurer =
            TextMeasurer.create (
                TextMetrics.createWithGraphemeAdvance 16.0f 8.0f 4 (fun _ grapheme ->
                    if grapheme = "🚧" then 16.0f else 8.0f)
            )

        let lines = LayoutEngine.layoutLines measurer 0 [ 0, "a🚧b" ]

        let cursor =
            LayoutEngine.layoutCursors measurer 0 lines [ { Line = 0; Column = 3 } ]
            |> List.exactlyOne

        let selection =
            LayoutEngine.layoutSelections
                measurer
                0
                lines
                [ { Start = { Line = 0; Column = 1 }
                    End = { Line = 0; Column = 3 } } ]
            |> List.exactlyOne
            |> fun layout -> layout.Rects |> List.exactlyOne

        Assert.Equal(24.0f, cursor.X)
        Assert.Equal(8.0f, selection.X)
        Assert.Equal(16.0f, selection.Width)

    [<Fact>]
    member _.``overlapping and malformed tokens produce non-overlapping clamped runs``() =
        let measurer = createMeasurer ()
        let lines = LayoutEngine.layoutLines measurer 0 [ 0, "abcdef" ]

        let tokens =
            [ { Kind = "keyword"
                Line = 0
                Column = -2
                Length = 5 }
              { Kind = "string"
                Line = 0
                Column = 2
                Length = 20 } ]
            |> LayoutEngine.layoutTokens measurer lines

        let runs = LayoutEngine.layoutTextRuns measurer lines tokens

        Assert.Equal("abcdef", runs |> List.map (fun run -> run.Text) |> String.concat "")

        Assert.True(
            runs
            |> List.pairwise
            |> List.forall (fun (left, right) -> left.Range.End.Column = right.Range.Start.Column)
        )

    [<Fact>]
    member _.``token slicing returns only lines inside the viewport``() =
        let syntax =
            { SyntaxModel.empty with
                Tokens =
                    [ { Line = 0
                        Tokens =
                          [ { Kind = "first"
                              Line = 0
                              Column = 0
                              Length = 1 } ] }
                      { Line = 2
                        Tokens =
                          [ { Kind = "third"
                              Line = 2
                              Column = 0
                              Length = 1 } ] } ] }

        let input =
            { Buffer = [ "a"; "b"; "c" ]
              View =
                { Viewport = { Width = 80; Height = 16 }
                  VerticalOffset = 2
                  HorizontalOffset = 0 }
              Editing = EditingModel.create ()
              Syntax = syntax
              Diagnostics = DiagnosticsModel.create ()
              Search = SearchModel.create () }

        let tokens = SlicingEngine.sliceTokens 1 input

        Assert.Single(tokens) |> ignore
        Assert.Equal("third", tokens.Head.Kind)

    [<Fact>]
    member _.``search matches become visible active and inactive highlight geometry``() =
        let measurer = createMeasurer ()

        let document: SearchDocument =
            { Id = System.Guid.NewGuid()
              Path = None
              Name = "untitled"
              Lines = [ "term here"; "term again" ] }

        let matches = SearchEngine.findInDocument (SearchOptions.create "term") document

        let search =
            { SearchModel.create () with
                Matches = matches
                Index = Some 1 }

        let input =
            { Buffer = document.Lines
              View =
                { Viewport = { Width = 80; Height = 32 }
                  VerticalOffset = 0
                  HorizontalOffset = 0 }
              Editing = EditingModel.create ()
              Syntax = SyntaxModel.empty
              Diagnostics = DiagnosticsModel.create ()
              Search = search }

        let sliced = SlicingEngine.sliceAll 2 input
        let lines = LayoutEngine.layoutLines measurer 0 sliced.Lines

        let highlights =
            LayoutEngine.layoutSearchHighlights measurer 0 lines sliced.SearchMatches

        Assert.Equal(2, highlights.Length)
        Assert.False(highlights[0].IsActive)
        Assert.True(highlights[1].IsActive)
        Assert.Equal(32.0f, highlights[1].Rects.Head.Width)

    [<Fact>]
    member _.``multiple matches on one line retain independent geometry``() =
        let measurer = createMeasurer ()

        let document: SearchDocument =
            { Id = System.Guid.NewGuid()
              Path = None
              Name = "untitled"
              Lines = [ "term term" ] }

        let matches = SearchEngine.findInDocument (SearchOptions.create "term") document
        let lines = LayoutEngine.layoutLines measurer 0 [ 0, document.Lines.Head ]

        let highlights =
            LayoutEngine.layoutSearchHighlights measurer 0 lines (matches |> List.map (fun value -> value, false))

        Assert.Equal(2, highlights.Length)
        Assert.Equal(0.0f, highlights[0].Rects.Head.X)
        Assert.Equal(32.0f, highlights[0].Rects.Head.Width)
        Assert.Equal(40.0f, highlights[1].Rects.Head.X)
        Assert.Equal(32.0f, highlights[1].Rects.Head.Width)

    [<Fact>]
    member _.``search geometry preserves utf16 offsets after emoji and combining text``() =
        let measurer =
            TextMeasurer.create (
                TextMetrics.createWithGraphemeAdvance 16.0f 8.0f 4 (fun _ grapheme ->
                    if grapheme = "😀" || grapheme = "e\u0301" then
                        16.0f
                    else
                        8.0f)
            )

        let document: SearchDocument =
            { Id = System.Guid.NewGuid()
              Path = None
              Name = "untitled"
              Lines = [ "😀term"; "e\u0301term" ] }

        let matches = SearchEngine.findInDocument (SearchOptions.create "term") document

        let lines =
            LayoutEngine.layoutLines measurer 0 (document.Lines |> List.mapi (fun index text -> index, text))

        let highlights =
            LayoutEngine.layoutSearchHighlights measurer 0 lines (matches |> List.map (fun value -> value, false))

        Assert.Equal(2, highlights.Length)
        Assert.Equal(2, highlights[0].Range.Start.Column)
        Assert.Equal(16.0f, highlights[0].Rects.Head.X)
        Assert.Equal(2, highlights[1].Range.Start.Column)
        Assert.Equal(16.0f, highlights[1].Rects.Head.X)

    [<Fact>]
    member _.``search slicing excludes matches outside the visible line range``() =
        let document: SearchDocument =
            { Id = System.Guid.NewGuid()
              Path = None
              Name = "untitled"
              Lines = [ "term"; "other"; "term" ] }

        let matches = SearchEngine.findInDocument (SearchOptions.create "term") document

        let input =
            { Buffer = document.Lines
              View =
                { Viewport = { Width = 80; Height = 16 }
                  VerticalOffset = 2
                  HorizontalOffset = 0 }
              Editing = EditingModel.create ()
              Syntax = SyntaxModel.empty
              Diagnostics = DiagnosticsModel.create ()
              Search =
                { SearchModel.create () with
                    Matches = matches } }

        let sliced = SlicingEngine.sliceSearchMatches 1 input

        Assert.Single(sliced) |> ignore
        Assert.Equal(2, sliced.Head |> fst |> (fun matchValue -> matchValue.Line))

    [<Fact>]
    member _.``render input excludes workspace matches from other documents``() =
        let activeDocument =
            { DocumentModel.createUntitled "active.fs" with
                InitialText = "active term" }

        let otherDocumentId = System.Guid.NewGuid()

        let activeMatch =
            { DocumentId = activeDocument.Id
              Path = None
              Name = "active.fs"
              Line = 0
              Column = 7
              Length = 4
              Range =
                { Start = { Line = 0; Column = 7 }
                  End = { Line = 0; Column = 11 } }
              Preview = "active term" }

        let otherMatch =
            { activeMatch with
                DocumentId = otherDocumentId
                Name = "other.fs" }

        let model =
            { CoreModel.create () with
                ActiveDocument = Some activeDocument
                Editing =
                    { EditingModel.create () with
                        Buffer = [ "active term" ] }
                Navigation =
                    { NavigationModel.create () with
                        Search =
                            { SearchModel.create () with
                                Matches = [ otherMatch; activeMatch ]
                                Index = Some 1 } } }

        let input = RenderInput.fromCoreModel model

        Assert.Equal<SearchMatch list>([ activeMatch ], input.Search.Matches)
        Assert.Equal(Some 0, input.Search.Index)

    [<Fact>]
    member _.``search geometry follows horizontal scrolling``() =
        let measurer = createMeasurer ()

        let document: SearchDocument =
            { Id = System.Guid.NewGuid()
              Path = None
              Name = "untitled"
              Lines = [ "prefix term" ] }

        let matchValue =
            SearchEngine.findInDocument (SearchOptions.create "term") document
            |> List.exactlyOne

        let lines = LayoutEngine.layoutLines measurer 7 [ 0, document.Lines.Head ]

        let highlight =
            LayoutEngine.layoutSearchHighlights measurer 7 lines [ matchValue, false ]
            |> List.exactlyOne
            |> fun layout -> layout.Rects |> List.exactlyOne

        Assert.Equal(0.0f, highlight.X)
        Assert.Equal(32.0f, highlight.Width)

    [<Fact>]
    member _.``empty search results produce no highlight geometry``() =
        let measurer = createMeasurer ()
        let lines = LayoutEngine.layoutLines measurer 0 [ 0, "visible text" ]

        let highlights = LayoutEngine.layoutSearchHighlights measurer 0 lines []

        Assert.Empty(highlights)

    [<Fact>]
    member _.``diagnostics produce glyphs and multiline underlines``() =
        let measurer = createMeasurer ()
        let lines = LayoutEngine.layoutLines measurer 0 [ 0, "first"; 1, "second" ]

        let diagnostic =
            { Severity = DiagnosticSeverity.Error
              Message = "problem"
              RangeStart = { Line = 0; Column = 2 }
              RangeEnd = { Line = 1; Column = 3 }
              Code = None
              Source = None }

        let layout =
            LayoutEngine.layoutDiagnostics measurer lines [ diagnostic ] |> List.exactlyOne

        Assert.True(layout.Glyph.IsSome)
        Assert.Equal(2, layout.Underline.Length)
        Assert.Equal(16.0f, layout.Underline.[0].X)
        Assert.Equal(24.0f, layout.Underline.[0].Width)
        Assert.Equal(24.0f, layout.Underline.[1].Width)

    [<Fact>]
    member _.``line numbers use document indexes and visible y positions``() =
        let measurer = createMeasurer ()
        let lines = LayoutEngine.layoutLines measurer 0 [ 4, "fifth"; 5, "sixth" ]
        let numbers = LayoutEngine.layoutLineNumbers measurer lines

        Assert.Equal(2, numbers.Length)
        Assert.Equal("5", numbers.[0].Text)
        Assert.Equal(0.0f, numbers.[0].Y)
        Assert.Equal("6", numbers.[1].Text)
        Assert.Equal(16.0f, numbers.[1].Y)

    [<Fact>]
    member _.``gutter-aware layout keeps content to the right of line numbers``() =
        let measurer = createMeasurer ()
        let gutter = LayoutEngine.gutterWidth measurer 120
        let lines = LayoutEngine.layoutLinesWithGutter measurer gutter 0 [ 0, "first" ]
        let numbers = LayoutEngine.layoutLineNumbersWithGutter measurer gutter lines

        Assert.True(lines.[0].X >= gutter)
        Assert.True(numbers.[0].X < lines.[0].X)
        Assert.Equal(gutter - 4.0f, numbers.[0].X + measurer.MeasureRange numbers.[0].Text 0 numbers.[0].Text.Length)

    [<Fact>]
    member _.``gutter width remains stable for multi-digit document positions``() =
        let measurer = createMeasurer ()
        let gutter = LayoutEngine.gutterWidth measurer 120

        let lines =
            LayoutEngine.layoutLinesWithGutter measurer gutter 0 [ 8, "ninth"; 99, "hundredth" ]

        let numbers = LayoutEngine.layoutLineNumbersWithGutter measurer gutter lines

        Assert.True(numbers.[0].X > numbers.[1].X)
        Assert.Equal(gutter - 4.0f, numbers.[0].X + measurer.MeasureRange numbers.[0].Text 0 numbers.[0].Text.Length)
        Assert.Equal(gutter - 4.0f, numbers.[1].X + measurer.MeasureRange numbers.[1].Text 0 numbers.[1].Text.Length)
