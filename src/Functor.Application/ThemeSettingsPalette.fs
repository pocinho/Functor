namespace Functor.Application

open System.Text.Json
open Functor.Rendering
open ThemeSettingsJson

module ThemeSettingsPalette =
    let private applyColor element name update palette =
        match tryString element name with
        | None -> Ok palette
        | Some value -> parseColor value |> Result.map (fun color -> update color palette)

    let apply (theme: JsonElement) preset =
        let basePalette = ThemePreset.palette preset

        basePalette
        |> applyColor theme "background" (fun value palette -> { palette with Background = value })
        |> Result.bind (applyColor theme "foreground" (fun value palette -> { palette with Foreground = value }))
        |> Result.bind (applyColor theme "selection" (fun value palette -> { palette with Selection = value }))
        |> Result.bind (applyColor theme "cursor" (fun value palette -> { palette with Cursor = value }))
        |> Result.bind (applyColor theme "lineNumber" (fun value palette -> { palette with LineNumber = value }))
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
        |> Result.map (fun palette ->
            let border =
                match tryString theme "editorBorder" with
                | None -> basePalette.EditorBorder
                | Some value -> parseColor value |> Result.toOption

            let width =
                tryFloat32 theme "editorBorderWidth"
                |> Option.defaultValue basePalette.EditorBorderWidth

            { palette with
                EditorBorder = border
                EditorBorderWidth = width })
