namespace Functor.Application

open System.Globalization
open Functor.Rendering

module SettingsFormProjection =
    let private colorText color = sprintf "#%08X" color

    let paletteForPreset preset = ThemePreset.palette preset

    let private syntaxColor name palette =
        palette.SyntaxColors
        |> Map.tryFind name
        |> Option.defaultValue palette.Foreground

    let fromAppSettingsWithPreset preset (settings: AppSettings) =
        let palette = settings.Theme.ThemeSource.Resolve()
        let ui = settings.Theme.Ui

        { ThemePreset = preset
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
          EditorFontFamily = ui.EditorFontFamily
          EditorFallbackFontFamily = ui.EditorFallbackFontFamily
          UiFontFamily = ui.UiFontFamily
          IconFontFamily = ui.IconFontFamily
          WorkspaceFontSize = ui.WorkspaceFontSize.ToString(CultureInfo.InvariantCulture)
          EditorFontSize = ui.EditorFontSize.ToString(CultureInfo.InvariantCulture)
          EditorLineHeight = ui.EditorLineHeight.ToString(CultureInfo.InvariantCulture)
          EditorTabSize = ui.EditorTabSize.ToString(CultureInfo.InvariantCulture)
          CursorWidth = ui.CursorWidth.ToString(CultureInfo.InvariantCulture)
          GutterSeparatorWidth = ui.GutterSeparatorWidth.ToString(CultureInfo.InvariantCulture)
          GutterPadding = ui.GutterPadding.ToString(CultureInfo.InvariantCulture)
          GutterMinimumWidth = ui.GutterMinimumWidth.ToString(CultureInfo.InvariantCulture)
          DocumentTabMinHeight = ui.DocumentTabMinHeight.ToString(CultureInfo.InvariantCulture)
          DocumentTabCloseButtonSize = ui.DocumentTabCloseButtonSize.ToString(CultureInfo.InvariantCulture)
          DocumentTabPaddingHorizontal = ui.DocumentTabPaddingHorizontal.ToString(CultureInfo.InvariantCulture)
          DocumentTabPaddingVertical = ui.DocumentTabPaddingVertical.ToString(CultureInfo.InvariantCulture)
          DocumentTabSpacing = ui.DocumentTabSpacing.ToString(CultureInfo.InvariantCulture)
          CommandPaletteFontSize = ui.CommandPaletteFontSize.ToString(CultureInfo.InvariantCulture)
          CommandPaletteWidth = ui.CommandPaletteWidth.ToString(CultureInfo.InvariantCulture)
          CommandPaletteTopMargin = ui.CommandPaletteTopMargin.ToString(CultureInfo.InvariantCulture)
          CommandPalettePadding = ui.CommandPalettePadding.ToString(CultureInfo.InvariantCulture)
          CommandPaletteMaxHeight = ui.CommandPaletteMaxHeight.ToString(CultureInfo.InvariantCulture)
          CommandPaletteItemMarginHorizontal =
            ui.CommandPaletteItemMarginHorizontal.ToString(CultureInfo.InvariantCulture)
          CommandPaletteItemMarginVertical = ui.CommandPaletteItemMarginVertical.ToString(CultureInfo.InvariantCulture)
          CommandPaletteGestureMargin = ui.CommandPaletteGestureMargin.ToString(CultureInfo.InvariantCulture)
          WelcomeTitleFontSize = ui.WelcomeTitleFontSize.ToString(CultureInfo.InvariantCulture)
          TextMutedOpacity = ui.TextMutedOpacity.ToString(CultureInfo.InvariantCulture)
          ControlCornerRadius = ui.ControlCornerRadius.ToString(CultureInfo.InvariantCulture)
          ResizeHandleColor = colorText ui.ResizeHandleColor
          CommandPaletteShadowColor = colorText ui.CommandPaletteShadowColor
          WorkspaceSeparatorColor = colorText ui.WorkspaceSeparatorColor
          MeasurementColor = colorText ui.MeasurementColor
          TabCloseIconSize = ui.TabCloseIconSize.ToString(CultureInfo.InvariantCulture) }

    let fromAppSettings (settings: AppSettings) =
        fromAppSettingsWithPreset settings.Theme.Preset settings

    let applyPreset preset form =
        let palette = paletteForPreset preset

        { form with
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
