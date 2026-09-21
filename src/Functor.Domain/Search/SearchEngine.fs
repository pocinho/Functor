namespace Functor.Domain.Search

open System
open Functor.Domain.Editing

module SearchEngine =
    let private comparison caseSensitive =
        if caseSensitive then
            StringComparison.Ordinal
        else
            StringComparison.OrdinalIgnoreCase

    let private isValidUtf16Range (line: string) startColumn length =
        let endColumn = startColumn + length

        let startsInsideSurrogatePair =
            startColumn > 0
            && startColumn < line.Length
            && Char.IsLowSurrogate(line[startColumn])

        let endsInsideSurrogatePair =
            endColumn > 0 && endColumn < line.Length && Char.IsLowSurrogate(line[endColumn])

        not startsInsideSurrogatePair && not endsInsideSurrogatePair

    let private preview (line: string) startColumn length =
        let context = 40
        let endColumn = startColumn + length
        let previewStart = max 0 (startColumn - context)
        let previewEnd = min line.Length (endColumn + context)
        let value = line.Substring(previewStart, previewEnd - previewStart)
        let prefix = if previewStart > 0 then "..." else ""
        let suffix = if previewEnd < line.Length then "..." else ""
        prefix + value + suffix

    let findInDocument (options: SearchOptions) (document: SearchDocument) : SearchMatch list =
        if String.IsNullOrEmpty options.Query then
            []
        else
            let stringComparison = comparison options.CaseSensitive

            document.Lines
            |> List.mapi (fun lineIndex line ->
                let rec findFrom startColumn matches =
                    let matchColumn = line.IndexOf(options.Query, startColumn, stringComparison)

                    if matchColumn < 0 then
                        List.rev matches
                    elif isValidUtf16Range line matchColumn options.Query.Length then
                        let matchRange: Selection =
                            { Start =
                                { Line = lineIndex
                                  Column = matchColumn }
                              End =
                                { Line = lineIndex
                                  Column = matchColumn + options.Query.Length } }

                        let result =
                            { DocumentId = document.Id
                              Path = document.Path
                              Name = document.Name
                              Line = lineIndex
                              Column = matchColumn
                              Length = options.Query.Length
                              Range = matchRange
                              Preview = preview line matchColumn options.Query.Length }

                        findFrom (matchColumn + max 1 options.Query.Length) (result :: matches)
                    else
                        findFrom (matchColumn + 1) matches

                findFrom 0 [])
            |> List.concat

    let findInDocuments (options: SearchOptions) (documents: SearchDocument list) =
        documents |> List.collect (findInDocument options)
