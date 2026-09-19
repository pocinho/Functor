namespace Functor.Avalonia

open Avalonia
open Avalonia.Controls
open Avalonia.Media
open System.Collections.Generic
open Functor.Application
open Functor.Rendering

module ThemeResources =
    let fontWeight weight =
        match weight with
        | 700 -> FontWeight.Bold
        | 600 -> FontWeight.SemiBold
        | _ -> FontWeight.Normal

    let private colorFromArgb (argb: uint32) =
        Color.FromArgb(byte (argb >>> 24), byte (argb >>> 16), byte (argb >>> 8), byte argb)

    let private brushFromArgb argb = SolidColorBrush(colorFromArgb argb)

    let private commandPaletteShadow argb =
        BoxShadows.Parse(sprintf "0 12 32 0 #%08X" argb)

    let private setItems (resources: IResourceDictionary) (items: (string * obj) seq) =
        let entries =
            items |> Seq.map (fun (key, value) -> KeyValuePair<obj, obj>(box key, value))

        match resources with
        | :? ResourceDictionary as dictionary -> dictionary.SetItems(entries)
        | _ -> items |> Seq.iter (fun (key, value) -> resources[key] <- value)

    let applyUi (resources: IResourceDictionary) (uiTheme: UiThemeDefaults) =
        let resizeHandleBrush = brushFromArgb uiTheme.ResizeHandleColor
        let workspaceSeparatorBrush = brushFromArgb uiTheme.WorkspaceSeparatorColor
        let shadow = commandPaletteShadow uiTheme.CommandPaletteShadowColor

        setItems
            resources
            [ "Theme.EditorFontFamily", box uiTheme.EditorFontFamily
              "Theme.EditorFallbackFontFamily", box uiTheme.EditorFallbackFontFamily
              "Theme.ControlFontFamily", box (FontFamily(uiTheme.UiFontFamily))
              "Theme.IconFontFamily", box (FontFamily(uiTheme.IconFontFamily))
              "Theme.TabCloseIconSize", box uiTheme.TabCloseIconSize
              "Theme.DocumentTabMinHeight", box UiThemeDefaults.documentTabMinHeight
              "Theme.EditorFontSize", box uiTheme.EditorFontSize
              "Theme.EditorLineHeight", box uiTheme.EditorLineHeight
              "Theme.EditorTabSize", box uiTheme.EditorTabSize
              "Theme.CursorWidth", box uiTheme.CursorWidth
              "Theme.GutterSeparatorWidth", box uiTheme.GutterSeparatorWidth
              "Theme.WorkspaceFontSize", box uiTheme.WorkspaceFontSize
              "Theme.CommandPaletteFontSize", box uiTheme.CommandPaletteFontSize
              "Theme.WelcomeTitleFontSize", box uiTheme.WelcomeTitleFontSize
              "Theme.FontWeightBold", box (fontWeight UiThemeDefaults.boldFontWeight)
              "Theme.FontWeightSemiBold", box (fontWeight UiThemeDefaults.semiBoldFontWeight)
              "Theme.BorderThickness", box (Thickness(UiThemeDefaults.separatorWidth))
              "Theme.SeparatorBottomThickness", box (Thickness(0, 0, 0, UiThemeDefaults.separatorWidth))
              "Theme.SeparatorRightThickness", box (Thickness(0, 0, UiThemeDefaults.separatorWidth, 0))
              "Theme.SeparatorTopThickness", box (Thickness(0, UiThemeDefaults.separatorWidth, 0, 0))
              "Theme.TextMutedOpacity", box uiTheme.TextMutedOpacity
              "Theme.ControlCornerRadius", box (CornerRadius(uiTheme.ControlCornerRadius))
              "Theme.ResizeHandleColor", box uiTheme.ResizeHandleColor
              "Theme.ResizeHandleBrush", box resizeHandleBrush
              "Theme.CommandPaletteShadowColor", box uiTheme.CommandPaletteShadowColor
              "Theme.CommandPaletteShadow", box shadow
              "Theme.WorkspaceSeparatorColor", box uiTheme.WorkspaceSeparatorColor
              "Theme.WorkspaceSeparatorBrush", box workspaceSeparatorBrush
              "Theme.MeasurementColor", box uiTheme.MeasurementColor ]

    let applyPalette (resources: IResourceDictionary) (palette: ThemePalette) =
        let surfaceBackground = brushFromArgb palette.Background
        let panelBackground = brushFromArgb palette.GutterBackground
        let inputBackground = brushFromArgb palette.GutterBackground
        let textPrimary = brushFromArgb palette.Foreground
        let textMuted = brushFromArgb palette.LineNumber
        let selection = brushFromArgb palette.Selection
        let hover = brushFromArgb palette.Selection
        let pressed = brushFromArgb palette.Cursor
        let disabled = brushFromArgb palette.LineNumber
        let error = brushFromArgb palette.DiagnosticError
        let warning = brushFromArgb palette.DiagnosticWarning
        let information = brushFromArgb palette.DiagnosticInfo

        let separator = palette.GutterSeparator |> Option.defaultValue palette.Foreground
        let separatorBrush = brushFromArgb separator
        let focus = brushFromArgb palette.Cursor

        let resizeHandleBrush = resources["Theme.ResizeHandleBrush"]
        let shadow = resources["Theme.CommandPaletteShadow"]

        setItems
            resources
            [ "Theme.SurfaceBackground", box surfaceBackground
              "Theme.SurfaceBackgroundBrush", box surfaceBackground
              "Theme.PanelBackground", box panelBackground
              "Theme.PanelBackgroundBrush", box panelBackground
              "Theme.InputBackground", box inputBackground
              "Theme.InputBackgroundBrush", box inputBackground
              "Theme.TextPrimary", box textPrimary
              "Theme.TextPrimaryBrush", box textPrimary
              "Theme.TextMuted", box textMuted
              "Theme.TextMutedBrush", box textMuted
              "Theme.Selection", box selection
              "Theme.SelectionBrush", box selection
              "Theme.Hover", box hover
              "Theme.HoverBrush", box hover
              "Theme.Pressed", box pressed
              "Theme.PressedBrush", box pressed
              "Theme.Disabled", box disabled
              "Theme.DisabledBrush", box disabled
              "Theme.Error", box error
              "Theme.ErrorBrush", box error
              "Theme.Warning", box warning
              "Theme.WarningBrush", box warning
              "Theme.Information", box information
              "Theme.InformationBrush", box information
              "Theme.Separator", box separatorBrush
              "Theme.SeparatorBrush", box separatorBrush
              "Theme.Border", box separatorBrush
              "Theme.BorderBrush", box separatorBrush
              "Theme.Focus", box focus
              "Theme.FocusBrush", box focus
              "Theme.Accent", box focus
              "Theme.AccentBrush", box focus
              "Theme.ResizeHandle", box resizeHandleBrush
              "Theme.Shadow", box shadow ]
