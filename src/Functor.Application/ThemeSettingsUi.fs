namespace Functor.Application

open System
open System.Text.Json
open ThemeSettingsJson

module ThemeSettingsUi =
    let private applyString element name update ui =
        match tryString element name with
        | None -> Ok ui
        | Some value -> Ok(update value ui)

    let private applyFloat element name update ui =
        match tryFloat32 element name with
        | None -> Ok ui
        | Some value when Single.IsFinite value -> Ok(update value ui)
        | Some _ -> Error(sprintf "Invalid UI theme value: %s" name)

    let private applyPositiveFloat element name update ui =
        match tryFloat32 element name with
        | None -> Ok ui
        | Some value when Single.IsFinite value && value > 0.0f -> Ok(update value ui)
        | Some _ -> Error(sprintf "Invalid UI theme value: %s" name)

    let private applyNonNegativeFloat element name update ui =
        match tryFloat32 element name with
        | None -> Ok ui
        | Some value when Single.IsFinite value && value >= 0.0f -> Ok(update value ui)
        | Some _ -> Error(sprintf "Invalid UI theme value: %s" name)

    let private applyOpacity element name update ui =
        match tryFloat32 element name with
        | None -> Ok ui
        | Some value when Single.IsFinite value && value >= 0.0f && value <= 1.0f -> Ok(update value ui)
        | Some _ -> Error(sprintf "Invalid UI theme value: %s" name)

    let private applyColor element name update ui =
        match tryString element name with
        | None -> Ok ui
        | Some value -> parseColor value |> Result.map (fun color -> update color ui)

    let apply (theme: JsonElement) preset =
        Ok(ThemePreset.ui preset)
        |> Result.bind (applyString theme "editorFontFamily" (fun value ui -> { ui with EditorFontFamily = value }))
        |> Result.bind (
            applyString theme "editorFallbackFontFamily" (fun value ui ->
                { ui with
                    EditorFallbackFontFamily = value })
        )
        |> Result.bind (applyString theme "uiFontFamily" (fun value ui -> { ui with UiFontFamily = value }))
        |> Result.bind (applyString theme "iconFontFamily" (fun value ui -> { ui with IconFontFamily = value }))
        |> Result.bind (
            applyPositiveFloat theme "tabCloseIconSize" (fun value ui ->
                { ui with
                    TabCloseIconSize = float value })
        )
        |> Result.bind (
            applyFloat theme "workspaceFontSize" (fun value ui ->
                { ui with
                    WorkspaceFontSize = float value })
        )
        |> Result.bind (
            applyPositiveFloat theme "editorFontSize" (fun value ui -> { ui with EditorFontSize = float value })
        )
        |> Result.bind (
            applyFloat theme "editorLineHeight" (fun value ui ->
                { ui with
                    EditorLineHeight = float value })
        )
        |> Result.bind (fun ui ->
            match tryInt32 theme "editorTabSize" with
            | None -> Ok ui
            | Some value when value > 0 -> Ok { ui with EditorTabSize = value }
            | Some _ -> Error "Invalid UI theme value: editorTabSize")
        |> Result.bind (applyPositiveFloat theme "cursorWidth" (fun value ui -> { ui with CursorWidth = float value }))
        |> Result.bind (
            applyPositiveFloat theme "gutterSeparatorWidth" (fun value ui ->
                { ui with
                    GutterSeparatorWidth = float value })
        )
        |> Result.bind (
            applyPositiveFloat theme "gutterPadding" (fun value ui -> { ui with GutterPadding = float value })
        )
        |> Result.bind (
            applyPositiveFloat theme "gutterMinimumWidth" (fun value ui ->
                { ui with
                    GutterMinimumWidth = float value })
        )
        |> Result.bind (
            applyPositiveFloat theme "documentTabMinHeight" (fun value ui ->
                { ui with
                    DocumentTabMinHeight = float value })
        )
        |> Result.bind (
            applyPositiveFloat theme "documentTabCloseButtonSize" (fun value ui ->
                { ui with
                    DocumentTabCloseButtonSize = float value })
        )
        |> Result.bind (
            applyPositiveFloat theme "documentTabPaddingHorizontal" (fun value ui ->
                { ui with
                    DocumentTabPaddingHorizontal = float value })
        )
        |> Result.bind (
            applyPositiveFloat theme "documentTabPaddingVertical" (fun value ui ->
                { ui with
                    DocumentTabPaddingVertical = float value })
        )
        |> Result.bind (
            applyPositiveFloat theme "documentTabSpacing" (fun value ui ->
                { ui with
                    DocumentTabSpacing = float value })
        )
        |> Result.bind (
            applyFloat theme "commandPaletteFontSize" (fun value ui ->
                { ui with
                    CommandPaletteFontSize = float value })
        )
        |> Result.bind (
            applyPositiveFloat theme "commandPaletteWidth" (fun value ui ->
                { ui with
                    CommandPaletteWidth = float value })
        )
        |> Result.bind (
            applyNonNegativeFloat theme "commandPaletteTopMargin" (fun value ui ->
                { ui with
                    CommandPaletteTopMargin = float value })
        )
        |> Result.bind (
            applyNonNegativeFloat theme "commandPalettePadding" (fun value ui ->
                { ui with
                    CommandPalettePadding = float value })
        )
        |> Result.bind (
            applyPositiveFloat theme "commandPaletteMaxHeight" (fun value ui ->
                { ui with
                    CommandPaletteMaxHeight = float value })
        )
        |> Result.bind (
            applyNonNegativeFloat theme "commandPaletteItemMarginHorizontal" (fun value ui ->
                { ui with
                    CommandPaletteItemMarginHorizontal = float value })
        )
        |> Result.bind (
            applyNonNegativeFloat theme "commandPaletteItemMarginVertical" (fun value ui ->
                { ui with
                    CommandPaletteItemMarginVertical = float value })
        )
        |> Result.bind (
            applyNonNegativeFloat theme "commandPaletteGestureMargin" (fun value ui ->
                { ui with
                    CommandPaletteGestureMargin = float value })
        )
        |> Result.bind (
            applyFloat theme "welcomeTitleFontSize" (fun value ui ->
                { ui with
                    WelcomeTitleFontSize = float value })
        )
        |> Result.bind (
            applyOpacity theme "textMutedOpacity" (fun value ui ->
                { ui with
                    TextMutedOpacity = float value })
        )
        |> Result.bind (
            applyFloat theme "controlCornerRadius" (fun value ui ->
                { ui with
                    ControlCornerRadius = float value })
        )
        |> Result.bind (applyColor theme "resizeHandleColor" (fun value ui -> { ui with ResizeHandleColor = value }))
        |> Result.bind (
            applyPositiveFloat theme "sidePanelResizeHandleWidth" (fun value ui ->
                { ui with
                    SidePanelResizeHandleWidth = float value })
        )
        |> Result.bind (
            applyColor theme "commandPaletteShadowColor" (fun value ui ->
                { ui with
                    CommandPaletteShadowColor = value })
        )
        |> Result.bind (
            applyColor theme "workspaceSeparatorColor" (fun value ui ->
                { ui with
                    WorkspaceSeparatorColor = value })
        )
        |> Result.bind (applyColor theme "measurementColor" (fun value ui -> { ui with MeasurementColor = value }))
