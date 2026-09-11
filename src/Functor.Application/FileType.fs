namespace Functor.Application

open System.IO

module FileType =
    type private FileTypeDefinition =
        { Extension: string
          DisplayName: string
          LanguageId: string option }

    let private knownTypes =
        [ { Extension = ".fs"; DisplayName = "F#"; LanguageId = Some "fsharp" }
          { Extension = ".fsi"; DisplayName = "F# Signature"; LanguageId = Some "fsharp" }
          { Extension = ".fsx"; DisplayName = "F# Script"; LanguageId = Some "fsharp" }
          { Extension = ".cs"; DisplayName = "C#"; LanguageId = Some "csharp" }
          { Extension = ".json"; DisplayName = "JSON"; LanguageId = Some "json" }
          { Extension = ".md"; DisplayName = "Markdown"; LanguageId = Some "markdown" }
          { Extension = ".markdown"; DisplayName = "Markdown"; LanguageId = Some "markdown" }
          { Extension = ".txt"; DisplayName = "Plain Text"; LanguageId = None }
          { Extension = ".xml"; DisplayName = "XML"; LanguageId = None }
          { Extension = ".yaml"; DisplayName = "YAML"; LanguageId = None }
          { Extension = ".yml"; DisplayName = "YAML"; LanguageId = None }
          { Extension = ".toml"; DisplayName = "TOML"; LanguageId = None } ]

    let private findDefinition path =
        match path with
        | Some (value: string) ->
            let extension = Path.GetExtension(value).ToLowerInvariant()
            knownTypes |> List.tryFind (fun definition -> definition.Extension = extension)
        | None ->
            None

    let fromPath path =
        findDefinition path
        |> Option.map (fun definition -> definition.DisplayName)
        |> Option.defaultValue "Plain Text"

    let languageId path =
        findDefinition path
        |> Option.bind (fun definition -> definition.LanguageId)
