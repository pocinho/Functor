namespace Functor.Avalonia

open Avalonia
open Avalonia.Controls
open Avalonia.Media
open Functor.Application
open Functor.Rendering

module ThemeResources =
    let private colorFromArgb (argb: uint32) =
        Color.FromArgb(byte (argb >>> 24), byte (argb >>> 16), byte (argb >>> 8), byte argb)

    let private brushFromArgb argb = SolidColorBrush(colorFromArgb argb)

    let private commandPaletteShadow argb =
        BoxShadows.Parse(sprintf "0 12 32 0 #%08X" argb)

    let applyUi (resources: IResourceDictionary) (uiTheme: UiThemeDefaults) =
        resources["Theme.EditorFontFamily"] <- uiTheme.EditorFontFamily
        resources["Theme.EditorFallbackFontFamily"] <- uiTheme.EditorFallbackFontFamily
        resources["Theme.ControlFontFamily"] <- FontFamily(uiTheme.EditorFontFamily)
        resources["Theme.IconFontFamily"] <- FontFamily(uiTheme.IconFontFamily)
        resources["Theme.TabCloseIconSize"] <- uiTheme.TabCloseIconSize
        resources["Theme.EditorLineHeight"] <- uiTheme.EditorLineHeight
        resources["Theme.EditorTabSize"] <- uiTheme.EditorTabSize
        resources["Theme.CursorWidth"] <- uiTheme.CursorWidth
        resources["Theme.GutterSeparatorWidth"] <- uiTheme.GutterSeparatorWidth
        resources["Theme.WorkspaceFontSize"] <- uiTheme.WorkspaceFontSize
        resources["Theme.CommandPaletteFontSize"] <- uiTheme.CommandPaletteFontSize
        resources["Theme.WelcomeTitleFontSize"] <- uiTheme.WelcomeTitleFontSize
        resources["Theme.ControlCornerRadius"] <- CornerRadius(uiTheme.ControlCornerRadius)
        resources["Theme.ResizeHandleColor"] <- uiTheme.ResizeHandleColor
        resources["Theme.ResizeHandleBrush"] <- brushFromArgb uiTheme.ResizeHandleColor
        resources["Theme.CommandPaletteShadowColor"] <- uiTheme.CommandPaletteShadowColor
        resources["Theme.CommandPaletteShadow"] <- commandPaletteShadow uiTheme.CommandPaletteShadowColor
        resources["Theme.WorkspaceSeparatorColor"] <- uiTheme.WorkspaceSeparatorColor
        resources["Theme.WorkspaceSeparatorBrush"] <- brushFromArgb uiTheme.WorkspaceSeparatorColor
        resources["Theme.MeasurementColor"] <- uiTheme.MeasurementColor

    let applyPalette (resources: IResourceDictionary) (palette: ThemePalette) =
        resources["Theme.SurfaceBackgroundBrush"] <- brushFromArgb palette.Background
        resources["Theme.PanelBackgroundBrush"] <- brushFromArgb palette.GutterBackground
        resources["Theme.InputBackgroundBrush"] <- brushFromArgb palette.GutterBackground
        resources["Theme.TextPrimaryBrush"] <- brushFromArgb palette.Foreground
        resources["Theme.TextMutedBrush"] <- brushFromArgb palette.LineNumber
        resources["Theme.SelectionBrush"] <- brushFromArgb palette.Selection
        resources["Theme.HoverBrush"] <- brushFromArgb palette.Selection
        resources["Theme.PressedBrush"] <- brushFromArgb palette.Cursor
        resources["Theme.DisabledBrush"] <- brushFromArgb palette.LineNumber
        resources["Theme.ErrorBrush"] <- brushFromArgb palette.DiagnosticError
        resources["Theme.WarningBrush"] <- brushFromArgb palette.DiagnosticWarning
        resources["Theme.InformationBrush"] <- brushFromArgb palette.DiagnosticInfo

        let separator = palette.GutterSeparator |> Option.defaultValue palette.Foreground
        resources["Theme.SeparatorBrush"] <- brushFromArgb separator
        resources["Theme.BorderBrush"] <- brushFromArgb separator

        resources["Theme.FocusBrush"] <- brushFromArgb palette.Cursor
        resources["Theme.AccentBrush"] <- brushFromArgb palette.Cursor
