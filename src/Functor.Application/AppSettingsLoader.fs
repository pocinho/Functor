namespace Functor.Application

open System.Globalization
open System.Collections.Generic
open System.Text.Json
open Functor.Rendering

module AppSettingsLoader =
    let private colorToHex color =
        sprintf "#%08X" color

    let loadText (json: string) : Result<AppSettings, string> =
        ThemeSettingsLoader.loadText json
        |> Result.map AppSettings.fromTheme

    let toJson (settings: AppSettings) =
        let palette = settings.Theme.ThemeSource.Resolve()

        let options = JsonSerializerOptions(WriteIndented = true)

        let theme = Dictionary<string, obj>()
        theme["background"] <- colorToHex palette.Background
        theme["foreground"] <- colorToHex palette.Foreground
        theme["selection"] <- colorToHex palette.Selection
        theme["cursor"] <- colorToHex palette.Cursor
        theme["lineNumber"] <- colorToHex palette.LineNumber
        theme["gutterBackground"] <- colorToHex palette.GutterBackground
        theme["diagnosticError"] <- colorToHex palette.DiagnosticError
        theme["diagnosticWarning"] <- colorToHex palette.DiagnosticWarning
        theme["diagnosticInfo"] <- colorToHex palette.DiagnosticInfo
        theme["editorBorderWidth"] <- float palette.EditorBorderWidth

        match palette.EditorBorder with
        | Some color -> theme["editorBorder"] <- colorToHex color
        | None -> ()

        JsonSerializer.Serialize(dict [ "theme", box theme ], options)