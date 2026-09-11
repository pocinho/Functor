namespace Functor.Rendering

open System
open System.Globalization
open System.Text

type TextMeasurer =
    { Metrics: TextMetrics; MeasureText: (string -> float32); MeasureRange: (string -> int -> int -> float32); MeasurePrefix: (string -> float32 array); HitTestColumn: (string -> float32 -> int) }

module TextMeasurer =
    let private graphemeStarts (text: string) =
        if String.IsNullOrEmpty(text) then
            [| 0 |]
        else
            StringInfo.ParseCombiningCharacters(text)
            |> Array.append [| 0; text.Length |]
            |> Array.distinct
            |> Array.sort
            |> Array.filter (fun boundary ->
                boundary = 0
                || boundary = text.Length
                || not (Char.IsLowSurrogate text.[boundary] && Char.IsHighSurrogate text.[boundary - 1]))

    let private graphemeRanges (text: string) =
        let starts = graphemeStarts text

        if starts.Length <= 1 then
            [| 0, text.Length |]
        else
            starts.[.. starts.Length - 2]
            |> Array.mapi (fun index start -> start, starts.[index + 1])

    let normalizeColumn (text: string) column =
        let column = max 0 (min text.Length column)

        graphemeStarts text
        |> Array.rev
        |> Array.tryFind (fun boundary -> boundary <= column)
        |> Option.defaultValue 0

    let normalizeRange (text: string) start length =
        let start = max 0 (min text.Length start)
        let finish = max start (min text.Length (start + max 0 length))
        let normalizedStart = normalizeColumn text start

        let normalizedFinish =
            if length <= 0 then
                normalizedStart
            else
                graphemeStarts text
                |> Array.tryFind (fun boundary -> boundary >= finish)
                |> Option.defaultValue text.Length

        normalizedStart, max normalizedStart normalizedFinish

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
        metrics.GraphemeAdvance column grapheme

    let create (metrics: TextMetrics) =
        let measureRange (text: string) start length =
            let start, finish = normalizeRange text start length
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

        let measurePrefix (text: string) =
            let widths = Array.zeroCreate<float32> (text.Length + 1)
            let mutable column = 0
            let mutable width = 0.0f

            for rangeStart, rangeEnd in graphemeRanges text do
                let grapheme = text.Substring(rangeStart, rangeEnd - rangeStart)
                let advance = graphemeAdvance metrics column grapheme

                for offset in rangeStart .. rangeEnd do
                    widths.[offset] <- width

                width <- width + advance
                widths.[rangeEnd] <- width
                column <- column + graphemeColumnWidth metrics column grapheme

            widths

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
          MeasureRange = measureRange;
                    MeasurePrefix = measurePrefix
          HitTestColumn = hitTestColumn }
