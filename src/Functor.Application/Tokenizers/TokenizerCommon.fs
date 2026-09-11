namespace Functor.Application

open System
open Functor.Domain.Syntax

module internal TokenizerCommon =
    let output tokens snapshots finalState : TokenProviderOutput =
        { Provider = LocalLexical
          Layer = Lexical
          Tokens = tokens
          Snapshots = snapshots
          FinalState = finalState }

    let addToken (tokens: ResizeArray<Token>) lineIndex kind column length =
        if length > 0 then
            tokens.Add(
                { Kind = kind
                  Line = lineIndex
                  Column = column
                  Length = length })

    let scanQuotedString (text: string) (column: int) =
        let quote = text.[column]
        let mutable current = column + 1
        let mutable terminated = false

        while current < text.Length && not terminated do
            if text.[current] = '\\' then
                current <- min text.Length (current + 2)
            elif text.[current] = quote then
                current <- current + 1
                terminated <- true
            else
                current <- current + 1

        current

    let tryFind (value: string) (start: int) (needle: string) =
        let index = value.IndexOf(needle, start, StringComparison.Ordinal)

        if index >= 0 then Some index else None

    let scopedLines scope lines =
        let lineInScope line =
            match scope with
            | FullDocument -> true
            | Line target -> line = target
            | LineRange(startLine, endLine) -> line >= startLine && line <= endLine

        lines
        |> List.mapi (fun line text -> line, text)
        |> List.filter (fun (line, _) -> lineInScope line)