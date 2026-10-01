namespace Functor.Application

open System
open System.Globalization
open Functor.Rendering

module internal SettingsFormValidation =
    let private parseColor name (value: string) =
        let normalized = value.Trim().TrimStart('#')

        if normalized.Length <> 6 && normalized.Length <> 8 then
            Error(sprintf "%s must be a 6- or 8-digit hex color." name)
        else
            let normalized =
                if normalized.Length = 6 then
                    "FF" + normalized
                else
                    normalized

            let mutable parsed = 0u

            if UInt32.TryParse(normalized, NumberStyles.HexNumber, CultureInfo.InvariantCulture, &parsed) then
                Ok parsed
            else
                Error(sprintf "%s must be a 6- or 8-digit hex color." name)

    let private parseOptionalColor name (value: string) =
        if String.IsNullOrWhiteSpace value then
            Ok None
        else
            parseColor name value |> Result.map Some

    let tryBuildAppSettings (form: SettingsFormModel) =
        let draft = form
        let bind next result = Result.bind next result

        Ok(ThemePreset.palette form.ThemePreset)
        |> bind (fun palette ->
            parseColor "Background" draft.Background
            |> Result.map (fun value -> { palette with Background = value }))
        |> bind (fun palette ->
            parseColor "Foreground" draft.Foreground
            |> Result.map (fun value -> { palette with Foreground = value }))
        |> bind (fun palette ->
            parseColor "Selection" draft.Selection
            |> Result.map (fun value -> { palette with Selection = value }))
        |> bind (fun palette ->
            parseColor "Cursor" draft.Cursor
            |> Result.map (fun value -> { palette with Cursor = value }))
        |> bind (fun palette ->
            parseColor "Line number" draft.LineNumber
            |> Result.map (fun value -> { palette with LineNumber = value }))
        |> bind (fun palette ->
            parseColor "Gutter background" draft.GutterBackground
            |> Result.map (fun value ->
                { palette with
                    GutterBackground = value }))
        |> bind (fun palette ->
            parseColor "Diagnostic error" draft.DiagnosticError
            |> Result.map (fun value -> { palette with DiagnosticError = value }))
        |> bind (fun palette ->
            parseColor "Diagnostic warning" draft.DiagnosticWarning
            |> Result.map (fun value ->
                { palette with
                    DiagnosticWarning = value }))
        |> bind (fun palette ->
            parseColor "Diagnostic info" draft.DiagnosticInfo
            |> Result.map (fun value -> { palette with DiagnosticInfo = value }))
        |> bind (fun palette ->
            parseColor "Syntax keyword" draft.SyntaxKeyword
            |> Result.map (fun value ->
                { palette with
                    SyntaxColors = palette.SyntaxColors.Add("keyword", value) }))
        |> bind (fun palette ->
            parseColor "Syntax string" draft.SyntaxString
            |> Result.map (fun value ->
                { palette with
                    SyntaxColors = palette.SyntaxColors.Add("string", value) }))
        |> bind (fun palette ->
            parseColor "Syntax comment" draft.SyntaxComment
            |> Result.map (fun value ->
                { palette with
                    SyntaxColors = palette.SyntaxColors.Add("comment", value) }))
        |> bind (fun palette ->
            parseColor "Syntax number" draft.SyntaxNumber
            |> Result.map (fun value ->
                { palette with
                    SyntaxColors = palette.SyntaxColors.Add("number", value) }))
        |> bind (fun palette ->
            parseColor "Syntax type" draft.SyntaxType
            |> Result.map (fun value ->
                { palette with
                    SyntaxColors = palette.SyntaxColors.Add("type", value) }))
        |> bind (fun palette ->
            parseColor "Syntax function" draft.SyntaxFunction
            |> Result.map (fun value ->
                { palette with
                    SyntaxColors = palette.SyntaxColors.Add("function", value) }))
        |> bind (fun palette ->
            parseOptionalColor "Editor border" draft.EditorBorder
            |> Result.map (fun value -> { palette with EditorBorder = value }))
        |> bind (fun palette ->
            let mutable width = 0.0f

            if
                Single.TryParse(draft.EditorBorderWidth, NumberStyles.Float, CultureInfo.InvariantCulture, &width)
                && Single.IsFinite width
                && width >= 0.0f
            then
                Ok
                    { palette with
                        EditorBorderWidth = width }
            else
                Error "Editor border width must be a non-negative number.")
        |> bind (fun palette ->
            let parsePositiveFloat name (value: string) =
                let mutable parsed = 0.0

                if
                    Double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, &parsed)
                    && Double.IsFinite parsed
                    && parsed > 0.0
                then
                    Ok parsed
                else
                    Error(sprintf "%s must be a positive number." name)

            let parseNonNegativeFloat name (value: string) =
                let mutable parsed = 0.0

                if
                    Double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, &parsed)
                    && Double.IsFinite parsed
                    && parsed >= 0.0
                then
                    Ok parsed
                else
                    Error(sprintf "%s must be a non-negative number." name)

            let parsePositiveInt name (value: string) =
                let mutable parsed = 0

                if
                    Int32.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, &parsed)
                    && parsed > 0
                then
                    Ok parsed
                else
                    Error(sprintf "%s must be a positive integer." name)

            Ok UiThemeDefaults.defaultTheme
            |> bind (fun ui ->
                if String.IsNullOrWhiteSpace draft.EditorFontFamily then
                    Error "Editor font family cannot be empty."
                else
                    Ok
                        { ui with
                            EditorFontFamily = draft.EditorFontFamily })
            |> bind (fun ui ->
                if String.IsNullOrWhiteSpace draft.EditorFallbackFontFamily then
                    Error "Editor fallback font family cannot be empty."
                else
                    Ok
                        { ui with
                            EditorFallbackFontFamily = draft.EditorFallbackFontFamily })
            |> bind (fun ui ->
                if String.IsNullOrWhiteSpace draft.UiFontFamily then
                    Error "UI font family cannot be empty."
                else
                    Ok
                        { ui with
                            UiFontFamily = draft.UiFontFamily })
            |> bind (fun ui ->
                if String.IsNullOrWhiteSpace draft.IconFontFamily then
                    Error "Icon font family cannot be empty."
                else
                    Ok
                        { ui with
                            IconFontFamily = draft.IconFontFamily })
            |> bind (fun ui ->
                parsePositiveFloat "Workspace font size" draft.WorkspaceFontSize
                |> Result.map (fun value -> { ui with WorkspaceFontSize = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Editor font size" draft.EditorFontSize
                |> Result.map (fun value -> { ui with EditorFontSize = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Editor line height" draft.EditorLineHeight
                |> Result.map (fun value -> { ui with EditorLineHeight = value }))
            |> bind (fun ui ->
                parsePositiveInt "Editor tab size" draft.EditorTabSize
                |> Result.map (fun value -> { ui with EditorTabSize = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Cursor width" draft.CursorWidth
                |> Result.map (fun value -> { ui with CursorWidth = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Gutter separator width" draft.GutterSeparatorWidth
                |> Result.map (fun value -> { ui with GutterSeparatorWidth = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Gutter padding" draft.GutterPadding
                |> Result.map (fun value -> { ui with GutterPadding = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Gutter minimum width" draft.GutterMinimumWidth
                |> Result.map (fun value -> { ui with GutterMinimumWidth = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Document tab minimum height" draft.DocumentTabMinHeight
                |> Result.map (fun value -> { ui with DocumentTabMinHeight = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Document tab close button size" draft.DocumentTabCloseButtonSize
                |> Result.map (fun value ->
                    { ui with
                        DocumentTabCloseButtonSize = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Document tab horizontal padding" draft.DocumentTabPaddingHorizontal
                |> Result.map (fun value ->
                    { ui with
                        DocumentTabPaddingHorizontal = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Document tab vertical padding" draft.DocumentTabPaddingVertical
                |> Result.map (fun value ->
                    { ui with
                        DocumentTabPaddingVertical = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Document tab spacing" draft.DocumentTabSpacing
                |> Result.map (fun value -> { ui with DocumentTabSpacing = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Command palette font size" draft.CommandPaletteFontSize
                |> Result.map (fun value ->
                    { ui with
                        CommandPaletteFontSize = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Command palette width" draft.CommandPaletteWidth
                |> Result.map (fun value -> { ui with CommandPaletteWidth = value }))
            |> bind (fun ui ->
                parseNonNegativeFloat "Command palette top margin" draft.CommandPaletteTopMargin
                |> Result.map (fun value ->
                    { ui with
                        CommandPaletteTopMargin = value }))
            |> bind (fun ui ->
                parseNonNegativeFloat "Command palette padding" draft.CommandPalettePadding
                |> Result.map (fun value ->
                    { ui with
                        CommandPalettePadding = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Command palette maximum height" draft.CommandPaletteMaxHeight
                |> Result.map (fun value ->
                    { ui with
                        CommandPaletteMaxHeight = value }))
            |> bind (fun ui ->
                parseNonNegativeFloat
                    "Command palette item horizontal margin"
                    draft.CommandPaletteItemMarginHorizontal
                |> Result.map (fun value ->
                    { ui with
                        CommandPaletteItemMarginHorizontal = value }))
            |> bind (fun ui ->
                parseNonNegativeFloat "Command palette item vertical margin" draft.CommandPaletteItemMarginVertical
                |> Result.map (fun value ->
                    { ui with
                        CommandPaletteItemMarginVertical = value }))
            |> bind (fun ui ->
                parseNonNegativeFloat "Command palette gesture margin" draft.CommandPaletteGestureMargin
                |> Result.map (fun value ->
                    { ui with
                        CommandPaletteGestureMargin = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Welcome title font size" draft.WelcomeTitleFontSize
                |> Result.map (fun value -> { ui with WelcomeTitleFontSize = value }))
            |> bind (fun ui ->
                let mutable parsed = 0.0

                if
                    Double.TryParse(draft.TextMutedOpacity, NumberStyles.Float, CultureInfo.InvariantCulture, &parsed)
                    && Double.IsFinite parsed
                    && parsed >= 0.0
                    && parsed <= 1.0
                then
                    Ok { ui with TextMutedOpacity = parsed }
                else
                    Error "Text muted opacity must be between 0 and 1.")
            |> bind (fun ui ->
                parseNonNegativeFloat "Control corner radius" draft.ControlCornerRadius
                |> Result.map (fun value -> { ui with ControlCornerRadius = value }))
            |> bind (fun ui ->
                parseColor "Resize handle color" draft.ResizeHandleColor
                |> Result.map (fun value -> { ui with ResizeHandleColor = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Side panel resize handle width" draft.SidePanelResizeHandleWidth
                |> Result.map (fun value ->
                    { ui with
                        SidePanelResizeHandleWidth = value }))
            |> bind (fun ui ->
                parseColor "Command palette shadow color" draft.CommandPaletteShadowColor
                |> Result.map (fun value ->
                    { ui with
                        CommandPaletteShadowColor = value }))
            |> bind (fun ui ->
                parseColor "Workspace separator color" draft.WorkspaceSeparatorColor
                |> Result.map (fun value ->
                    { ui with
                        WorkspaceSeparatorColor = value }))
            |> bind (fun ui ->
                parseColor "Measurement color" draft.MeasurementColor
                |> Result.map (fun value -> { ui with MeasurementColor = value }))
            |> bind (fun ui ->
                parsePositiveFloat "Tab close icon size" draft.TabCloseIconSize
                |> Result.map (fun value -> { ui with TabCloseIconSize = value }))
            |> Result.map (
                ThemeSettings.fromPaletteWithPresetAndUi draft.ThemePreset palette
                >> AppSettings.fromTheme
            ))
