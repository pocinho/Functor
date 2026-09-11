namespace Functor.Application

open System
open System.Threading
open Functor.Domain.Syntax

module internal MarkdownTokenizer =
    let tokenize (request: TokenizationRequest) (cancellationToken: CancellationToken) =
        let results = ResizeArray<LineTokens>()
        let snapshots = ResizeArray<LexerSnapshot>()
        let initialState =
            match request.InitialState with
            | MarkdownState(htmlComment, fenced) -> htmlComment, fenced
            | _ -> false, None
        let initialHtmlComment, initialFence = initialState
        let mutable inHtmlComment = initialHtmlComment
        let mutable fencedCodeMarker: string option = initialFence

        let scanLine lineIndex (text: string) =
            let tokens = ResizeArray<Token>()
            let mutable column = 0

            let mutable headingStart = 0

            while headingStart < text.Length && headingStart < 3 && text.[headingStart] = ' ' do
                headingStart <- headingStart + 1

            let mutable headingEnd = headingStart

            while headingEnd < text.Length && headingEnd - headingStart < 6 && text.[headingEnd] = '#' do
                headingEnd <- headingEnd + 1

            let trimmed = text.TrimStart()
            let isFence = trimmed.StartsWith("```") || trimmed.StartsWith("~~~")

            match fencedCodeMarker with
            | Some marker ->
                TokenizerCommon.addToken tokens lineIndex "code" 0 text.Length
                let closesFence = trimmed.StartsWith(marker) && trimmed.Substring(marker.Length).Trim().Length = 0

                if closesFence then
                    fencedCodeMarker <- None

                column <- text.Length
            | None when isFence ->
                let marker: string = trimmed.Substring(0, 3)
                let start = text.Length - trimmed.Length
                TokenizerCommon.addToken tokens lineIndex "code" start (text.Length - start)
                fencedCodeMarker <- Some marker
                column <- text.Length
            | None ->
                if headingEnd > headingStart && headingEnd < text.Length && Char.IsWhiteSpace text.[headingEnd] then
                    TokenizerCommon.addToken tokens lineIndex "heading" headingStart (headingEnd - headingStart)
                    column <- headingEnd + 1

            if fencedCodeMarker.IsSome then
                column <- text.Length

            while column < text.Length do
                cancellationToken.ThrowIfCancellationRequested()

                if inHtmlComment then
                    let start = column

                    match TokenizerCommon.tryFind text column "-->" with
                    | Some closeIndex ->
                        column <- closeIndex + 3
                        inHtmlComment <- false
                    | None ->
                        column <- text.Length

                    TokenizerCommon.addToken tokens lineIndex "comment" start (column - start)
                elif column + 3 < text.Length && text.Substring(column, 4) = "<!--" then
                    let start = column

                    match TokenizerCommon.tryFind text (column + 4) "-->" with
                    | Some closeIndex -> column <- closeIndex + 3
                    | None ->
                        column <- text.Length
                        inHtmlComment <- true

                    TokenizerCommon.addToken tokens lineIndex "comment" start (column - start)
                elif text.[column] = '`' then
                    let start = column
                    let closeIndex = text.IndexOf('`', column + 1)
                    column <- if closeIndex >= 0 then closeIndex + 1 else text.Length
                    TokenizerCommon.addToken tokens lineIndex "code" start (column - start)
                elif column + 1 < text.Length && (text.Substring(column, 2) = "**" || text.Substring(column, 2) = "__") then
                    let marker = text.Substring(column, 2)
                    let start = column

                    match TokenizerCommon.tryFind text (column + 2) marker with
                    | Some closeIndex -> column <- closeIndex + 2
                    | None -> column <- column + 2

                    TokenizerCommon.addToken tokens lineIndex "emphasis" start (column - start)
                elif text.[column] = '[' then
                    let start = column
                    let closeLabel = text.IndexOf(']', column + 1)

                    if closeLabel >= 0 && closeLabel + 1 < text.Length && text.[closeLabel + 1] = '(' then
                        let closeUrl = text.IndexOf(')', closeLabel + 2)

                        if closeUrl >= 0 then
                            column <- closeUrl + 1
                            TokenizerCommon.addToken tokens lineIndex "link" start (column - start)
                        else
                            TokenizerCommon.addToken tokens lineIndex "text" column 1
                            column <- column + 1
                    else
                        TokenizerCommon.addToken tokens lineIndex "text" column 1
                        column <- column + 1
                elif Char.IsWhiteSpace text.[column] then
                    column <- column + 1
                else
                    let start = column

                    while column < text.Length
                          && not (Char.IsWhiteSpace text.[column])
                          && text.[column] <> '`'
                          && text.[column] <> '['
                          && not (column + 1 < text.Length && (text.Substring(column, 2) = "**" || text.Substring(column, 2) = "__"))
                          && not (column + 3 < text.Length && text.Substring(column, 4) = "<!--") do
                        column <- column + 1

                    TokenizerCommon.addToken tokens lineIndex "text" start (column - start)

            { Line = lineIndex
              Tokens = List.ofSeq tokens }

        for lineIndex, line in TokenizerCommon.scopedLines request.Scope request.Lines do
            cancellationToken.ThrowIfCancellationRequested()
            snapshots.Add(
                { Line = lineIndex
                  State = MarkdownState(inHtmlComment, fencedCodeMarker) })
            results.Add(scanLine lineIndex line)

        Ok(
            TokenizerCommon.output
                (List.ofSeq results)
                (List.ofSeq snapshots)
                (MarkdownState(inHtmlComment, fencedCodeMarker))
        )