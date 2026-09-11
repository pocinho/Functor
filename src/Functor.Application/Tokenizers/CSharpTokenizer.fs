namespace Functor.Application

open System
open System.Threading
open Functor.Domain.Syntax

module internal CSharpTokenizer =
    let private keywords =
        Set.ofList
            [ "abstract"
              "as"
              "base"
              "bool"
              "break"
              "byte"
              "case"
              "catch"
              "char"
              "checked"
              "class"
              "const"
              "continue"
              "decimal"
              "default"
              "delegate"
              "do"
              "double"
              "else"
              "enum"
              "event"
              "explicit"
              "extern"
              "false"
              "finally"
              "fixed"
              "float"
              "for"
              "foreach"
              "goto"
              "if"
              "implicit"
              "in"
              "int"
              "interface"
              "internal"
              "is"
              "lock"
              "long"
              "namespace"
              "new"
              "null"
              "object"
              "operator"
              "out"
              "override"
              "params"
              "private"
              "protected"
              "public"
              "readonly"
              "record"
              "ref"
              "return"
              "sbyte"
              "sealed"
              "short"
              "sizeof"
              "stackalloc"
              "static"
              "string"
              "struct"
              "switch"
              "this"
              "throw"
              "true"
              "try"
              "typeof"
              "uint"
              "ulong"
              "unchecked"
              "unsafe"
              "ushort"
              "using"
              "virtual"
              "void"
              "volatile"
              "while"
              "var"
              "async"
              "await"
              "init"
              "required"
              "yield" ]

    let private typeDeclarationKeywords =
        Set.ofList [ "class"; "struct"; "interface"; "record"; "enum"; "delegate" ]

    let private isIdentifierStart character =
        Char.IsLetter character || character = '_' || character = '@'

    let private isIdentifierPart character =
        Char.IsLetterOrDigit character || character = '_'

    let private isNumberPart character =
        Char.IsLetterOrDigit character || character = '_' || character = '.'

    let private isOperatorCharacter (character: char) =
        "!%&*+-./:<=>?^|~".IndexOf(character) >= 0

    let tokenize (request: TokenizationRequest) (cancellationToken: CancellationToken) =
        let results = ResizeArray<LineTokens>()
        let snapshots = ResizeArray<LexerSnapshot>()
        let initialState =
            match request.InitialState with
            | CSharpState(blockComment, verbatimString, expectsType) -> blockComment, verbatimString, expectsType
            | _ -> false, false, false
        let initialBlockComment, initialVerbatimString, initialExpectsType = initialState
        let mutable inBlockComment = initialBlockComment
        let mutable inVerbatimString = initialVerbatimString
        let mutable expectsTypeDeclarationName = initialExpectsType

        let scanLine lineIndex (text: string) =
            let tokens = ResizeArray<Token>()
            let mutable column = 0

            while column < text.Length do
                cancellationToken.ThrowIfCancellationRequested()

                if inVerbatimString then
                    let start = column
                    column <- column + 1
                    let mutable terminated = false

                    while column < text.Length && not terminated do
                        if text.[column] = '"' && column + 1 < text.Length && text.[column + 1] = '"' then
                            column <- column + 2
                        elif text.[column] = '"' then
                            column <- column + 1
                            terminated <- true
                        else
                            column <- column + 1

                    inVerbatimString <- not terminated
                    TokenizerCommon.addToken tokens lineIndex "string" start (column - start)
                elif inBlockComment then
                    let start = column
                    let closeIndex = text.IndexOf("*/", column, StringComparison.Ordinal)

                    if closeIndex >= 0 then
                        column <- closeIndex + 2
                        inBlockComment <- false
                    else
                        column <- text.Length

                    TokenizerCommon.addToken tokens lineIndex "comment" start (column - start)
                elif column + 1 < text.Length && text.[column] = '/' && text.[column + 1] = '/' then
                    TokenizerCommon.addToken tokens lineIndex "comment" column (text.Length - column)
                    column <- text.Length
                elif column + 1 < text.Length && text.[column] = '/' && text.[column + 1] = '*' then
                    let start = column
                    let closeIndex = text.IndexOf("*/", column + 2, StringComparison.Ordinal)

                    if closeIndex >= 0 then
                        column <- closeIndex + 2
                    else
                        column <- text.Length
                        inBlockComment <- true

                    TokenizerCommon.addToken tokens lineIndex "comment" start (column - start)
                elif text.[column] = '@' && column + 1 < text.Length && text.[column + 1] = '"' then
                    let start = column
                    column <- column + 2
                    let mutable terminated = false

                    while column < text.Length && not terminated do
                        if text.[column] = '"' && column + 1 < text.Length && text.[column + 1] = '"' then
                            column <- column + 2
                        elif text.[column] = '"' then
                            column <- column + 1
                            terminated <- true
                        else
                            column <- column + 1

                            inVerbatimString <- not terminated
                    TokenizerCommon.addToken tokens lineIndex "string" start (column - start)
                elif text.[column] = '"' || text.[column] = '\'' then
                    let start = column
                    column <- TokenizerCommon.scanQuotedString text column
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

                    let value = text.Substring(start, column - start).TrimStart('@')
                    let kind =
                        if expectsTypeDeclarationName then
                            "type"
                        elif keywords.Contains value then
                            "keyword"
                        else
                            "identifier"

                    expectsTypeDeclarationName <- keywords.Contains value && typeDeclarationKeywords.Contains value

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
            snapshots.Add(
                { Line = lineIndex
                  State = CSharpState(inBlockComment, inVerbatimString, expectsTypeDeclarationName) })
            results.Add(scanLine lineIndex line)

        Ok(
            TokenizerCommon.output
                (List.ofSeq results)
                (List.ofSeq snapshots)
                (CSharpState(inBlockComment, inVerbatimString, expectsTypeDeclarationName))
        )