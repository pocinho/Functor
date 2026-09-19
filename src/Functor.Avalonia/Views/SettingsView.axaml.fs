namespace Functor.Avalonia.Views

open System
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
    let fontCombo name = this.FindControl<ComboBox>(name)
    let colorPicker name = this.FindControl<ColorPicker>(name)

    let setText (control: TextBox) value = control.Text <- value
    let setFontText (control: ComboBox) value = control.Text <- value

    let tryParseColor (value: string) =
        let normalized = value.Trim().TrimStart('#')

        if normalized.Length <> 6 && normalized.Length <> 8 then
            None
        else
            let normalized =
                if normalized.Length = 6 then
                    "FF" + normalized
                else
                    normalized

            let mutable parsed = 0u

            if
                UInt32.TryParse(
                    normalized,
                    Globalization.NumberStyles.HexNumber,
                    Globalization.CultureInfo.InvariantCulture,
                    &parsed
                )
            then
                Some(Color.FromArgb(byte (parsed >>> 24), byte (parsed >>> 16), byte (parsed >>> 8), byte parsed))
            else
                None

    let colorText (color: Color) =
        sprintf "#%02X%02X%02X%02X" color.A color.R color.G color.B

    let setColorPicker (picker: ColorPicker) value =
        match tryParseColor value with
        | Some color ->
            picker.Color <- color
            picker.IsVisible <- true
        | None -> picker.IsVisible <- false

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

        let fontFamilies =
            FontManager.Current.SystemFonts
            |> Seq.map (fun family -> family.Name)
            |> Seq.distinct
            |> Seq.sort
            |> Seq.toArray

        for name in
            [ "EditorFontFamily"
              "EditorFallbackFontFamily"
              "UiFontFamily"
              "IconFontFamily" ] do
            (fontCombo name).ItemsSource <- fontFamilies

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

        let updateColorField textName pickerName update =
            let picker = colorPicker pickerName

            picker.ColorChanged.Add(fun args ->
                if not updatingControls then
                    let value = colorText args.NewColor
                    setText (textBox textName) value
                    setColorPicker picker value
                    updateDraft (fun current -> update value current))

        updateColorField "Background" "BackgroundPicker" (fun value current -> { current with Background = value })
        updateColorField "Foreground" "ForegroundPicker" (fun value current -> { current with Foreground = value })
        updateColorField "Selection" "SelectionPicker" (fun value current -> { current with Selection = value })
        updateColorField "Cursor" "CursorPicker" (fun value current -> { current with Cursor = value })
        updateColorField "LineNumber" "LineNumberPicker" (fun value current -> { current with LineNumber = value })

        updateColorField "GutterBackground" "GutterBackgroundPicker" (fun value current ->
            { current with
                GutterBackground = value })

        updateColorField "DiagnosticError" "DiagnosticErrorPicker" (fun value current ->
            { current with DiagnosticError = value })

        updateColorField "DiagnosticWarning" "DiagnosticWarningPicker" (fun value current ->
            { current with
                DiagnosticWarning = value })

        updateColorField "DiagnosticInfo" "DiagnosticInfoPicker" (fun value current ->
            { current with DiagnosticInfo = value })

        updateColorField "EditorBorder" "EditorBorderPicker" (fun value current ->
            { current with EditorBorder = value })

        updateColorField "SyntaxKeyword" "SyntaxKeywordPicker" (fun value current ->
            { current with SyntaxKeyword = value })

        updateColorField "SyntaxString" "SyntaxStringPicker" (fun value current ->
            { current with SyntaxString = value })

        updateColorField "SyntaxComment" "SyntaxCommentPicker" (fun value current ->
            { current with SyntaxComment = value })

        updateColorField "SyntaxNumber" "SyntaxNumberPicker" (fun value current ->
            { current with SyntaxNumber = value })

        updateColorField "SyntaxType" "SyntaxTypePicker" (fun value current -> { current with SyntaxType = value })

        updateColorField "SyntaxFunction" "SyntaxFunctionPicker" (fun value current ->
            { current with SyntaxFunction = value })

        updateColorField "ResizeHandleColor" "ResizeHandleColorPicker" (fun value current ->
            { current with
                ResizeHandleColor = value })

        updateColorField "CommandPaletteShadowColor" "CommandPaletteShadowColorPicker" (fun value current ->
            { current with
                CommandPaletteShadowColor = value })

        updateColorField "WorkspaceSeparatorColor" "WorkspaceSeparatorColorPicker" (fun value current ->
            { current with
                WorkspaceSeparatorColor = value })

        updateColorField "MeasurementColor" "MeasurementColorPicker" (fun value current ->
            { current with
                MeasurementColor = value })

        updateTextField "EditorBorderWidth" (fun value current ->
            { current with
                EditorBorderWidth = value })

        let updateFontField name update =
            let control = fontCombo name

            control
                .GetObservable(ComboBox.TextProperty)
                .Subscribe(fun _ -> updateDraft (fun current -> update control.Text current))
            |> ignore

        updateFontField "EditorFontFamily" (fun value current ->
            { current with
                EditorFontFamily = value })

        updateFontField "EditorFallbackFontFamily" (fun value current ->
            { current with
                EditorFallbackFontFamily = value })

        updateFontField "UiFontFamily" (fun value current -> { current with UiFontFamily = value })

        updateFontField "IconFontFamily" (fun value current -> { current with IconFontFamily = value })

        updateTextField "TabCloseIconSize" (fun value current ->
            { current with
                TabCloseIconSize = value })

        updateTextField "WorkspaceFontSize" (fun value current ->
            { current with
                WorkspaceFontSize = value })

        updateTextField "EditorFontSize" (fun value current -> { current with EditorFontSize = value })

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

        updateTextField "TextMutedOpacity" (fun value current ->
            { current with
                TextMutedOpacity = value })

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
            setColorPicker (colorPicker "BackgroundPicker") value.Background
            setText (textBox "Foreground") value.Foreground
            setColorPicker (colorPicker "ForegroundPicker") value.Foreground
            setText (textBox "Selection") value.Selection
            setColorPicker (colorPicker "SelectionPicker") value.Selection
            setText (textBox "Cursor") value.Cursor
            setColorPicker (colorPicker "CursorPicker") value.Cursor
            setText (textBox "LineNumber") value.LineNumber
            setColorPicker (colorPicker "LineNumberPicker") value.LineNumber
            setText (textBox "GutterBackground") value.GutterBackground
            setColorPicker (colorPicker "GutterBackgroundPicker") value.GutterBackground
            setText (textBox "DiagnosticError") value.DiagnosticError
            setColorPicker (colorPicker "DiagnosticErrorPicker") value.DiagnosticError
            setText (textBox "DiagnosticWarning") value.DiagnosticWarning
            setColorPicker (colorPicker "DiagnosticWarningPicker") value.DiagnosticWarning
            setText (textBox "DiagnosticInfo") value.DiagnosticInfo
            setColorPicker (colorPicker "DiagnosticInfoPicker") value.DiagnosticInfo
            setText (textBox "SyntaxKeyword") value.SyntaxKeyword
            setColorPicker (colorPicker "SyntaxKeywordPicker") value.SyntaxKeyword
            setText (textBox "SyntaxString") value.SyntaxString
            setColorPicker (colorPicker "SyntaxStringPicker") value.SyntaxString
            setText (textBox "SyntaxComment") value.SyntaxComment
            setColorPicker (colorPicker "SyntaxCommentPicker") value.SyntaxComment
            setText (textBox "SyntaxNumber") value.SyntaxNumber
            setColorPicker (colorPicker "SyntaxNumberPicker") value.SyntaxNumber
            setText (textBox "SyntaxType") value.SyntaxType
            setColorPicker (colorPicker "SyntaxTypePicker") value.SyntaxType
            setText (textBox "SyntaxFunction") value.SyntaxFunction
            setColorPicker (colorPicker "SyntaxFunctionPicker") value.SyntaxFunction
            setText (textBox "EditorBorder") value.EditorBorder
            setColorPicker (colorPicker "EditorBorderPicker") value.EditorBorder
            setText (textBox "EditorBorderWidth") value.EditorBorderWidth
            setFontText (fontCombo "EditorFontFamily") value.EditorFontFamily
            setFontText (fontCombo "EditorFallbackFontFamily") value.EditorFallbackFontFamily
            setFontText (fontCombo "UiFontFamily") value.UiFontFamily
            setFontText (fontCombo "IconFontFamily") value.IconFontFamily
            setText (textBox "TabCloseIconSize") value.TabCloseIconSize
            setText (textBox "WorkspaceFontSize") value.WorkspaceFontSize
            setText (textBox "EditorFontSize") value.EditorFontSize
            setText (textBox "EditorLineHeight") value.EditorLineHeight
            setText (textBox "EditorTabSize") value.EditorTabSize
            setText (textBox "CursorWidth") value.CursorWidth
            setText (textBox "GutterSeparatorWidth") value.GutterSeparatorWidth
            setText (textBox "CommandPaletteFontSize") value.CommandPaletteFontSize
            setText (textBox "WelcomeTitleFontSize") value.WelcomeTitleFontSize
            setText (textBox "TextMutedOpacity") value.TextMutedOpacity
            setText (textBox "ControlCornerRadius") value.ControlCornerRadius
            setText (textBox "ResizeHandleColor") value.ResizeHandleColor
            setColorPicker (colorPicker "ResizeHandleColorPicker") value.ResizeHandleColor
            setText (textBox "CommandPaletteShadowColor") value.CommandPaletteShadowColor
            setColorPicker (colorPicker "CommandPaletteShadowColorPicker") value.CommandPaletteShadowColor
            setText (textBox "WorkspaceSeparatorColor") value.WorkspaceSeparatorColor
            setColorPicker (colorPicker "WorkspaceSeparatorColorPicker") value.WorkspaceSeparatorColor
            setText (textBox "MeasurementColor") value.MeasurementColor
            setColorPicker (colorPicker "MeasurementColorPicker") value.MeasurementColor
            updatingControls <- false
        | None -> ()

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
