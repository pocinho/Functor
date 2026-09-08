namespace Functor.Tests.Rendering

open Functor.Domain.Diagnostics
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Rendering
open Xunit
open TestFixtures

type LayoutEngineTests() =
    [<Fact>]
    member _.``vertical offset is bounded by content``() =
        Assert.Equal(90, LayoutEngine.maxVerticalOffset 10 (List.replicate 100 "line"))
        Assert.Equal(0, LayoutEngine.maxVerticalOffset 10 (List.replicate 5 "line"))

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
        Assert.Equal("keyword", layout.Style.Kind)
        Assert.Equal(0, layout.Range.Start.Column)
        Assert.Equal(3, layout.Range.End.Column)
        Assert.Equal(-8.0f, layout.XStart)
        Assert.Equal(16.0f, layout.XEnd)

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
