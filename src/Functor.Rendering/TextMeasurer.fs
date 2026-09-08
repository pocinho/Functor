namespace Functor.Rendering

open System
open System.Globalization
open System.Text

type TextMeasurer =
    { Metrics: TextMetrics
      MeasureText: string -> float32
      MeasureRange: string -> int -> int -> float32
      HitTestColumn: string -> float32 -> int }

module TextMeasurer =
    let private graphemeStarts (text: string) =
        if String.IsNullOrEmpty(text) then
            [||]
        else
            StringInfo.ParseCombiningCharacters(text)

    let private graphemeRanges (text: string) =
        let starts = graphemeStarts text

        starts
        |> Array.mapi (fun index start ->
            let finish =
                if index + 1 < starts.Length then
                    starts.[index + 1]
                else
                    text.Length

            start, finish)

    let private isZeroWidthRune (rune: Rune) =
        match Rune.GetUnicodeCategory(rune) with
        | UnicodeCategory.NonSpacingMark
        | UnicodeCategory.SpacingCombiningMark
        | UnicodeCategory.EnclosingMark
        | UnicodeCategory.Format -> true
        | _ -> false

    let private graphemeColumnWidth (metrics: TextMetrics) (column: int) (grapheme: string) =
        let runes = grapheme.EnumerateRunes() |> Seq.toList

        if runes |> List.exists (fun rune -> rune.Value = int '\t') then
            metrics.TabWidth - (column % metrics.TabWidth)
        elif not runes.IsEmpty && runes |> List.forall isZeroWidthRune then
            0
        else
            1

    let private graphemeAdvance (metrics: TextMetrics) (column: int) (grapheme: string) =
        float32 (graphemeColumnWidth metrics column grapheme) * metrics.DefaultAdvance

    let create (metrics: TextMetrics) =
        let measureRange (text: string) start length =
            let start = max 0 (min text.Length start)
            let finish = max start (min text.Length (start + max 0 length))
            let mutable column = 0
            let mutable width = 0.0f

            for rangeStart, rangeEnd in graphemeRanges text do
                if rangeStart >= start && rangeEnd <= finish then
                    let grapheme = text.Substring(rangeStart, rangeEnd - rangeStart)
                    width <- width + graphemeAdvance metrics column grapheme
                    column <- column + graphemeColumnWidth metrics column grapheme
                elif rangeEnd <= start then
                    let grapheme = text.Substring(rangeStart, rangeEnd - rangeStart)
                    column <- column + graphemeColumnWidth metrics column grapheme

            width

        let measureText text = measureRange text 0 text.Length

        let hitTestColumn (text: string) (x: float32) =
            if x <= 0.0f then
                0
            else
                let mutable column = 0
                let mutable position = text.Length
                let mutable consumed = 0.0f

                for rangeStart, rangeEnd in graphemeRanges text do
                    let grapheme = text.Substring(rangeStart, rangeEnd - rangeStart)
                    let advance = graphemeAdvance metrics column grapheme

                    if position = text.Length && x < consumed + advance / 2.0f then
                        position <- rangeStart

                    consumed <- consumed + advance
                    column <- column + graphemeColumnWidth metrics column grapheme

                position

        { Metrics = metrics
          MeasureText = measureText
          MeasureRange = measureRange
          HitTestColumn = hitTestColumn }
