namespace Functor.Application

/// UI defaults that remain themeable without being tied to Avalonia types.
type UiThemeDefaults =
    { EditorFontFamily: string
      EditorFallbackFontFamily: string
      IconFontFamily: string
      TabCloseIconSize: float
      WorkspaceFontSize: float
      EditorLineHeight: float
      EditorTabSize: int
      CursorWidth: float
      GutterSeparatorWidth: float
      GutterPadding: float
      GutterMinimumWidth: float
      CommandPaletteFontSize: float
      WelcomeTitleFontSize: float
      ControlCornerRadius: float
      ResizeHandleColor: uint32
      CommandPaletteShadowColor: uint32
      WorkspaceSeparatorColor: uint32
      MeasurementColor: uint32 }

module UiThemeDefaults =
    let defaultTheme =
        { EditorFontFamily = "Consolas"
          EditorFallbackFontFamily = "Segoe UI Emoji"
          IconFontFamily = "Segoe MDL2 Assets"
          TabCloseIconSize = 9.0
          WorkspaceFontSize = 12.0
          EditorLineHeight = 16.0
          EditorTabSize = 4
          CursorWidth = 1.5
          GutterSeparatorWidth = 1.0
          GutterPadding = 4.0
          GutterMinimumWidth = 16.0
          CommandPaletteFontSize = 14.0
          WelcomeTitleFontSize = 28.0
          ControlCornerRadius = 0.0
          ResizeHandleColor = 0xDCDC3C3Cu
          CommandPaletteShadowColor = 0x66000000u
          WorkspaceSeparatorColor = 0x6EA0A0A0u
          MeasurementColor = 0xFFFFFFFFu }
