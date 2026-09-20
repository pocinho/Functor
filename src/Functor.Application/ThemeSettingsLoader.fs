namespace Functor.Application

open System
open System.Text.Json
open Functor.Rendering
open ThemeSettingsJson

module ThemeSettingsLoader =
    let private currentSchemaVersion = 1

    let loadText (json: string) : Result<ThemeSettings, string> =
        try
            use document = JsonDocument.Parse(json)
            let root = document.RootElement
            let mutable themeElement = Unchecked.defaultof<JsonElement>

            let theme =
                if
                    root.TryGetProperty("theme", &themeElement)
                    && themeElement.ValueKind = JsonValueKind.Object
                then
                    themeElement
                else
                    root

            let mutable schemaVersionElement = Unchecked.defaultof<JsonElement>

            if not (theme.TryGetProperty("schemaVersion", &schemaVersionElement)) then
                failwith "Theme schema warning: missing schemaVersion; theme was not loaded"

            if schemaVersionElement.ValueKind <> JsonValueKind.Number then
                failwith "Invalid theme schema version: expected an integer"

            match schemaVersionElement.TryGetInt32() with
            | false, _ -> failwith "Invalid theme schema version: expected an integer"
            | true, version when version > currentSchemaVersion ->
                failwith (sprintf "Unsupported theme schema version: %d" version)
            | true, _ -> ()

            let preset =
                tryString theme "preset" |> Option.defaultValue ThemePreset.GraphiteDark

            if not (ThemePreset.valid |> List.contains preset) then
                failwith (sprintf "Invalid theme preset: %s" preset)

            let result = ThemeSettingsPalette.apply theme preset

            result
            |> Result.bind (fun palette ->
                ThemeSettingsUi.apply theme
                |> Result.map (ThemeSettings.fromPaletteWithPresetAndUi preset palette))
        with ex ->
            Error ex.Message

    let loadFile path =
        try
            System.IO.File.ReadAllText(path) |> loadText
        with ex ->
            Error ex.Message
