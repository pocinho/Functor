namespace Functor.Avalonia.ViewModels

open System
open System.Globalization
open Functor.Application
open Functor.Rendering

type SettingsViewModel(initialSettings: AppSettings) =
    inherit ViewModelBase()

    let palette = initialSettings.Theme.ThemeSource.Resolve()
    let colorText color = sprintf "#%08X" color
    let mutable background = colorText palette.Background
    let mutable foreground = colorText palette.Foreground
    let mutable selection = colorText palette.Selection
    let mutable cursor = colorText palette.Cursor
    let mutable lineNumber = colorText palette.LineNumber
    let mutable gutterBackground = colorText palette.GutterBackground
    let mutable diagnosticError = colorText palette.DiagnosticError
    let mutable diagnosticWarning = colorText palette.DiagnosticWarning
    let mutable diagnosticInfo = colorText palette.DiagnosticInfo
    let mutable editorBorder = palette.EditorBorder |> Option.map colorText |> Option.defaultValue ""
    let mutable editorBorderWidth = palette.EditorBorderWidth.ToString(CultureInfo.InvariantCulture)
    let mutable errorMessage = ""

    let parseColor name (value: string) =
        let normalized = value.Trim().TrimStart('#')
        let normalized = if normalized.Length = 6 then "FF" + normalized else normalized
        let mutable parsed = 0u

        if UInt32.TryParse(normalized, NumberStyles.HexNumber, CultureInfo.InvariantCulture, &parsed) then
            Ok parsed
        else
            Error(sprintf "%s must be a 6- or 8-digit hex color." name)

    let parseOptionalColor name (value: string) =
        if String.IsNullOrWhiteSpace value then
            Ok None
        else
            parseColor name value |> Result.map Some

    let bind next result = Result.bind next result

    member this.Background
        with get () = background
        and set value = this.SetProperty(&background, value) |> ignore

    member this.Foreground
        with get () = foreground
        and set value = this.SetProperty(&foreground, value) |> ignore

    member this.Selection
        with get () = selection
        and set value = this.SetProperty(&selection, value) |> ignore

    member this.Cursor
        with get () = cursor
        and set value = this.SetProperty(&cursor, value) |> ignore

    member this.LineNumber
        with get () = lineNumber
        and set value = this.SetProperty(&lineNumber, value) |> ignore

    member this.GutterBackground
        with get () = gutterBackground
        and set value = this.SetProperty(&gutterBackground, value) |> ignore

    member this.DiagnosticError
        with get () = diagnosticError
        and set value = this.SetProperty(&diagnosticError, value) |> ignore

    member this.DiagnosticWarning
        with get () = diagnosticWarning
        and set value = this.SetProperty(&diagnosticWarning, value) |> ignore

    member this.DiagnosticInfo
        with get () = diagnosticInfo
        and set value = this.SetProperty(&diagnosticInfo, value) |> ignore

    member this.EditorBorder
        with get () = editorBorder
        and set value = this.SetProperty(&editorBorder, value) |> ignore

    member this.EditorBorderWidth
        with get () = editorBorderWidth
        and set value = this.SetProperty(&editorBorderWidth, value) |> ignore

    member this.ErrorMessage
        with get () = errorMessage
        and set value = this.SetProperty(&errorMessage, value) |> ignore

    member _.TryCreateSettings() =
        Ok Theme.defaultPalette
        |> bind (fun palette -> parseColor "Background" background |> Result.map (fun value -> { palette with Background = value }))
        |> bind (fun palette -> parseColor "Foreground" foreground |> Result.map (fun value -> { palette with Foreground = value }))
        |> bind (fun palette -> parseColor "Selection" selection |> Result.map (fun value -> { palette with Selection = value }))
        |> bind (fun palette -> parseColor "Cursor" cursor |> Result.map (fun value -> { palette with Cursor = value }))
        |> bind (fun palette -> parseColor "Line number" lineNumber |> Result.map (fun value -> { palette with LineNumber = value }))
        |> bind (fun palette -> parseColor "Gutter background" gutterBackground |> Result.map (fun value -> { palette with GutterBackground = value }))
        |> bind (fun palette -> parseColor "Diagnostic error" diagnosticError |> Result.map (fun value -> { palette with DiagnosticError = value }))
        |> bind (fun palette -> parseColor "Diagnostic warning" diagnosticWarning |> Result.map (fun value -> { palette with DiagnosticWarning = value }))
        |> bind (fun palette -> parseColor "Diagnostic info" diagnosticInfo |> Result.map (fun value -> { palette with DiagnosticInfo = value }))
        |> bind (fun palette -> parseOptionalColor "Editor border" editorBorder |> Result.map (fun value -> { palette with EditorBorder = value }))
        |> bind (fun palette ->
            let mutable width = 0.0f

            if Single.TryParse(editorBorderWidth, NumberStyles.Float, CultureInfo.InvariantCulture, &width) && width >= 0.0f then
                Ok { palette with EditorBorderWidth = width }
            else
                Error "Editor border width must be a non-negative number.")
        |> Result.map (ThemeSettings.fromPalette >> AppSettings.fromTheme)