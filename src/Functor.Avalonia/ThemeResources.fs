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
        let typography = ThemeTypography.fromUiTheme uiTheme
        let shape = ThemeShapeDensity.fromUiTheme uiTheme
        let layout = ThemeLayoutDensity.fromUiTheme uiTheme
        let resizeHandleBrush = brushFromArgb uiTheme.ResizeHandleColor
        let workspaceSeparatorBrush = brushFromArgb uiTheme.WorkspaceSeparatorColor
        let shadow = commandPaletteShadow uiTheme.CommandPaletteShadowColor

        setItems
            resources
            [ ("Theme.EditorFontFamily", box typography.EditorFontFamily)
              ("Theme.EditorFallbackFontFamily", box typography.EditorFallbackFontFamily)
              ("Theme.ControlFontFamily", box (FontFamily(typography.UiFontFamily)))
              ("Theme.IconFontFamily", box (FontFamily(typography.IconFontFamily)))
              ("Theme.TabCloseIconSize", box uiTheme.TabCloseIconSize)
              ("Theme.DocumentTabMinHeight", box shape.DocumentTabMinHeight)
              ("Theme.DocumentTabPadding",
               box (Thickness(shape.DocumentTabPaddingHorizontal, shape.DocumentTabPaddingVertical)))
              ("Theme.DocumentTabSpacing", box shape.DocumentTabSpacing)
              ("Theme.TabNavigationButtonWidth", box shape.TabNavigationButtonWidth)
              ("Theme.TitleBarHeight", box shape.TitleBarHeight)
              ("Theme.WindowControlWidth", box shape.WindowControlWidth)
              ("Theme.WindowControlPadding", box (Thickness(shape.WindowControlPadding)))
              ("Theme.CommandBarWidth", box shape.CommandBarWidth)
              ("Theme.MinimumWindowWidth", box shape.MinimumWindowWidth)
              ("Theme.CommandBarHeight", box shape.CommandBarHeight)
              ("Theme.TitleBarHorizontalPadding", box (Thickness(shape.TitleBarHorizontalPadding, 0.0)))
              ("Theme.CommandPaletteWidth", box shape.CommandPaletteWidth)
              ("Theme.CommandPaletteOverlayMargin", box (Thickness(0.0, shape.CommandPaletteTopMargin, 0.0, 0.0)))
              ("Theme.CommandPalettePadding", box (Thickness(shape.CommandPalettePadding)))
              ("Theme.CommandPaletteMaxHeight", box shape.CommandPaletteMaxHeight)
              ("Theme.CommandPaletteItemMargin",
               box (Thickness(shape.CommandPaletteItemMarginHorizontal, shape.CommandPaletteItemMarginVertical)))
              ("Theme.CommandPaletteGestureMargin", box (Thickness(shape.CommandPaletteGestureMargin, 0.0, 0.0, 0.0)))
              ("Theme.SidePanelResizeHandleWidth", box shape.SidePanelResizeHandleWidth)
              ("Theme.SidePanelRightPadding", box (Thickness(0.0, 0.0, shape.SidePanelRightPadding, 0.0)))
              ("Theme.TabBarPadding", box (Thickness(layout.TabBarPaddingHorizontal, layout.TabBarPaddingVertical)))
              ("Theme.ToolRailPadding", box (Thickness(layout.ToolRailPadding)))
              ("Theme.ToolRailSpacing", box layout.ToolRailSpacing)
              ("Theme.SettingsFooterPadding", box (Thickness(layout.SettingsFooterPadding)))
              ("Theme.SettingsFooterSpacing", box layout.SettingsFooterSpacing)
              ("Theme.StatusBarPadding",
               box (Thickness(layout.StatusBarPaddingHorizontal, layout.StatusBarPaddingVertical)))
              ("Theme.StatusTextGap", box (Thickness(layout.StatusTextGap, 0.0, 0.0, 0.0)))
              ("Theme.StatusDirtyGap", box (Thickness(layout.StatusDirtyGap, 0.0, 0.0, 0.0)))
              ("Theme.WelcomeWidth", box layout.WelcomeWidth)
              ("Theme.WelcomeSectionSpacing", box layout.WelcomeSectionSpacing)
              ("Theme.WelcomeActionSpacing", box layout.WelcomeActionSpacing)
              ("Theme.EditorFontSize", box typography.EditorFontSize)
              ("Theme.EditorLineHeight", box typography.EditorLineHeight)
              ("Theme.EditorTabSize", box uiTheme.EditorTabSize)
              ("Theme.CursorWidth", box uiTheme.CursorWidth)
              ("Theme.GutterSeparatorWidth", box uiTheme.GutterSeparatorWidth)
              ("Theme.WorkspaceFontSize", box typography.WorkspaceFontSize)
              ("Theme.CommandPaletteFontSize", box typography.CommandPaletteFontSize)
              ("Theme.WelcomeTitleFontSize", box typography.WelcomeTitleFontSize)
              ("Theme.FontWeightBold", box (fontWeight typography.BoldFontWeight))
              ("Theme.FontWeightSemiBold", box (fontWeight typography.SemiBoldFontWeight))
              ("Theme.BorderThickness", box (Thickness(shape.BorderWidth)))
              ("Theme.SeparatorBottomThickness", box (Thickness(0, 0, 0, shape.SeparatorWidth)))
              ("Theme.SeparatorRightThickness", box (Thickness(0, 0, shape.SeparatorWidth, 0)))
              ("Theme.SeparatorTopThickness", box (Thickness(0, shape.SeparatorWidth, 0, 0)))
              ("Theme.TextMutedOpacity", box uiTheme.TextMutedOpacity)
              ("Theme.ControlCornerRadius", box (CornerRadius(shape.ControlCornerRadius)))
              ("Theme.ResizeHandleColor", box uiTheme.ResizeHandleColor)
              ("Theme.ResizeHandleBrush", box resizeHandleBrush)
              ("Theme.CommandPaletteShadowColor", box uiTheme.CommandPaletteShadowColor)
              ("Theme.CommandPaletteShadow", box shadow)
              ("Theme.WorkspaceSeparatorColor", box uiTheme.WorkspaceSeparatorColor)
              ("Theme.WorkspaceSeparatorBrush", box workspaceSeparatorBrush)
              ("Theme.MeasurementColor", box uiTheme.MeasurementColor) ]

    let applyPalette (resources: IResourceDictionary) (uiTheme: UiThemeDefaults) (palette: ThemePalette) =
        let semantic = ThemeSemanticColors.fromPalette uiTheme palette
        let surfaceBackground = brushFromArgb semantic.SurfaceBackground
        let panelBackground = brushFromArgb semantic.PanelBackground
        let inputBackground = brushFromArgb semantic.InputBackground
        let textPrimary = brushFromArgb semantic.TextPrimary
        let textMuted = brushFromArgb semantic.TextMuted
        let selection = brushFromArgb semantic.Selection
        let hover = brushFromArgb semantic.Hover
        let pressed = brushFromArgb semantic.Pressed
        let disabled = brushFromArgb semantic.Disabled
        let error = brushFromArgb semantic.Error
        let warning = brushFromArgb semantic.Warning
        let information = brushFromArgb semantic.Information
        let separatorBrush = brushFromArgb semantic.Separator
        let focus = brushFromArgb semantic.Focus
        let resizeHandleBrush = brushFromArgb semantic.ResizeHandle
        let shadow = commandPaletteShadow semantic.Shadow

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
