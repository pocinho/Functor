namespace Functor.Application

open System
open System.Globalization
open Functor.Rendering

type SettingsDraft =
    { ThemePreset: string
      Background: string
      Foreground: string
      Selection: string
      Cursor: string
      LineNumber: string
      GutterBackground: string
      DiagnosticError: string
      DiagnosticWarning: string
      DiagnosticInfo: string
      SyntaxKeyword: string
      SyntaxString: string
      SyntaxComment: string
      SyntaxNumber: string
      SyntaxType: string
      SyntaxFunction: string
      EditorBorder: string
      EditorBorderWidth: string
      EditorFontFamily: string
      EditorFallbackFontFamily: string
      IconFontFamily: string
      WorkspaceFontSize: string
      EditorLineHeight: string
      EditorTabSize: string
      CursorWidth: string
      GutterSeparatorWidth: string
      GutterPadding: string
      GutterMinimumWidth: string
      CommandPaletteFontSize: string
      WelcomeTitleFontSize: string
      TextMutedOpacity: string
      ControlCornerRadius: string
      ResizeHandleColor: string
      CommandPaletteShadowColor: string
      WorkspaceSeparatorColor: string
      MeasurementColor: string
      TabCloseIconSize: string
      UiFontFamily: string }

module SettingsDraft =
    let private colorText color = sprintf "#%08X" color

    let paletteForPreset preset =
        if preset = "Graphite Light" then
            Theme.graphiteLight
        else
            Theme.defaultPalette

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

    let private syntaxColor name palette =
        palette.SyntaxColors
        |> Map.tryFind name
        |> Option.defaultValue palette.Foreground

    let fromSettings (settings: AppSettings) =
        let palette = settings.Theme.ThemeSource.Resolve()

        { ThemePreset = settings.Theme.Preset
          Background = colorText palette.Background
          Foreground = colorText palette.Foreground
          Selection = colorText palette.Selection
          Cursor = colorText palette.Cursor
          LineNumber = colorText palette.LineNumber
          GutterBackground = colorText palette.GutterBackground
          DiagnosticError = colorText palette.DiagnosticError
          DiagnosticWarning = colorText palette.DiagnosticWarning
          DiagnosticInfo = colorText palette.DiagnosticInfo
          SyntaxKeyword = colorText (syntaxColor "keyword" palette)
          SyntaxString = colorText (syntaxColor "string" palette)
          SyntaxComment = colorText (syntaxColor "comment" palette)
          SyntaxNumber = colorText (syntaxColor "number" palette)
          SyntaxType = colorText (syntaxColor "type" palette)
          SyntaxFunction = colorText (syntaxColor "function" palette)
          EditorBorder = palette.EditorBorder |> Option.map colorText |> Option.defaultValue ""
          EditorBorderWidth = palette.EditorBorderWidth.ToString(CultureInfo.InvariantCulture)
          EditorFontFamily = settings.Theme.Ui.EditorFontFamily
          EditorFallbackFontFamily = settings.Theme.Ui.EditorFallbackFontFamily
          UiFontFamily = settings.Theme.Ui.UiFontFamily
          IconFontFamily = settings.Theme.Ui.IconFontFamily
          WorkspaceFontSize = settings.Theme.Ui.WorkspaceFontSize.ToString(CultureInfo.InvariantCulture)
          EditorLineHeight = settings.Theme.Ui.EditorLineHeight.ToString(CultureInfo.InvariantCulture)
          EditorTabSize = settings.Theme.Ui.EditorTabSize.ToString(CultureInfo.InvariantCulture)
          CursorWidth = settings.Theme.Ui.CursorWidth.ToString(CultureInfo.InvariantCulture)
          GutterSeparatorWidth = settings.Theme.Ui.GutterSeparatorWidth.ToString(CultureInfo.InvariantCulture)
          GutterPadding = settings.Theme.Ui.GutterPadding.ToString(CultureInfo.InvariantCulture)
          GutterMinimumWidth = settings.Theme.Ui.GutterMinimumWidth.ToString(CultureInfo.InvariantCulture)
          CommandPaletteFontSize = settings.Theme.Ui.CommandPaletteFontSize.ToString(CultureInfo.InvariantCulture)
          WelcomeTitleFontSize = settings.Theme.Ui.WelcomeTitleFontSize.ToString(CultureInfo.InvariantCulture)
          TextMutedOpacity = settings.Theme.Ui.TextMutedOpacity.ToString(CultureInfo.InvariantCulture)
          ControlCornerRadius = settings.Theme.Ui.ControlCornerRadius.ToString(CultureInfo.InvariantCulture)
          ResizeHandleColor = colorText settings.Theme.Ui.ResizeHandleColor
          CommandPaletteShadowColor = colorText settings.Theme.Ui.CommandPaletteShadowColor
          WorkspaceSeparatorColor = colorText settings.Theme.Ui.WorkspaceSeparatorColor
          MeasurementColor = colorText settings.Theme.Ui.MeasurementColor
          TabCloseIconSize = settings.Theme.Ui.TabCloseIconSize.ToString(CultureInfo.InvariantCulture) }

    let applyPreset preset draft =
        let palette = paletteForPreset preset

        { draft with
            ThemePreset = preset
            Background = colorText palette.Background
            Foreground = colorText palette.Foreground
            Selection = colorText palette.Selection
            Cursor = colorText palette.Cursor
            LineNumber = colorText palette.LineNumber
            GutterBackground = colorText palette.GutterBackground
            DiagnosticError = colorText palette.DiagnosticError
            DiagnosticWarning = colorText palette.DiagnosticWarning
            DiagnosticInfo = colorText palette.DiagnosticInfo
            SyntaxKeyword = colorText (syntaxColor "keyword" palette)
            SyntaxString = colorText (syntaxColor "string" palette)
            SyntaxComment = colorText (syntaxColor "comment" palette)
            SyntaxNumber = colorText (syntaxColor "number" palette)
            SyntaxType = colorText (syntaxColor "type" palette)
            SyntaxFunction = colorText (syntaxColor "function" palette)
            EditorBorder = palette.EditorBorder |> Option.map colorText |> Option.defaultValue ""
            EditorBorderWidth = palette.EditorBorderWidth.ToString(CultureInfo.InvariantCulture) }

    let tryCreateSettings draft =
        let bind next result = Result.bind next result

        Ok(paletteForPreset draft.ThemePreset)
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
                parsePositiveFloat "Command palette font size" draft.CommandPaletteFontSize
                |> Result.map (fun value ->
                    { ui with
                        CommandPaletteFontSize = value }))
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
