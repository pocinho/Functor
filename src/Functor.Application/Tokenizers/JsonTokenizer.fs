namespace Functor.Application

open System
open System.Threading
open Functor.Domain.Syntax

module internal JsonTokenizer =
    let private literals = Set.ofList [ "true"; "false"; "null" ]

    let private isNumberStart character =
        Char.IsDigit character || character = '-'

    let private isNumberPart character =
        Char.IsDigit character || character = '.' || character = '-' || character = '+' || character = 'e' || character = 'E'

    let tokenize (request: TokenizationRequest) (cancellationToken: CancellationToken) =
        let results = ResizeArray<LineTokens>()
        let snapshots = ResizeArray<LexerSnapshot>()

        let scanLine lineIndex (text: string) =
            let tokens = ResizeArray<Token>()
            let mutable column = 0

            while column < text.Length do
                cancellationToken.ThrowIfCancellationRequested()

                if Char.IsWhiteSpace(text.[column]) then
                    column <- column + 1
                elif text.[column] = '"' then
                    let start = column
                    column <- TokenizerCommon.scanQuotedString text column
                    let mutable lookahead = column

                    while lookahead < text.Length && Char.IsWhiteSpace(text.[lookahead]) do
                        lookahead <- lookahead + 1

                    let kind =
                        if lookahead < text.Length && text.[lookahead] = ':' then
                            "property"
                        else
                            "string"

                    TokenizerCommon.addToken tokens lineIndex kind start (column - start)
                elif isNumberStart text.[column] then
                    let start = column
                    column <- column + 1

                    while column < text.Length && isNumberPart text.[column] do
                        column <- column + 1

                    TokenizerCommon.addToken tokens lineIndex "number" start (column - start)
                elif Char.IsLetter text.[column] then
                    let start = column
                    column <- column + 1

                    while column < text.Length && Char.IsLetter text.[column] do
                        column <- column + 1

                    let value = text.Substring(start, column - start)
                    let kind = if literals.Contains value then "keyword" else "identifier"
                    TokenizerCommon.addToken tokens lineIndex kind start (column - start)
                else
                    TokenizerCommon.addToken tokens lineIndex "punctuation" column 1
                    column <- column + 1

            { Line = lineIndex
              Tokens = List.ofSeq tokens }

        for lineIndex, line in TokenizerCommon.scopedLines request.Scope request.Lines do
            cancellationToken.ThrowIfCancellationRequested()
            snapshots.Add({ Line = lineIndex; State = Initial })
            results.Add(scanLine lineIndex line)

        Ok(TokenizerCommon.output (List.ofSeq results) (List.ofSeq snapshots) Initial)