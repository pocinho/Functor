namespace Functor.Application

open System.Globalization
open System.Collections.Generic
open System.Text.Json
open Functor.Rendering

module AppSettingsLoader =
    let private currentSchemaVersion = 1
    let private colorToHex color = sprintf "#%08X" color

    let loadText (json: string) : Result<AppSettings, string> =
        ThemeSettingsLoader.loadText json |> Result.map AppSettings.fromTheme

    let toJson (settings: AppSettings) =
        let palette = settings.Theme.ThemeSource.Resolve()

        let options = JsonSerializerOptions(WriteIndented = true)

        let theme = Dictionary<string, obj>()
        theme["schemaVersion"] <- currentSchemaVersion
        theme["preset"] <- settings.Theme.Preset
        theme["background"] <- colorToHex palette.Background
        theme["foreground"] <- colorToHex palette.Foreground
        theme["selection"] <- colorToHex palette.Selection
        theme["cursor"] <- colorToHex palette.Cursor
        theme["lineNumber"] <- colorToHex palette.LineNumber
        theme["gutterBackground"] <- colorToHex palette.GutterBackground
        theme["diagnosticError"] <- colorToHex palette.DiagnosticError
        theme["diagnosticWarning"] <- colorToHex palette.DiagnosticWarning
        theme["diagnosticInfo"] <- colorToHex palette.DiagnosticInfo

        theme["syntaxKeyword"] <-
            colorToHex (
                palette.SyntaxColors
                |> Map.tryFind "keyword"
                |> Option.defaultValue palette.Foreground
            )

        theme["syntaxString"] <-
            colorToHex (
                palette.SyntaxColors
                |> Map.tryFind "string"
                |> Option.defaultValue palette.Foreground
            )

        theme["syntaxComment"] <-
            colorToHex (
                palette.SyntaxColors
                |> Map.tryFind "comment"
                |> Option.defaultValue palette.Foreground
            )

        theme["syntaxNumber"] <-
            colorToHex (
                palette.SyntaxColors
                |> Map.tryFind "number"
                |> Option.defaultValue palette.Foreground
            )

        theme["syntaxType"] <-
            colorToHex (
                palette.SyntaxColors
                |> Map.tryFind "type"
                |> Option.defaultValue palette.Foreground
            )

        theme["syntaxFunction"] <-
            colorToHex (
                palette.SyntaxColors
                |> Map.tryFind "function"
                |> Option.defaultValue palette.Foreground
            )

        theme["editorBorderWidth"] <- float palette.EditorBorderWidth
        let ui = settings.Theme.Ui
        theme["editorFontFamily"] <- ui.EditorFontFamily
        theme["editorFallbackFontFamily"] <- ui.EditorFallbackFontFamily
        theme["uiFontFamily"] <- ui.UiFontFamily
        theme["iconFontFamily"] <- ui.IconFontFamily
        theme["tabCloseIconSize"] <- ui.TabCloseIconSize
        theme["workspaceFontSize"] <- ui.WorkspaceFontSize
        theme["editorFontSize"] <- ui.EditorFontSize
        theme["editorLineHeight"] <- ui.EditorLineHeight
        theme["editorTabSize"] <- ui.EditorTabSize
        theme["cursorWidth"] <- ui.CursorWidth
        theme["gutterSeparatorWidth"] <- ui.GutterSeparatorWidth
        theme["gutterPadding"] <- ui.GutterPadding
        theme["gutterMinimumWidth"] <- ui.GutterMinimumWidth
        theme["commandPaletteFontSize"] <- ui.CommandPaletteFontSize
        theme["welcomeTitleFontSize"] <- ui.WelcomeTitleFontSize
        theme["textMutedOpacity"] <- ui.TextMutedOpacity
        theme["controlCornerRadius"] <- ui.ControlCornerRadius
        theme["resizeHandleColor"] <- colorToHex ui.ResizeHandleColor
        theme["commandPaletteShadowColor"] <- colorToHex ui.CommandPaletteShadowColor
        theme["workspaceSeparatorColor"] <- colorToHex ui.WorkspaceSeparatorColor
        theme["measurementColor"] <- colorToHex ui.MeasurementColor

        match palette.EditorBorder with
        | Some color -> theme["editorBorder"] <- colorToHex color
        | None -> ()

        JsonSerializer.Serialize(dict [ "theme", box theme ], options)
