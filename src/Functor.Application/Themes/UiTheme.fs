namespace Functor.Application

open Functor.Rendering

/// UI defaults that remain themeable without being tied to Avalonia types.
type UiThemeDefaults =
    { EditorFontFamily: string
      EditorFallbackFontFamily: string
      UiFontFamily: string
      IconFontFamily: string
      TabCloseIconSize: float
      WorkspaceFontSize: float
      EditorFontSize: float
      EditorLineHeight: float
      EditorTabSize: int
      CursorWidth: float
      GutterSeparatorWidth: float
      GutterPadding: float
      GutterMinimumWidth: float
      DocumentTabMinHeight: float
      DocumentTabCloseButtonSize: float
      DocumentTabPaddingHorizontal: float
      DocumentTabPaddingVertical: float
      DocumentTabSpacing: float
      CommandPaletteFontSize: float
      CommandPaletteWidth: float
      CommandPaletteTopMargin: float
      CommandPalettePadding: float
      CommandPaletteMaxHeight: float
      CommandPaletteItemMarginHorizontal: float
      CommandPaletteItemMarginVertical: float
      CommandPaletteGestureMargin: float
      WelcomeTitleFontSize: float
      TextMutedOpacity: float
      ControlCornerRadius: float
      ResizeHandleColor: uint32
      SidePanelResizeHandleWidth: float
      CommandPaletteShadowColor: uint32
      WorkspaceSeparatorColor: uint32
      MeasurementColor: uint32 }

type ThemeTypography =
    { EditorFontFamily: string
      EditorFallbackFontFamily: string
      UiFontFamily: string
      IconFontFamily: string
      TabCloseIconSize: float
      WorkspaceFontSize: float
      EditorFontSize: float
      EditorLineHeight: float
      CommandPaletteFontSize: float
      WelcomeTitleFontSize: float
      BoldFontWeight: int
      SemiBoldFontWeight: int }

module ThemeTypography =
    let fromUiTheme (uiTheme: UiThemeDefaults) =
        { EditorFontFamily = uiTheme.EditorFontFamily
          EditorFallbackFontFamily = uiTheme.EditorFallbackFontFamily
          UiFontFamily = uiTheme.UiFontFamily
          IconFontFamily = uiTheme.IconFontFamily
          TabCloseIconSize = uiTheme.TabCloseIconSize
          WorkspaceFontSize = uiTheme.WorkspaceFontSize
          EditorFontSize = uiTheme.EditorFontSize
          EditorLineHeight = uiTheme.EditorLineHeight
          CommandPaletteFontSize = uiTheme.CommandPaletteFontSize
          WelcomeTitleFontSize = uiTheme.WelcomeTitleFontSize
          BoldFontWeight = 700
          SemiBoldFontWeight = 600 }

type ThemeSemanticColors =
    { SurfaceBackground: uint32
      PanelBackground: uint32
      InputBackground: uint32
      TextPrimary: uint32
      TextMuted: uint32
      Border: uint32
      Separator: uint32
      Accent: uint32
      Selection: uint32
      Hover: uint32
      Pressed: uint32
      Disabled: uint32
      Focus: uint32
      Error: uint32
      Warning: uint32
      Information: uint32
      ResizeHandle: uint32
      Shadow: uint32 }

module ThemeSemanticColors =
    let fromPalette (uiTheme: UiThemeDefaults) (palette: ThemePalette) =
        let separator = palette.GutterSeparator |> Option.defaultValue palette.Foreground

        { SurfaceBackground = palette.Background
          PanelBackground = palette.GutterBackground
          InputBackground = palette.GutterBackground
          TextPrimary = palette.Foreground
          TextMuted = palette.LineNumber
          Border = separator
          Separator = separator
          Accent = palette.Cursor
          Selection = palette.Selection
          Hover = palette.Selection
          Pressed = palette.Cursor
          Disabled = palette.LineNumber
          Focus = palette.Cursor
          Error = palette.DiagnosticError
          Warning = palette.DiagnosticWarning
          Information = palette.DiagnosticInfo
          ResizeHandle = uiTheme.ResizeHandleColor
          Shadow = uiTheme.CommandPaletteShadowColor }

module UiThemeDefaults =
    let boldFontWeight = 700
    let semiBoldFontWeight = 600
    let separatorWidth = 1.0

    let defaultTheme =
        { EditorFontFamily = "Consolas"
          EditorFallbackFontFamily = "Segoe UI Emoji"
          UiFontFamily = "Segoe UI"
          IconFontFamily = "Segoe MDL2 Assets"
          TabCloseIconSize = 9.0
          WorkspaceFontSize = 12.0
          EditorFontSize = 16.0 * (5.0 / 6.0)
          EditorLineHeight = 16.0
          EditorTabSize = 4
          CursorWidth = 1.5
          GutterSeparatorWidth = 1.0
          GutterPadding = 4.0
          GutterMinimumWidth = 16.0
          DocumentTabMinHeight = 28.0
          DocumentTabCloseButtonSize = 22.0
          DocumentTabPaddingHorizontal = 10.0
          DocumentTabPaddingVertical = 4.0
          DocumentTabSpacing = 2.0
          CommandPaletteFontSize = 14.0
          CommandPaletteWidth = 560.0
          CommandPaletteTopMargin = 8.0
          CommandPalettePadding = 10.0
          CommandPaletteMaxHeight = 260.0
          CommandPaletteItemMarginHorizontal = 8.0
          CommandPaletteItemMarginVertical = 6.0
          CommandPaletteGestureMargin = 16.0
          WelcomeTitleFontSize = 28.0
          TextMutedOpacity = 0.68
          ControlCornerRadius = 0.0
          ResizeHandleColor = 0xDCDC3C3Cu
          SidePanelResizeHandleWidth = 1.0
          CommandPaletteShadowColor = 0x66000000u
          WorkspaceSeparatorColor = 0x6EA0A0A0u
          MeasurementColor = 0xFFFFFFFFu }

type ThemeShapeDensity =
    { DocumentTabMinHeight: float
      DocumentTabCloseButtonSize: float
      DocumentTabPaddingHorizontal: float
      DocumentTabPaddingVertical: float
      DocumentTabSpacing: float
      DocumentTabContentSpacing: float
      TabNavigationButtonWidth: float
      WorkspaceTreeIndent: float
      WorkspaceTreeCollapseButtonSize: float
      WorkspaceRowSpacing: float
      TitleBarHeight: float
      WindowControlWidth: float
      WindowControlPadding: float
      CommandBarWidth: float
      MinimumWindowWidth: float
      CommandBarHeight: float
      TitleBarHorizontalPadding: float
      CommandPaletteWidth: float
      CommandPaletteTopMargin: float
      CommandPalettePadding: float
      CommandPaletteMaxHeight: float
      CommandPaletteItemMarginHorizontal: float
      CommandPaletteItemMarginVertical: float
      CommandPaletteGestureMargin: float
      SidePanelResizeHandleWidth: float
      SidePanelRightPadding: float
      BorderWidth: float
      SeparatorWidth: float
      ControlCornerRadius: float }

module ThemeShapeDensity =
    let fromUiTheme (uiTheme: UiThemeDefaults) =
        { DocumentTabMinHeight = uiTheme.DocumentTabMinHeight
          DocumentTabCloseButtonSize = uiTheme.DocumentTabCloseButtonSize
          DocumentTabPaddingHorizontal = uiTheme.DocumentTabPaddingHorizontal
          DocumentTabPaddingVertical = uiTheme.DocumentTabPaddingVertical
          DocumentTabSpacing = uiTheme.DocumentTabSpacing
          DocumentTabContentSpacing = 8.0
          TabNavigationButtonWidth = 28.0
          WorkspaceTreeIndent = 14.0
          WorkspaceTreeCollapseButtonSize = 18.0
          WorkspaceRowSpacing = 0.0
          TitleBarHeight = 32.0
          WindowControlWidth = 46.0
          WindowControlPadding = 2.0
          CommandBarWidth = 360.0
          MinimumWindowWidth = 360.0 + (46.0 * 3.0) + 102.0
          CommandBarHeight = 24.0
          TitleBarHorizontalPadding = 12.0
          CommandPaletteWidth = uiTheme.CommandPaletteWidth
          CommandPaletteTopMargin = uiTheme.CommandPaletteTopMargin
          CommandPalettePadding = uiTheme.CommandPalettePadding
          CommandPaletteMaxHeight = uiTheme.CommandPaletteMaxHeight
          CommandPaletteItemMarginHorizontal = uiTheme.CommandPaletteItemMarginHorizontal
          CommandPaletteItemMarginVertical = uiTheme.CommandPaletteItemMarginVertical
          CommandPaletteGestureMargin = uiTheme.CommandPaletteGestureMargin
          SidePanelResizeHandleWidth = uiTheme.SidePanelResizeHandleWidth
          SidePanelRightPadding = 8.0
          BorderWidth = 1.0
          SeparatorWidth = UiThemeDefaults.separatorWidth
          ControlCornerRadius = uiTheme.ControlCornerRadius }

type ThemeLayoutDensity =
    { TabBarPaddingHorizontal: float
      TabBarPaddingVertical: float
      ToolRailPadding: float
      ToolRailSpacing: float
      SettingsFooterPadding: float
      SettingsFooterSpacing: float
      StatusBarPaddingHorizontal: float
      StatusBarPaddingVertical: float
      StatusTextGap: float
      StatusDirtyGap: float
      WelcomeWidth: float
      WelcomeSectionSpacing: float
      WelcomeActionSpacing: float
      DialogContentPadding: float
      DialogContentSpacing: float
      DialogButtonSpacing: float
      SettingsPagePadding: float
      SettingsPageSpacing: float
      SettingsSectionHeadingTopSpacing: float
      SettingsSectionHeadingBottomSpacing: float
      SettingsControlWidth: float
      SettingsRowMarginBottom: float
      SettingsColorPickerWidth: float
      SettingsColorPickerHeight: float }

module ThemeLayoutDensity =
    let fromUiTheme (_uiTheme: UiThemeDefaults) =
        { TabBarPaddingHorizontal = 4.0
          TabBarPaddingVertical = 3.0
          ToolRailPadding = 3.0
          ToolRailSpacing = 4.0
          SettingsFooterPadding = 12.0
          SettingsFooterSpacing = 8.0
          StatusBarPaddingHorizontal = 8.0
          StatusBarPaddingVertical = 4.0
          StatusTextGap = 20.0
          StatusDirtyGap = 12.0
          WelcomeWidth = 320.0
          WelcomeSectionSpacing = 16.0
          WelcomeActionSpacing = 8.0
          DialogContentPadding = 16.0
          DialogContentSpacing = 16.0
          DialogButtonSpacing = 8.0
          SettingsPagePadding = 16.0
          SettingsPageSpacing = 4.0
          SettingsSectionHeadingTopSpacing = 12.0
          SettingsSectionHeadingBottomSpacing = 8.0
          SettingsControlWidth = 200.0
          SettingsRowMarginBottom = 8.0
          SettingsColorPickerWidth = 64.0
          SettingsColorPickerHeight = 32.0 }
