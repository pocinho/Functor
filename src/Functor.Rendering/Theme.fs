namespace Functor.Rendering

/// Small, backend-neutral theme palette used by render backends.
/// Values are stored as 32-bit ARGB colors so they remain portable across UI stacks.
type ThemePalette =
    { Background: uint32; Foreground: uint32; Selection: uint32; Cursor: uint32; LineNumber: uint32; GutterBackground: uint32; GutterSeparator: uint32 option; EditorBorder: uint32 option; EditorBorderWidth: float32; DiagnosticError: uint32; DiagnosticWarning: uint32; DiagnosticInfo: uint32; SyntaxColors: Map<string, uint32> }

type ThemeSource =
    { Resolve: unit -> ThemePalette }

module Theme =
    let graphiteLight =
        { Background = 0xFFE7E5EAu; Foreground = 0xFF24212Bu; Selection = 0x604E3A78u; Cursor = 0xFF4E3A78u; LineNumber = 0xFF77707Fu; GutterBackground = 0xFFDCD9E1u; GutterSeparator = Some 0xFFB9B3C2u; EditorBorder = Some 0xFFB9B3C2u; EditorBorderWidth = 1.0f; DiagnosticError = 0xFFB83B5Eu; DiagnosticWarning = 0xFF9A6B18u; DiagnosticInfo = 0xFF356B8Cu; SyntaxColors = Map.ofList [ "keyword", 0xFF633F9Au; "string", 0xFF8C4B4Bu; "comment", 0xFF59705Au; "number", 0xFF3B6871u; "type", 0xFF356B8Cu; "function", 0xFF7A4F8Cu ] }

    let dark =
        { Background = 0xFF17151Bu; Foreground = 0xFFE7E3EDu; Selection = 0x705E4A8Fu; Cursor = 0xFFD8C9F0u; LineNumber = 0xFF8F8799u; GutterBackground = 0xFF1E1B24u; GutterSeparator = Some 0xFF393241u; EditorBorder = Some 0xFF393241u; EditorBorderWidth = 1.0f; DiagnosticError = 0xFFFF7185u; DiagnosticWarning = 0xFFE4B45Du; DiagnosticInfo = 0xFF72C5E8u; SyntaxColors = Map.ofList [ "keyword", 0xFFC7A6F7u; "string", 0xFFE5A58Fu; "comment", 0xFF7CA889u; "number", 0xFF9DD6C8u; "type", 0xFF7CC4C7u; "function", 0xFFE1B1E8u ] }

    let defaultPalette = dark
    let defaultSource = { Resolve = fun () -> defaultPalette }

    let resolveTextColor palette role =
        match role with
        | EditorForeground -> palette.Foreground
        | SyntaxForeground kind -> palette.SyntaxColors |> Map.tryFind kind |> Option.defaultValue palette.Foreground

module ThemeSource =
    let fixedPalette (palette: ThemePalette) = { Resolve = fun () -> palette }
    let dark = fixedPalette Theme.defaultPalette
    let light = fixedPalette Theme.graphiteLight
    let custom (factory: unit -> ThemePalette) = { Resolve = factory }
