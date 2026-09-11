namespace Functor.Rendering

open System
open System.Globalization
open System.Text

type TextMetrics =
    { LineHeight: float32
      DefaultAdvance: float32
      TabWidth: int
      GraphemeAdvance: int -> string -> float32 }

module TextMetrics =
    let private isZeroWidthRune (rune: Rune) =
        match Rune.GetUnicodeCategory(rune) with
        | UnicodeCategory.NonSpacingMark
        | UnicodeCategory.SpacingCombiningMark
        | UnicodeCategory.EnclosingMark
        | UnicodeCategory.Format -> true
        | _ -> false

    let private defaultGraphemeAdvance tabWidth defaultAdvance column (grapheme: string) =
        let runes = grapheme.EnumerateRunes() |> Seq.toList

        if runes |> List.exists (fun rune -> rune.Value = int '\t') then
            float32 (tabWidth - (column % tabWidth)) * defaultAdvance
        elif not runes.IsEmpty && runes |> List.forall isZeroWidthRune then
            0.0f
        else
            defaultAdvance

    let createWithGraphemeAdvance lineHeight defaultAdvance tabWidth graphemeAdvance =
        let tabWidth = max 1 tabWidth

        { LineHeight = lineHeight
          DefaultAdvance = defaultAdvance
          TabWidth = tabWidth
          GraphemeAdvance = graphemeAdvance }

    let create lineHeight defaultAdvance tabWidth =
        createWithGraphemeAdvance
            lineHeight
            defaultAdvance
            tabWidth
            (defaultGraphemeAdvance (max 1 tabWidth) defaultAdvance)
