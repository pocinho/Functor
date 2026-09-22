namespace Functor.Application

open System
open Functor.Domain.Search

module WorkspaceReplacement =
    let private lineStart (source: string) line =
        if line < 0 then
            None
        else
            let mutable currentLine = 0
            let mutable index = 0
            let mutable result = if line = 0 then Some 0 else None

            while index < source.Length && result.IsNone do
                if source[index] = '\n' then
                    currentLine <- currentLine + 1

                    if currentLine = line then
                        result <- Some(index + 1)

                index <- index + 1

            result

    let private replacementOffset source (matchValue: SearchMatch) =
        lineStart source matchValue.Line
        |> Option.map (fun start -> start + matchValue.Column)

    let apply replacement (sources: Map<string, string>) (matches: SearchMatch list) =
        let grouped = matches |> List.groupBy (fun matchValue -> matchValue.Path)

        let errors = ResizeArray<string>()
        let replacements = ResizeArray<string * string>()

        for path, pathMatches in grouped do
            match path with
            | None -> errors.Add("Search replacement requires a file path.")
            | Some value ->
                let canonicalPath = Functor.Domain.Document.DocumentModel.canonicalizePath value

                match sources.TryFind canonicalPath with
                | None -> errors.Add(sprintf "No source snapshot is available for %s." canonicalPath)
                | Some source ->
                    let offsets =
                        pathMatches
                        |> List.choose (fun matchValue ->
                            matchValue
                            |> replacementOffset source
                            |> Option.map (fun offset -> offset, matchValue.Length))
                        |> List.sortByDescending fst

                    if offsets.Length <> pathMatches.Length then
                        errors.Add(sprintf "Search replacement contains an invalid line for %s." canonicalPath)
                    elif
                        offsets
                        |> List.exists (fun (offset, length) ->
                            offset < 0 || length < 0 || offset + length > source.Length)
                    then
                        errors.Add(sprintf "Search replacement contains an invalid range for %s." canonicalPath)
                    else
                        let mutable updated = source

                        for offset, length in offsets do
                            updated <- updated.Remove(offset, length).Insert(offset, replacement)

                        replacements.Add(canonicalPath, updated)

        if errors.Count > 0 then
            Error(List.ofSeq errors)
        else
            Ok(Map.ofSeq replacements)
