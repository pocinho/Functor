namespace Functor.Application

open System
open System.Globalization
open System.Text.Json
open Functor.Rendering

module ThemeSettingsLoader =
    let private currentSchemaVersion = 1

    let private parseColor (value: string) =
        let normalized = value.Trim().TrimStart('#')

        let normalized =
            if normalized.Length = 6 then
                "FF" + normalized
            else
                normalized

        let mutable parsed = 0u

        if UInt32.TryParse(normalized, NumberStyles.HexNumber, CultureInfo.InvariantCulture, &parsed) then
            Ok parsed
        else
            Error(sprintf "Invalid theme color: %s" value)

    let private tryString (element: JsonElement) (name: string) =
        let mutable property = Unchecked.defaultof<JsonElement>

        if
            element.TryGetProperty(name, &property)
            && property.ValueKind = JsonValueKind.String
        then
            property.GetString() |> Option.ofObj
        else
            None

    let private tryFloat32 (element: JsonElement) (name: string) =
        let mutable property = Unchecked.defaultof<JsonElement>

        if
            element.TryGetProperty(name, &property)
            && property.ValueKind = JsonValueKind.Number
        then
            match property.TryGetSingle() with
            | true, value -> Some value
            | _ -> None
        else
            None

    let private tryInt32 (element: JsonElement) (name: string) =
        let mutable property = Unchecked.defaultof<JsonElement>

        if
            element.TryGetProperty(name, &property)
            && property.ValueKind = JsonValueKind.Number
        then
            match property.TryGetInt32() with
            | true, value -> Some value
            | _ -> None
        else
            None

    let private applyColor element name update palette =
        match tryString element name with
        | None -> Ok palette
        | Some value -> parseColor value |> Result.map (fun color -> update color palette)

    let private applyUiString element name update ui =
        match tryString element name with
        | None -> Ok ui
        | Some value -> Ok(update value ui)

    let private applyUiFloat element name update ui =
        match tryFloat32 element name with
        | None -> Ok ui
        | Some value when Single.IsFinite value -> Ok(update value ui)
        | Some _ -> Error(sprintf "Invalid UI theme value: %s" name)

    let private applyUiPositiveFloat element name update ui =
        match tryFloat32 element name with
        | None -> Ok ui
        | Some value when Single.IsFinite value && value > 0.0f -> Ok(update value ui)
        | Some _ -> Error(sprintf "Invalid UI theme value: %s" name)

    let private applyUiOpacity element name update ui =
        match tryFloat32 element name with
        | None -> Ok ui
        | Some value when Single.IsFinite value && value >= 0.0f && value <= 1.0f -> Ok(update value ui)
        | Some _ -> Error(sprintf "Invalid UI theme value: %s" name)

    let private applyUiInt element name update ui =
        match tryInt32 element name with
        | None -> Ok ui
        | Some value -> Ok(update value ui)

    let private applyUiColor element name update ui =
        match tryString element name with
        | None -> Ok ui
        | Some value -> parseColor value |> Result.map (fun color -> update color ui)

    let loadText (json: string) : Result<ThemeSettings, string> =
        try
            use document = JsonDocument.Parse(json)
            let root = document.RootElement
            let mutable themeElement = Unchecked.defaultof<JsonElement>

            let theme =
                if
                    root.TryGetProperty("theme", &themeElement)
                    && themeElement.ValueKind = JsonValueKind.Object
                then
                    themeElement
                else
                    root

            let mutable schemaVersionElement = Unchecked.defaultof<JsonElement>

            if not (theme.TryGetProperty("schemaVersion", &schemaVersionElement)) then
                failwith "Theme schema warning: missing schemaVersion; theme was not loaded"

            if schemaVersionElement.ValueKind <> JsonValueKind.Number then
                failwith "Invalid theme schema version: expected an integer"

            match schemaVersionElement.TryGetInt32() with
            | false, _ -> failwith "Invalid theme schema version: expected an integer"
            | true, version when version > currentSchemaVersion ->
                failwith (sprintf "Unsupported theme schema version: %d" version)
            | true, _ -> ()

            let preset =
                tryString theme "preset" |> Option.defaultValue ThemePreset.GraphiteDark

            let basePalette =
                if preset = ThemePreset.GraphiteLight then
                    Theme.graphiteLight
                else
                    Theme.defaultPalette

            let result =
                basePalette
                |> applyColor theme "background" (fun value palette -> { palette with Background = value })
                |> Result.bind (
                    applyColor theme "foreground" (fun value palette -> { palette with Foreground = value })
                )
                |> Result.bind (applyColor theme "selection" (fun value palette -> { palette with Selection = value }))
                |> Result.bind (applyColor theme "cursor" (fun value palette -> { palette with Cursor = value }))
                |> Result.bind (
                    applyColor theme "lineNumber" (fun value palette -> { palette with LineNumber = value })
                )
                |> Result.bind (
                    applyColor theme "gutterBackground" (fun value palette ->
                        { palette with
                            GutterBackground = value })
                )
                |> Result.bind (
                    applyColor theme "diagnosticError" (fun value palette -> { palette with DiagnosticError = value })
                )
                |> Result.bind (
                    applyColor theme "diagnosticWarning" (fun value palette ->
                        { palette with
                            DiagnosticWarning = value })
                )
                |> Result.bind (
                    applyColor theme "diagnosticInfo" (fun value palette -> { palette with DiagnosticInfo = value })
                )
                |> Result.bind (
                    applyColor theme "syntaxKeyword" (fun value palette ->
                        { palette with
                            SyntaxColors = palette.SyntaxColors.Add("keyword", value) })
                )
                |> Result.bind (
                    applyColor theme "syntaxString" (fun value palette ->
                        { palette with
                            SyntaxColors = palette.SyntaxColors.Add("string", value) })
                )
                |> Result.bind (
                    applyColor theme "syntaxComment" (fun value palette ->
                        { palette with
                            SyntaxColors = palette.SyntaxColors.Add("comment", value) })
                )
                |> Result.bind (
                    applyColor theme "syntaxNumber" (fun value palette ->
                        { palette with
                            SyntaxColors = palette.SyntaxColors.Add("number", value) })
                )
                |> Result.bind (
                    applyColor theme "syntaxType" (fun value palette ->
                        { palette with
                            SyntaxColors = palette.SyntaxColors.Add("type", value) })
                )
                |> Result.bind (
                    applyColor theme "syntaxFunction" (fun value palette ->
                        { palette with
                            SyntaxColors = palette.SyntaxColors.Add("function", value) })
                )

            result
            |> Result.bind (fun palette ->
                let border =
                    match tryString theme "editorBorder" with
                    | None -> basePalette.EditorBorder
                    | Some value -> parseColor value |> Result.toOption

                let width =
                    tryFloat32 theme "editorBorderWidth"
                    |> Option.defaultValue basePalette.EditorBorderWidth

                let palette =
                    { palette with
                        EditorBorder = border
                        EditorBorderWidth = width }

                let uiResult =
                    Ok UiThemeDefaults.defaultTheme
                    |> Result.bind (
                        applyUiString theme "editorFontFamily" (fun value ui -> { ui with EditorFontFamily = value })
                    )
                    |> Result.bind (
                        applyUiString theme "editorFallbackFontFamily" (fun value ui ->
                            { ui with
                                EditorFallbackFontFamily = value })
                    )
                    |> Result.bind (
                        applyUiString theme "uiFontFamily" (fun value ui -> { ui with UiFontFamily = value })
                    )
                    |> Result.bind (
                        applyUiString theme "iconFontFamily" (fun value ui -> { ui with IconFontFamily = value })
                    )
                    |> Result.bind (
                        applyUiPositiveFloat theme "tabCloseIconSize" (fun value ui ->
                            { ui with
                                TabCloseIconSize = float value })
                    )
                    |> Result.bind (
                        applyUiFloat theme "workspaceFontSize" (fun value ui ->
                            { ui with
                                WorkspaceFontSize = float value })
                    )
                    |> Result.bind (
                        applyUiPositiveFloat theme "editorFontSize" (fun value ui ->
                            { ui with EditorFontSize = float value })
                    )
                    |> Result.bind (
                        applyUiFloat theme "editorLineHeight" (fun value ui ->
                            { ui with
                                EditorLineHeight = float value })
                    )
                    |> Result.bind (fun ui ->
                        match tryInt32 theme "editorTabSize" with
                        | None -> Ok ui
                        | Some value when value > 0 -> Ok { ui with EditorTabSize = value }
                        | Some _ -> Error "Invalid UI theme value: editorTabSize")
                    |> Result.bind (
                        applyUiPositiveFloat theme "cursorWidth" (fun value ui ->
                            { ui with CursorWidth = float value })
                    )
                    |> Result.bind (
                        applyUiPositiveFloat theme "gutterSeparatorWidth" (fun value ui ->
                            { ui with
                                GutterSeparatorWidth = float value })
                    )
                    |> Result.bind (
                        applyUiPositiveFloat theme "gutterPadding" (fun value ui ->
                            { ui with GutterPadding = float value })
                    )
                    |> Result.bind (
                        applyUiPositiveFloat theme "gutterMinimumWidth" (fun value ui ->
                            { ui with
                                GutterMinimumWidth = float value })
                    )
                    |> Result.bind (
                        applyUiPositiveFloat theme "documentTabMinHeight" (fun value ui ->
                            { ui with
                                DocumentTabMinHeight = float value })
                    )
                    |> Result.bind (
                        applyUiPositiveFloat theme "documentTabCloseButtonSize" (fun value ui ->
                            { ui with
                                DocumentTabCloseButtonSize = float value })
                    )
                    |> Result.bind (
                        applyUiPositiveFloat theme "documentTabPaddingHorizontal" (fun value ui ->
                            { ui with
                                DocumentTabPaddingHorizontal = float value })
                    )
                    |> Result.bind (
                        applyUiPositiveFloat theme "documentTabPaddingVertical" (fun value ui ->
                            { ui with
                                DocumentTabPaddingVertical = float value })
                    )
                    |> Result.bind (
                        applyUiPositiveFloat theme "documentTabSpacing" (fun value ui ->
                            { ui with
                                DocumentTabSpacing = float value })
                    )
                    |> Result.bind (
                        applyUiFloat theme "commandPaletteFontSize" (fun value ui ->
                            { ui with
                                CommandPaletteFontSize = float value })
                    )
                    |> Result.bind (
                        applyUiFloat theme "welcomeTitleFontSize" (fun value ui ->
                            { ui with
                                WelcomeTitleFontSize = float value })
                    )
                    |> Result.bind (
                        applyUiOpacity theme "textMutedOpacity" (fun value ui ->
                            { ui with
                                TextMutedOpacity = float value })
                    )
                    |> Result.bind (
                        applyUiFloat theme "controlCornerRadius" (fun value ui ->
                            { ui with
                                ControlCornerRadius = float value })
                    )
                    |> Result.bind (
                        applyUiColor theme "resizeHandleColor" (fun value ui -> { ui with ResizeHandleColor = value })
                    )
                    |> Result.bind (
                        applyUiColor theme "commandPaletteShadowColor" (fun value ui ->
                            { ui with
                                CommandPaletteShadowColor = value })
                    )
                    |> Result.bind (
                        applyUiColor theme "workspaceSeparatorColor" (fun value ui ->
                            { ui with
                                WorkspaceSeparatorColor = value })
                    )
                    |> Result.bind (
                        applyUiColor theme "measurementColor" (fun value ui -> { ui with MeasurementColor = value })
                    )

                uiResult |> Result.map (ThemeSettings.fromPaletteWithPresetAndUi preset palette))
        with ex ->
            Error ex.Message

    let loadFile path =
        try
            System.IO.File.ReadAllText(path) |> loadText
        with ex ->
            Error ex.Message
