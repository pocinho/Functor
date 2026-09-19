namespace Functor.Avalonia.Views

open Avalonia
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Media
open Avalonia.Markup.Xaml
open Functor.Application
open Functor.Rendering

type SettingsView() as this =
    inherit UserControl()

    let mutable draft = None
    let mutable updatingControls = false

    let textBox name = this.FindControl<TextBox>(name)

    let setText (control: TextBox) value = control.Text <- value

    let updateDraft update =
        if not updatingControls then
            draft <- draft |> Option.map update

    let updateTextField name update =
        let control = textBox name
        control.TextChanged.Add(fun _ -> updateDraft (fun current -> update control.Text current))

    do
        this.InitializeComponent()

        let preset = this.FindControl<ComboBox>("ThemePreset")
        preset.ItemsSource <- [ "Graphite Dark"; "Graphite Light"; "Custom" ]

        preset.SelectionChanged.Add(fun _ ->
            if not updatingControls then
                match preset.SelectedItem with
                | :? string as value when value <> "Custom" ->
                    draft <- draft |> Option.map (SettingsDraft.applyPreset value)
                    this.RefreshControls()
                | :? string as value -> updateDraft (fun current -> { current with ThemePreset = value })
                | _ -> ())

        updateTextField "Background" (fun value current -> { current with Background = value })
        updateTextField "Foreground" (fun value current -> { current with Foreground = value })
        updateTextField "Selection" (fun value current -> { current with Selection = value })
        updateTextField "Cursor" (fun value current -> { current with Cursor = value })
        updateTextField "LineNumber" (fun value current -> { current with LineNumber = value })

        updateTextField "GutterBackground" (fun value current ->
            { current with
                GutterBackground = value })

        updateTextField "DiagnosticError" (fun value current -> { current with DiagnosticError = value })

        updateTextField "DiagnosticWarning" (fun value current ->
            { current with
                DiagnosticWarning = value })

        updateTextField "DiagnosticInfo" (fun value current -> { current with DiagnosticInfo = value })

        updateTextField "SyntaxKeyword" (fun value current -> { current with SyntaxKeyword = value })
        updateTextField "SyntaxString" (fun value current -> { current with SyntaxString = value })
        updateTextField "SyntaxComment" (fun value current -> { current with SyntaxComment = value })
        updateTextField "SyntaxNumber" (fun value current -> { current with SyntaxNumber = value })
        updateTextField "SyntaxType" (fun value current -> { current with SyntaxType = value })
        updateTextField "SyntaxFunction" (fun value current -> { current with SyntaxFunction = value })

        updateTextField "EditorBorder" (fun value current -> { current with EditorBorder = value })

        updateTextField "EditorBorderWidth" (fun value current ->
            { current with
                EditorBorderWidth = value })

        updateTextField "EditorFontFamily" (fun value current ->
            { current with
                EditorFontFamily = value })

        updateTextField "EditorFallbackFontFamily" (fun value current ->
            { current with
                EditorFallbackFontFamily = value })

        updateTextField "IconFontFamily" (fun value current -> { current with IconFontFamily = value })

        updateTextField "WorkspaceFontSize" (fun value current ->
            { current with
                WorkspaceFontSize = value })

        updateTextField "EditorLineHeight" (fun value current ->
            { current with
                EditorLineHeight = value })

        updateTextField "EditorTabSize" (fun value current -> { current with EditorTabSize = value })

        updateTextField "CursorWidth" (fun value current -> { current with CursorWidth = value })

        updateTextField "GutterSeparatorWidth" (fun value current ->
            { current with
                GutterSeparatorWidth = value })

        updateTextField "CommandPaletteFontSize" (fun value current ->
            { current with
                CommandPaletteFontSize = value })

        updateTextField "WelcomeTitleFontSize" (fun value current ->
            { current with
                WelcomeTitleFontSize = value })

        updateTextField "ControlCornerRadius" (fun value current ->
            { current with
                ControlCornerRadius = value })

        updateTextField "ResizeHandleColor" (fun value current ->
            { current with
                ResizeHandleColor = value })

        updateTextField "CommandPaletteShadowColor" (fun value current ->
            { current with
                CommandPaletteShadowColor = value })

        updateTextField "WorkspaceSeparatorColor" (fun value current ->
            { current with
                WorkspaceSeparatorColor = value })

        updateTextField "MeasurementColor" (fun value current ->
            { current with
                MeasurementColor = value })

    member this.Configure(settings: AppSettings) =
        draft <- Some(SettingsDraft.fromSettings settings)
        this.RefreshControls()

    member _.Draft = draft

    member this.SetError(error: string) =
        this.FindControl<TextBlock>("ErrorText").Text <- error

    member private this.RefreshControls() =
        match draft with
        | Some value ->
            updatingControls <- true
            let preset = this.FindControl<ComboBox>("ThemePreset")
            preset.SelectedItem <- value.ThemePreset
            setText (textBox "Background") value.Background
            setText (textBox "Foreground") value.Foreground
            setText (textBox "Selection") value.Selection
            setText (textBox "Cursor") value.Cursor
            setText (textBox "LineNumber") value.LineNumber
            setText (textBox "GutterBackground") value.GutterBackground
            setText (textBox "DiagnosticError") value.DiagnosticError
            setText (textBox "DiagnosticWarning") value.DiagnosticWarning
            setText (textBox "DiagnosticInfo") value.DiagnosticInfo
            setText (textBox "SyntaxKeyword") value.SyntaxKeyword
            setText (textBox "SyntaxString") value.SyntaxString
            setText (textBox "SyntaxComment") value.SyntaxComment
            setText (textBox "SyntaxNumber") value.SyntaxNumber
            setText (textBox "SyntaxType") value.SyntaxType
            setText (textBox "SyntaxFunction") value.SyntaxFunction
            setText (textBox "EditorBorder") value.EditorBorder
            setText (textBox "EditorBorderWidth") value.EditorBorderWidth
            setText (textBox "EditorFontFamily") value.EditorFontFamily
            setText (textBox "EditorFallbackFontFamily") value.EditorFallbackFontFamily
            setText (textBox "IconFontFamily") value.IconFontFamily
            setText (textBox "WorkspaceFontSize") value.WorkspaceFontSize
            setText (textBox "EditorLineHeight") value.EditorLineHeight
            setText (textBox "EditorTabSize") value.EditorTabSize
            setText (textBox "CursorWidth") value.CursorWidth
            setText (textBox "GutterSeparatorWidth") value.GutterSeparatorWidth
            setText (textBox "CommandPaletteFontSize") value.CommandPaletteFontSize
            setText (textBox "WelcomeTitleFontSize") value.WelcomeTitleFontSize
            setText (textBox "ControlCornerRadius") value.ControlCornerRadius
            setText (textBox "ResizeHandleColor") value.ResizeHandleColor
            setText (textBox "CommandPaletteShadowColor") value.CommandPaletteShadowColor
            setText (textBox "WorkspaceSeparatorColor") value.WorkspaceSeparatorColor
            setText (textBox "MeasurementColor") value.MeasurementColor
            updatingControls <- false
        | None -> ()

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
