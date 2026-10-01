namespace Functor.Platform

open System.Globalization
open System.Collections.Generic
open System.Text.Json
open Functor.Application
open Functor.Rendering

module AppSettingsLoader =
    let private currentSchemaVersion = 1

    let loadText (json: string) : Result<AppSettings, string> =
        ThemeSettingsLoader.loadText json |> Result.map AppSettings.fromTheme

    let toJson (settings: AppSettings) =
        let palette = settings.Theme.ThemeSource.Resolve()

        let options = JsonSerializerOptions(WriteIndented = true)

        let theme = Dictionary<string, obj>()
        theme["schemaVersion"] <- currentSchemaVersion
        theme["preset"] <- settings.Theme.Preset
        theme["background"] <- ColorUtilities.toHex palette.Background
        theme["foreground"] <- ColorUtilities.toHex palette.Foreground
        theme["selection"] <- ColorUtilities.toHex palette.Selection
        theme["cursor"] <- ColorUtilities.toHex palette.Cursor
        theme["lineNumber"] <- ColorUtilities.toHex palette.LineNumber
        theme["gutterBackground"] <- ColorUtilities.toHex palette.GutterBackground
        theme["diagnosticError"] <- ColorUtilities.toHex palette.DiagnosticError
        theme["diagnosticWarning"] <- ColorUtilities.toHex palette.DiagnosticWarning
        theme["diagnosticInfo"] <- ColorUtilities.toHex palette.DiagnosticInfo

        theme["syntaxKeyword"] <-
            ColorUtilities.toHex (
                palette.SyntaxColors
                |> Map.tryFind "keyword"
                |> Option.defaultValue palette.Foreground
            )

        theme["syntaxString"] <-
            ColorUtilities.toHex (
                palette.SyntaxColors
                |> Map.tryFind "string"
                |> Option.defaultValue palette.Foreground
            )

        theme["syntaxComment"] <-
            ColorUtilities.toHex (
                palette.SyntaxColors
                |> Map.tryFind "comment"
                |> Option.defaultValue palette.Foreground
            )

        theme["syntaxNumber"] <-
            ColorUtilities.toHex (
                palette.SyntaxColors
                |> Map.tryFind "number"
                |> Option.defaultValue palette.Foreground
            )

        theme["syntaxType"] <-
            ColorUtilities.toHex (
                palette.SyntaxColors
                |> Map.tryFind "type"
                |> Option.defaultValue palette.Foreground
            )

        theme["syntaxFunction"] <-
            ColorUtilities.toHex (
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
        theme["documentTabMinHeight"] <- ui.DocumentTabMinHeight
        theme["documentTabCloseButtonSize"] <- ui.DocumentTabCloseButtonSize
        theme["documentTabPaddingHorizontal"] <- ui.DocumentTabPaddingHorizontal
        theme["documentTabPaddingVertical"] <- ui.DocumentTabPaddingVertical
        theme["documentTabSpacing"] <- ui.DocumentTabSpacing
        theme["commandPaletteFontSize"] <- ui.CommandPaletteFontSize
        theme["commandPaletteWidth"] <- ui.CommandPaletteWidth
        theme["commandPaletteTopMargin"] <- ui.CommandPaletteTopMargin
        theme["commandPalettePadding"] <- ui.CommandPalettePadding
        theme["commandPaletteMaxHeight"] <- ui.CommandPaletteMaxHeight
        theme["commandPaletteItemMarginHorizontal"] <- ui.CommandPaletteItemMarginHorizontal
        theme["commandPaletteItemMarginVertical"] <- ui.CommandPaletteItemMarginVertical
        theme["commandPaletteGestureMargin"] <- ui.CommandPaletteGestureMargin
        theme["welcomeTitleFontSize"] <- ui.WelcomeTitleFontSize
        theme["textMutedOpacity"] <- ui.TextMutedOpacity
        theme["controlCornerRadius"] <- ui.ControlCornerRadius
        theme["resizeHandleColor"] <- ColorUtilities.toHex ui.ResizeHandleColor
        theme["sidePanelResizeHandleWidth"] <- ui.SidePanelResizeHandleWidth
        theme["commandPaletteShadowColor"] <- ColorUtilities.toHex ui.CommandPaletteShadowColor
        theme["workspaceSeparatorColor"] <- ColorUtilities.toHex ui.WorkspaceSeparatorColor
        theme["measurementColor"] <- ColorUtilities.toHex ui.MeasurementColor

        match palette.EditorBorder with
        | Some color -> theme["editorBorder"] <- ColorUtilities.toHex color
        | None -> ()

        JsonSerializer.Serialize(dict [ "theme", box theme ], options)
