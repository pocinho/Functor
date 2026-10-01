namespace Functor.Platform

open System
open System.Text.Json
open Functor.Application
open Functor.Rendering
open ThemeSettingsJson

module ThemeSettingsLoader =
    let loadText (json: string) : Result<ThemeSettings, string> =
        try
            use document = JsonDocument.Parse(json)
            match ThemeSchemaMigration.migrate document.RootElement with
            | Error error -> Error error
            | Ok theme ->
                let preset =
                    tryString theme "preset" |> Option.defaultValue ThemePreset.GraphiteDark

                if not (ThemePreset.valid |> List.contains preset) then
                    Error(sprintf "Invalid theme preset: %s" preset)
                else
                    let result = ThemeSettingsPalette.apply theme preset

                    result
                    |> Result.bind (fun palette ->
                        ThemeSettingsUi.apply theme preset
                        |> Result.map (ThemeSettings.fromPaletteWithPresetAndUi preset palette))
        with ex ->
            Error ex.Message

    let loadFile path =
        try
            System.IO.File.ReadAllText(path) |> loadText
        with ex ->
            Error ex.Message
