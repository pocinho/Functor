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
      EditorBorder: string
      EditorBorderWidth: string }

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
          EditorBorder = palette.EditorBorder |> Option.map colorText |> Option.defaultValue ""
          EditorBorderWidth = palette.EditorBorderWidth.ToString(CultureInfo.InvariantCulture) }

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
        |> Result.map (ThemeSettings.fromPaletteWithPreset draft.ThemePreset >> AppSettings.fromTheme)
