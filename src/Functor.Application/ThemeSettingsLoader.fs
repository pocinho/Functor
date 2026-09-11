namespace Functor.Application

open System
open System.Globalization
open System.Text.Json
open Functor.Rendering

module ThemeSettingsLoader =
    let private parseColor (value: string) =
        let normalized = value.Trim().TrimStart('#')
        let normalized = if normalized.Length = 6 then "FF" + normalized else normalized
        let mutable parsed = 0u

        if UInt32.TryParse(normalized, NumberStyles.HexNumber, CultureInfo.InvariantCulture, &parsed) then
            Ok parsed
        else
            Error(sprintf "Invalid theme color: %s" value)

    let private tryString (element: JsonElement) (name: string) =
        let mutable property = Unchecked.defaultof<JsonElement>

        if element.TryGetProperty(name, &property) && property.ValueKind = JsonValueKind.String then
            property.GetString() |> Option.ofObj
        else
            None

    let private tryFloat32 (element: JsonElement) (name: string) =
        let mutable property = Unchecked.defaultof<JsonElement>

        if element.TryGetProperty(name, &property) && property.ValueKind = JsonValueKind.Number then
            match property.TryGetSingle() with
            | true, value -> Some value
            | _ -> None
        else
            None

    let private applyColor element name update palette =
        match tryString element name with
        | None -> Ok palette
        | Some value ->
            parseColor value
            |> Result.map (fun color -> update color palette)

    let loadText (json: string) : Result<ThemeSettings, string> =
        try
            use document = JsonDocument.Parse(json)
            let root = document.RootElement
            let mutable themeElement = Unchecked.defaultof<JsonElement>

            let theme =
                if root.TryGetProperty("theme", &themeElement) && themeElement.ValueKind = JsonValueKind.Object then
                    themeElement
                else
                    root

            let preset = tryString theme "preset" |> Option.defaultValue "Graphite Dark"
            let basePalette = if preset = "Graphite Light" then Theme.graphiteLight else Theme.defaultPalette

            let result =
                basePalette
                |> applyColor theme "background" (fun value palette -> { palette with Background = value })
                |> Result.bind (applyColor theme "foreground" (fun value palette -> { palette with Foreground = value }))
                |> Result.bind (applyColor theme "selection" (fun value palette -> { palette with Selection = value }))
                |> Result.bind (applyColor theme "cursor" (fun value palette -> { palette with Cursor = value }))
                |> Result.bind (applyColor theme "lineNumber" (fun value palette -> { palette with LineNumber = value }))
                |> Result.bind (applyColor theme "gutterBackground" (fun value palette -> { palette with GutterBackground = value }))
                |> Result.bind (applyColor theme "diagnosticError" (fun value palette -> { palette with DiagnosticError = value }))
                |> Result.bind (applyColor theme "diagnosticWarning" (fun value palette -> { palette with DiagnosticWarning = value }))
                |> Result.bind (applyColor theme "diagnosticInfo" (fun value palette -> { palette with DiagnosticInfo = value }))

            result
            |> Result.map (fun palette ->
                let border =
                    match tryString theme "editorBorder" with
                    | None -> basePalette.EditorBorder
                    | Some value -> parseColor value |> Result.toOption

                let width = tryFloat32 theme "editorBorderWidth" |> Option.defaultValue basePalette.EditorBorderWidth

                ThemeSettings.fromPaletteWithPreset preset
                    { palette with
                        EditorBorder = border
                        EditorBorderWidth = width })
        with ex ->
            Error ex.Message

    let loadFile path =
        try
            System.IO.File.ReadAllText(path) |> loadText
        with ex ->
            Error ex.Message
