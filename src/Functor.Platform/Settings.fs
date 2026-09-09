namespace Functor.Platform

open System
open System.IO

module Settings =
    let applicationDirectory =
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "Functor")

    let themeFilePath = Path.Combine(applicationDirectory, "settings.json")

    let tryReadText path =
        try
            if File.Exists(path) then
                Ok(File.ReadAllText(path))
            else
                Error(sprintf "Settings file does not exist: %s" path)
        with ex ->
            Error ex.Message

    let tryWriteText (path: string) (contents: string) =
        try
            let directory = Path.GetDirectoryName(path)

            if not (String.IsNullOrWhiteSpace directory) then
                Directory.CreateDirectory(directory) |> ignore

            File.WriteAllText(path, contents)
            Ok()
        with ex ->
            Error ex.Message
