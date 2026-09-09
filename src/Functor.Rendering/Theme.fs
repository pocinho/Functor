namespace Functor.Rendering

/// Small, backend-neutral theme palette used by render backends.
/// Values are stored as 32-bit ARGB colors so they remain portable across UI stacks.
type ThemePalette =
    { Background: uint32
      Foreground: uint32
      Selection: uint32
      Cursor: uint32
      LineNumber: uint32
      GutterBackground: uint32
      GutterSeparator: uint32 option
      EditorBorder: uint32 option
      EditorBorderWidth: float32
      DiagnosticError: uint32
      DiagnosticWarning: uint32
      DiagnosticInfo: uint32 }

type ThemeSource =
    { Resolve: unit -> ThemePalette }

module Theme =
    let dark =
        { Background = 0xFF111111u
          Foreground = 0xFFE6E6E6u
          Selection = 0x5A4A82D9u
          Cursor = 0xFFE6E6E6u
          LineNumber = 0xFF7A7A7Au
          GutterBackground = 0xFF111111u
          GutterSeparator = Some 0xFF333333u
          EditorBorder = None
          EditorBorderWidth = 0.0f
          DiagnosticError = 0xFFFF5C5Cu
          DiagnosticWarning = 0xFFFFC857u
          DiagnosticInfo = 0xFF5CC8FFu }

    let defaultPalette = dark
    let defaultSource = { Resolve = fun () -> defaultPalette }

module ThemeSource =
    let fixedPalette (palette: ThemePalette) = { Resolve = fun () -> palette }
    let dark = fixedPalette Theme.defaultPalette
    let custom (factory: unit -> ThemePalette) = { Resolve = factory }
