namespace Functor.Tests.Rendering

open Functor.Domain.Diagnostics
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Rendering
open Functor.Domain.Search
open Xunit
open TestFixtures

type RenderingLayerTests() =
    [<Fact>]
    member _.``highlight geometry and hit testing share horizontal scroll coordinates``() =
        let measurer = createMeasurer ()
        let text = "prefix term"
        let lines = LayoutEngine.layoutLines measurer 7 [ 0, text ]

        let document: SearchDocument =
            { Id = System.Guid.NewGuid()
              Path = None
              Name = "untitled"
              Lines = [ text ] }

        let matchValue =
            SearchEngine.findInDocument (SearchOptions.create "term") document
            |> List.exactlyOne

        let highlight =
            LayoutEngine.layoutSearchHighlights measurer 7 lines [ matchValue, false ]
            |> List.exactlyOne
            |> fun layout -> layout.Rects |> List.exactlyOne

        let position = LayoutEngine.positionAtPoint measurer 7 0 [ text ] highlight.X 0.0f

        Assert.Equal(0.0f, highlight.X)
        Assert.Equal({ Line = 0; Column = 7 }, position)

    [<Fact>]
    member _.``overlapping rendering layers retain independent geometry``() =
        let measurer = createMeasurer ()
        let lines = LayoutEngine.layoutLines measurer 0 [ 0, "term" ]

        let range: Selection =
            { Start = { Line = 0; Column = 0 }
              End = { Line = 0; Column = 4 } }

        let renderingRange: Functor.Rendering.Range =
            { Start = range.Start; End = range.End }

        let matchValue =
            { DocumentId = System.Guid.NewGuid()
              Path = None
              Name = "untitled"
              Line = 0
              Column = 0
              Length = 4
              Range = range
              Preview = "term" }

        let token =
            { Kind = "keyword"
              Line = 0
              Column = 0
              Length = 4 }

        let selection = LayoutEngine.layoutSelections measurer 0 lines [ renderingRange ]

        let highlights =
            LayoutEngine.layoutSearchHighlights measurer 0 lines [ matchValue, true ]

        let tokens = LayoutEngine.layoutTokens measurer lines [ token ]

        let diagnostic =
            { Severity = DiagnosticSeverity.Error
              Message = "term"
              RangeStart = range.Start
              RangeEnd = range.End
              Code = None
              Source = None }

        let diagnostics = LayoutEngine.layoutDiagnostics measurer lines [ diagnostic ]

        let cursor =
            LayoutEngine.layoutCursors measurer 0 lines [ { Line = 0; Column = 4 } ]

        Assert.Single(selection) |> ignore
        Assert.Single(highlights) |> ignore
        Assert.Single(tokens) |> ignore
        Assert.Single(diagnostics) |> ignore
        Assert.Single(cursor) |> ignore
        Assert.True(highlights.Head.IsActive)
        Assert.Equal(range.Start, selection.Head.Range.Start)
        Assert.Equal(range.End, selection.Head.Range.End)
        Assert.Equal(range.Start, highlights.Head.Range.Start)
        Assert.Equal(range.End, highlights.Head.Range.End)
        Assert.Equal(range.Start, tokens.Head.Range.Start)
        Assert.Equal(range.End, tokens.Head.Range.End)
        Assert.Equal(range.Start, diagnostics.Head.Range.Start)
        Assert.Equal(range.End, diagnostics.Head.Range.End)
