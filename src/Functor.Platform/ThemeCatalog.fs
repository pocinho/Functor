namespace Functor.Platform

open System
open System.IO
open Functor.Application

type ThemeFile = { Name: string; Settings: AppSettings }

module ThemeCatalog =
    let themesDirectory = Path.Combine(Settings.applicationDirectory, "themes")
    let private extension = ".functortheme"

    let private invalidName name =
        String.IsNullOrWhiteSpace name
        || name.IndexOfAny(Path.GetInvalidFileNameChars()) >= 0
        || name = "."
        || name = ".."

    let loadFromDirectory directory =
        try
            if not (Directory.Exists(directory)) then
                []
            else
                Directory.EnumerateFiles(directory, "*", SearchOption.TopDirectoryOnly)
                |> Seq.filter (fun path ->
                    String.Equals(Path.GetExtension(path), extension, StringComparison.OrdinalIgnoreCase))
                |> Seq.choose (fun path ->
                    match ThemeSettingsLoader.loadFile path with
                    | Ok settings ->
                        let name = Path.GetFileNameWithoutExtension(path)

                        if ThemePreset.all |> List.contains name then
                            None
                        else
                            Some
                                { Name = name
                                  Settings = AppSettings.fromTheme settings }
                    | Error _ -> None)
                |> Seq.sortBy (fun theme -> theme.Name.ToUpperInvariant())
                |> Seq.toList
        with _ ->
            []

    let load () = loadFromDirectory themesDirectory

    let private nameTaken directory (name: string) =
        ThemePreset.all
        |> List.exists (fun preset -> String.Equals(preset, name, StringComparison.OrdinalIgnoreCase))
        || (loadFromDirectory directory
            |> List.exists (fun theme -> String.Equals(theme.Name, name, StringComparison.OrdinalIgnoreCase)))
        || File.Exists(Path.Combine(directory, name + extension))

    let exportToDirectory directory (name: string) (settings: AppSettings) =
        let normalizedName = if isNull name then "" else name.Trim()

        if invalidName normalizedName then
            Error "Theme name must be a non-empty valid file name."
        elif nameTaken directory normalizedName then
            Error(sprintf "A theme named '%s' already exists." normalizedName)
        else
            let path = Path.Combine(directory, normalizedName + extension)
            Settings.tryWriteText path (AppSettingsLoader.toJson settings)

    let export (name: string) (settings: AppSettings) =
        exportToDirectory themesDirectory name settings
