namespace Functor.Application

open System.Globalization
open Functor.Rendering

module internal SettingsFormProjection =
    let paletteForPreset preset = ThemePreset.palette preset

    let private syntaxColor name palette =
        palette.SyntaxColors
        |> Map.tryFind name
        |> Option.defaultValue palette.Foreground

    let fromAppSettingsWithPreset preset (settings: AppSettings) =
        let palette = settings.Theme.ThemeSource.Resolve()
        let ui = settings.Theme.Ui

        { ThemePreset = preset
          Background = ColorUtilities.toHex palette.Background
          Foreground = ColorUtilities.toHex palette.Foreground
          Selection = ColorUtilities.toHex palette.Selection
          Cursor = ColorUtilities.toHex palette.Cursor
          LineNumber = ColorUtilities.toHex palette.LineNumber
          GutterBackground = ColorUtilities.toHex palette.GutterBackground
          DiagnosticError = ColorUtilities.toHex palette.DiagnosticError
          DiagnosticWarning = ColorUtilities.toHex palette.DiagnosticWarning
          DiagnosticInfo = ColorUtilities.toHex palette.DiagnosticInfo
          SyntaxKeyword = ColorUtilities.toHex (syntaxColor "keyword" palette)
          SyntaxString = ColorUtilities.toHex (syntaxColor "string" palette)
          SyntaxComment = ColorUtilities.toHex (syntaxColor "comment" palette)
          SyntaxNumber = ColorUtilities.toHex (syntaxColor "number" palette)
          SyntaxType = ColorUtilities.toHex (syntaxColor "type" palette)
          SyntaxFunction = ColorUtilities.toHex (syntaxColor "function" palette)
          EditorBorder =
            palette.EditorBorder
            |> Option.map ColorUtilities.toHex
            |> Option.defaultValue ""
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
          ResizeHandleColor = ColorUtilities.toHex ui.ResizeHandleColor
          SidePanelResizeHandleWidth = ui.SidePanelResizeHandleWidth.ToString(CultureInfo.InvariantCulture)
          CommandPaletteShadowColor = ColorUtilities.toHex ui.CommandPaletteShadowColor
          WorkspaceSeparatorColor = ColorUtilities.toHex ui.WorkspaceSeparatorColor
          MeasurementColor = ColorUtilities.toHex ui.MeasurementColor
          TabCloseIconSize = ui.TabCloseIconSize.ToString(CultureInfo.InvariantCulture) }

    let fromAppSettings (settings: AppSettings) =
        fromAppSettingsWithPreset settings.Theme.Preset settings

    let applyPreset preset form =
        let palette = paletteForPreset preset

        { form with
            ThemePreset = preset
            Background = ColorUtilities.toHex palette.Background
            Foreground = ColorUtilities.toHex palette.Foreground
            Selection = ColorUtilities.toHex palette.Selection
            Cursor = ColorUtilities.toHex palette.Cursor
            LineNumber = ColorUtilities.toHex palette.LineNumber
            GutterBackground = ColorUtilities.toHex palette.GutterBackground
            DiagnosticError = ColorUtilities.toHex palette.DiagnosticError
            DiagnosticWarning = ColorUtilities.toHex palette.DiagnosticWarning
            DiagnosticInfo = ColorUtilities.toHex palette.DiagnosticInfo
            SyntaxKeyword = ColorUtilities.toHex (syntaxColor "keyword" palette)
            SyntaxString = ColorUtilities.toHex (syntaxColor "string" palette)
            SyntaxComment = ColorUtilities.toHex (syntaxColor "comment" palette)
            SyntaxNumber = ColorUtilities.toHex (syntaxColor "number" palette)
            SyntaxType = ColorUtilities.toHex (syntaxColor "type" palette)
            SyntaxFunction = ColorUtilities.toHex (syntaxColor "function" palette)
            EditorBorder =
                palette.EditorBorder
                |> Option.map ColorUtilities.toHex
                |> Option.defaultValue ""
            EditorBorderWidth = palette.EditorBorderWidth.ToString(CultureInfo.InvariantCulture)
            ResizeHandleColor = ColorUtilities.toHex (ThemePreset.ui preset).ResizeHandleColor
            SidePanelResizeHandleWidth =
                (ThemePreset.ui preset).SidePanelResizeHandleWidth.ToString(CultureInfo.InvariantCulture) }
