namespace Functor.Tests.Rendering

open Xunit
open TestFixtures

type TextMeasurerTests() =
    [<Fact>]
    member _.``grapheme clusters stay together while preserving utf16 offsets``() =
        let measurer = createMeasurer ()
        let text = "a😀e\u0301b"

        Assert.Equal(32.0f, measurer.MeasureText text)
        Assert.Equal(1, measurer.HitTestColumn text 9.0f)
        Assert.Equal(3, measurer.HitTestColumn text 17.0f)
        Assert.Equal(5, measurer.HitTestColumn text 25.0f)

    [<Fact>]
    member _.``tabs advance to the next configured tab stop``() =
        let measurer = createMeasurer ()

        Assert.Equal(40.0f, measurer.MeasureText "a\tb")
        Assert.Equal(24.0f, measurer.MeasureRange "a\tb" 1 1)
