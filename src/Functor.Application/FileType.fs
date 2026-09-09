namespace Functor.Application

open System.IO

module FileType =
    let private knownTypes =
        [ ".fs", "F#"
          ".fsi", "F# Signature"
          ".fsx", "F# Script"
          ".cs", "C#"
          ".json", "JSON"
          ".md", "Markdown"
          ".txt", "Plain Text"
          ".xml", "XML"
          ".yaml", "YAML"
          ".yml", "YAML"
          ".toml", "TOML" ]

    let fromPath path =
        match path with
        | None -> "Plain Text"
        | Some (value: string) ->
            let extension = Path.GetExtension(value).ToLowerInvariant()
            knownTypes
            |> List.tryFind (fun (knownExtension, _) -> knownExtension = extension)
            |> Option.map snd
            |> Option.defaultValue "Plain Text"
