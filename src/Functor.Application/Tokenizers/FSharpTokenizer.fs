namespace Functor.Application

open System
open System.Threading
open Functor.Domain.Syntax

module internal FSharpTokenizer =
    let private keywords =
        Set.ofList
            [ "abstract"
              "and"
              "as"
              "assert"
              "begin"
              "class"
              "default"
              "delegate"
              "do"
              "done"
              "downcast"
              "downto"
              "else"
              "exception"
              "extern"
              "false"
              "finally"
              "for"
              "fun"
              "function"
              "if"
              "in"
              "inherit"
              "inline"
              "interface"
              "internal"
              "lazy"
              "let"
              "match"
              "member"
              "module"
              "mutable"
              "namespace"
              "new"
              "null"
              "of"
              "open"
              "or"
              "override"
              "private"
              "public"
              "rec"
              "return"
              "select"
              "static"
              "struct"
              "then"
              "this"
              "throw"
              "to"
              "true"
              "try"
              "type"
              "upcast"
              "use"
              "val"
              "void"
              "when"
              "while"
              "with"
              "yield" ]

    let private isIdentifierStart character =
        Char.IsLetter character || character = '_' || character = '\''

    let private isIdentifierPart character =
        isIdentifierStart character || Char.IsDigit character

    let private isNumberPart character =
        Char.IsLetterOrDigit character || character = '_' || character = '.'

    let private isOperatorCharacter (character: char) =
        "!$%&*+-./:<=>?@^|~".IndexOf(character) >= 0

    let tokenize (request: TokenizationRequest) (cancellationToken: CancellationToken) =
        let results = ResizeArray<LineTokens>()
        let snapshots = ResizeArray<LexerSnapshot>()
        let mutable blockCommentDepth =
            match request.InitialState with
            | FSharpState depth -> depth
            | _ -> 0

        let scanLine lineIndex (text: string) =
            let tokens = ResizeArray<Token>()
            let mutable column = 0

            while column < text.Length do
                cancellationToken.ThrowIfCancellationRequested()

                if blockCommentDepth > 0 then
                    let start = column

                    while column < text.Length && blockCommentDepth > 0 do
                        if column + 1 < text.Length && text.[column] = '(' && text.[column + 1] = '*' then
                            blockCommentDepth <- blockCommentDepth + 1
                            column <- column + 2
                        elif column + 1 < text.Length && text.[column] = '*' && text.[column + 1] = ')' then
                            blockCommentDepth <- blockCommentDepth - 1
                            column <- column + 2
                        else
                            column <- column + 1

                    TokenizerCommon.addToken tokens lineIndex "comment" start (column - start)
                elif column + 1 < text.Length && text.[column] = '/' && text.[column + 1] = '/' then
                    TokenizerCommon.addToken tokens lineIndex "comment" column (text.Length - column)
                    column <- text.Length
                elif column + 1 < text.Length && text.[column] = '(' && text.[column + 1] = '*' then
                    let start = column
                    blockCommentDepth <- 1
                    column <- column + 2

                    while column < text.Length && blockCommentDepth > 0 do
                        if column + 1 < text.Length && text.[column] = '(' && text.[column + 1] = '*' then
                            blockCommentDepth <- blockCommentDepth + 1
                            column <- column + 2
                        elif column + 1 < text.Length && text.[column] = '*' && text.[column + 1] = ')' then
                            blockCommentDepth <- blockCommentDepth - 1
                            column <- column + 2
                        else
                            column <- column + 1

                    TokenizerCommon.addToken tokens lineIndex "comment" start (column - start)
                elif text.[column] = '"' then
                    let start = column
                    column <- column + 1
                    let mutable terminated = false

                    while column < text.Length && not terminated do
                        if text.[column] = '\\' then
                            column <- min text.Length (column + 2)
                        elif text.[column] = '"' then
                            column <- column + 1
                            while column < text.Length && text.[column] = '"' do
                                column <- column + 1
                            terminated <- true
                        else
                            column <- column + 1

                    TokenizerCommon.addToken tokens lineIndex "string" start (column - start)
                elif text.[column] = '\'' && column + 2 < text.Length && text.[column + 2] = '\'' then
                    let start = column
                    column <- min text.Length (column + 3)
                    TokenizerCommon.addToken tokens lineIndex "string" start (column - start)
                elif Char.IsWhiteSpace(text.[column]) then
                    column <- column + 1
                elif text.[column] = '#' && (column = 0 || text.[0..column - 1].Trim().Length = 0) then
                    TokenizerCommon.addToken tokens lineIndex "directive" column (text.Length - column)
                    column <- text.Length
                elif isIdentifierStart text.[column] then
                    let start = column
                    column <- column + 1

                    while column < text.Length && isIdentifierPart text.[column] do
                        column <- column + 1

                    let value = text.Substring(start, column - start)
                    let kind =
                        if keywords.Contains value then
                            "keyword"
                        elif Char.IsUpper(value.[0]) then
                            "type"
                        else
                            "identifier"

                    TokenizerCommon.addToken tokens lineIndex kind start (column - start)
                elif Char.IsDigit(text.[column]) then
                    let start = column
                    column <- column + 1

                    while column < text.Length && isNumberPart text.[column] do
                        column <- column + 1

                    TokenizerCommon.addToken tokens lineIndex "number" start (column - start)
                elif isOperatorCharacter text.[column] then
                    let start = column
                    column <- column + 1

                    while column < text.Length && isOperatorCharacter text.[column] do
                        column <- column + 1

                    TokenizerCommon.addToken tokens lineIndex "operator" start (column - start)
                else
                    TokenizerCommon.addToken tokens lineIndex "punctuation" column 1
                    column <- column + 1

            { Line = lineIndex
              Tokens = List.ofSeq tokens }

        for lineIndex, line in TokenizerCommon.scopedLines request.Scope request.Lines do
            cancellationToken.ThrowIfCancellationRequested()
            snapshots.Add({ Line = lineIndex; State = FSharpState blockCommentDepth })
            results.Add(scanLine lineIndex line)

        Ok(TokenizerCommon.output (List.ofSeq results) (List.ofSeq snapshots) (FSharpState blockCommentDepth))