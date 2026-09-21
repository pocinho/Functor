namespace Functor.Avalonia.Views

open System
open System.Globalization
open Avalonia
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Media
open Avalonia.Markup.Xaml
open Functor.Application
open Functor.Platform
open Functor.Rendering

type SettingsView() as this =
    inherit UserControl()

    let mutable form = None
    let mutable themeFiles: ThemeFile list = []
    let mutable selectedThemeName: string option = None
    let mutable themeName = ThemePreset.GraphiteDark
    let mutable updatingControls = false

    let textBox name = this.FindControl<TextBox>(name)
    let numericUpDown name = this.FindControl<NumericUpDown>(name)
    let fontCombo name = this.FindControl<ComboBox>(name)
    let colorPicker name = this.FindControl<ColorPicker>(name)

    let setText (control: TextBox) value = control.Text <- value

    let setNumericText (control: NumericUpDown) (value: string) =
        let mutable parsed = 0M

        if Decimal.TryParse(value, NumberStyles.Any, CultureInfo.InvariantCulture, &parsed) then
            control.Value <- Nullable parsed
        else
            control.Value <- Nullable()

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
            form <- form |> Option.map update

    let updateTextField name update =
        let control = textBox name
        control.TextChanged.Add(fun _ -> updateDraft (fun current -> update control.Text current))

    let updateColorTextField textName pickerName update =
        let text = textBox textName
        let picker = colorPicker pickerName

        text.TextChanged.Add(fun _ ->
            if not updatingControls then
                setColorPicker picker text.Text
                updateDraft (fun current -> update text.Text current))

    let updateNumericField name update =
        let control = numericUpDown name

        control
            .GetObservable(NumericUpDown.ValueProperty)
            .Subscribe(fun value ->
                if not updatingControls then
                    let text =
                        if value.HasValue then
                            value.Value.ToString(CultureInfo.InvariantCulture)
                        else
                            ""

                    updateDraft (fun current -> update text current))
        |> ignore

    let tryPositiveFloat (value: string) =
        let mutable parsed = 0.0

        if
            not (String.IsNullOrWhiteSpace value)
            && Double.TryParse(
                value,
                Globalization.NumberStyles.Any,
                Globalization.CultureInfo.InvariantCulture,
                &parsed
            )
            && parsed > 0.0
        then
            Some parsed
        else
            None

    let formatFloat (value: float) =
        value.ToString(Globalization.CultureInfo.InvariantCulture)

    do
        this.InitializeComponent()

        let preset = this.FindControl<ComboBox>("ThemePreset")
        preset.ItemsSource <- ThemePreset.all

        let themeNameControl = textBox "ThemeName"

        themeNameControl.TextChanged.Add(fun _ ->
            if not updatingControls then
                themeName <- themeNameControl.Text)

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
                | :? string as value ->
                    match themeFiles |> List.tryFind (fun theme -> theme.Name = value) with
                    | Some theme ->
                        selectedThemeName <- Some value
                        themeName <- value
                        form <- Some(SettingsForm.fromAppSettingsWithPreset theme.Settings.Theme.Preset theme.Settings)
                        this.RefreshControls()
                    | None ->
                        selectedThemeName <- None
                        themeName <- value
                        form <- form |> Option.map (SettingsForm.applyPreset value)
                        this.RefreshControls()
                | _ -> ())

        updateColorTextField "Background" "BackgroundPicker" (fun value current -> { current with Background = value })
        updateColorTextField "Foreground" "ForegroundPicker" (fun value current -> { current with Foreground = value })
        updateColorTextField "Selection" "SelectionPicker" (fun value current -> { current with Selection = value })
        updateColorTextField "Cursor" "CursorPicker" (fun value current -> { current with Cursor = value })
        updateColorTextField "LineNumber" "LineNumberPicker" (fun value current -> { current with LineNumber = value })

        updateColorTextField "GutterBackground" "GutterBackgroundPicker" (fun value current ->
            { current with
                GutterBackground = value })

        updateColorTextField "DiagnosticError" "DiagnosticErrorPicker" (fun value current ->
            { current with DiagnosticError = value })

        updateColorTextField "DiagnosticWarning" "DiagnosticWarningPicker" (fun value current ->
            { current with
                DiagnosticWarning = value })

        updateColorTextField "DiagnosticInfo" "DiagnosticInfoPicker" (fun value current ->
            { current with DiagnosticInfo = value })

        updateColorTextField "SyntaxKeyword" "SyntaxKeywordPicker" (fun value current ->
            { current with SyntaxKeyword = value })

        updateColorTextField "SyntaxString" "SyntaxStringPicker" (fun value current ->
            { current with SyntaxString = value })

        updateColorTextField "SyntaxComment" "SyntaxCommentPicker" (fun value current ->
            { current with SyntaxComment = value })

        updateColorTextField "SyntaxNumber" "SyntaxNumberPicker" (fun value current ->
            { current with SyntaxNumber = value })

        updateColorTextField "SyntaxType" "SyntaxTypePicker" (fun value current -> { current with SyntaxType = value })

        updateColorTextField "SyntaxFunction" "SyntaxFunctionPicker" (fun value current ->
            { current with SyntaxFunction = value })

        updateColorTextField "EditorBorder" "EditorBorderPicker" (fun value current ->
            { current with EditorBorder = value })

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

        updateNumericField "EditorBorderWidth" (fun value current ->
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

        updateNumericField "TabCloseIconSize" (fun value current ->
            { current with
                TabCloseIconSize = value })

        updateNumericField "WorkspaceFontSize" (fun value current ->
            { current with
                WorkspaceFontSize = value })

        let editorFontSize = numericUpDown "EditorFontSize"
        let editorLineHeight = numericUpDown "EditorLineHeight"

        editorFontSize
            .GetObservable(NumericUpDown.ValueProperty)
            .Subscribe(fun value ->
                if not updatingControls then
                    if value.HasValue then
                        let fontSize = value.Value
                        let lineHeightText = formatFloat (float fontSize * (6.0 / 5.0))
                        updatingControls <- true
                        editorLineHeight.Value <- Nullable(Decimal.Parse(lineHeightText, CultureInfo.InvariantCulture))
                        updatingControls <- false

                        updateDraft (fun current ->
                            { current with
                                EditorFontSize = fontSize.ToString(CultureInfo.InvariantCulture)
                                EditorLineHeight = lineHeightText })
                    else
                        updateDraft (fun current -> { current with EditorFontSize = "" }))
        |> ignore

        editorLineHeight
            .GetObservable(NumericUpDown.ValueProperty)
            .Subscribe(fun value ->
                if not updatingControls then
                    if value.HasValue then
                        let lineHeight = value.Value
                        let fontSizeText = formatFloat (float lineHeight * (5.0 / 6.0))
                        updatingControls <- true
                        editorFontSize.Value <- Nullable(Decimal.Parse(fontSizeText, CultureInfo.InvariantCulture))
                        updatingControls <- false

                        updateDraft (fun current ->
                            { current with
                                EditorFontSize = fontSizeText
                                EditorLineHeight = lineHeight.ToString(CultureInfo.InvariantCulture) })
                    else
                        updateDraft (fun current -> { current with EditorLineHeight = "" }))
        |> ignore

        updateNumericField "EditorTabSize" (fun value current -> { current with EditorTabSize = value })

        updateNumericField "CursorWidth" (fun value current -> { current with CursorWidth = value })

        updateNumericField "GutterSeparatorWidth" (fun value current ->
            { current with
                GutterSeparatorWidth = value })

        updateNumericField "GutterPadding" (fun value current -> { current with GutterPadding = value })

        updateNumericField "GutterMinimumWidth" (fun value current ->
            { current with
                GutterMinimumWidth = value })

        updateNumericField "DocumentTabMinHeight" (fun value current ->
            { current with
                DocumentTabMinHeight = value })

        updateNumericField "DocumentTabCloseButtonSize" (fun value current ->
            { current with
                DocumentTabCloseButtonSize = value })

        updateNumericField "DocumentTabPaddingHorizontal" (fun value current ->
            { current with
                DocumentTabPaddingHorizontal = value })

        updateNumericField "DocumentTabPaddingVertical" (fun value current ->
            { current with
                DocumentTabPaddingVertical = value })

        updateNumericField "DocumentTabSpacing" (fun value current ->
            { current with
                DocumentTabSpacing = value })

        updateNumericField "CommandPaletteFontSize" (fun value current ->
            { current with
                CommandPaletteFontSize = value })

        updateNumericField "CommandPaletteWidth" (fun value current ->
            { current with
                CommandPaletteWidth = value })

        updateNumericField "CommandPaletteTopMargin" (fun value current ->
            { current with
                CommandPaletteTopMargin = value })

        updateNumericField "CommandPalettePadding" (fun value current ->
            { current with
                CommandPalettePadding = value })

        updateNumericField "CommandPaletteMaxHeight" (fun value current ->
            { current with
                CommandPaletteMaxHeight = value })

        updateNumericField "CommandPaletteItemMarginHorizontal" (fun value current ->
            { current with
                CommandPaletteItemMarginHorizontal = value })

        updateNumericField "CommandPaletteItemMarginVertical" (fun value current ->
            { current with
                CommandPaletteItemMarginVertical = value })

        updateNumericField "CommandPaletteGestureMargin" (fun value current ->
            { current with
                CommandPaletteGestureMargin = value })

        updateNumericField "WelcomeTitleFontSize" (fun value current ->
            { current with
                WelcomeTitleFontSize = value })

        updateNumericField "TextMutedOpacity" (fun value current ->
            { current with
                TextMutedOpacity = value })

        updateNumericField "ControlCornerRadius" (fun value current ->
            { current with
                ControlCornerRadius = value })

        updateColorTextField "ResizeHandleColor" "ResizeHandleColorPicker" (fun value current ->
            { current with
                ResizeHandleColor = value })

        updateColorTextField "CommandPaletteShadowColor" "CommandPaletteShadowColorPicker" (fun value current ->
            { current with
                CommandPaletteShadowColor = value })

        updateColorTextField "WorkspaceSeparatorColor" "WorkspaceSeparatorColorPicker" (fun value current ->
            { current with
                WorkspaceSeparatorColor = value })

        updateColorTextField "MeasurementColor" "MeasurementColorPicker" (fun value current ->
            { current with
                MeasurementColor = value })

    member this.Configure(settings: AppSettings, loadedThemes: ThemeFile list) =
        themeFiles <- loadedThemes
        let preset = this.FindControl<ComboBox>("ThemePreset")
        preset.ItemsSource <- ThemePreset.all @ (loadedThemes |> List.map (fun theme -> theme.Name))
        selectedThemeName <- None
        themeName <- settings.Theme.Preset
        form <- Some(SettingsForm.fromAppSettings settings)
        this.RefreshControls()

    member this.Configure(settings: AppSettings) = this.Configure(settings, [])

    member _.Form = form

    member _.ThemeName = themeName

    member this.UpdateThemes(loadedThemes: ThemeFile list) =
        themeFiles <- loadedThemes
        let preset = this.FindControl<ComboBox>("ThemePreset")
        preset.ItemsSource <- ThemePreset.all @ (loadedThemes |> List.map (fun theme -> theme.Name))

    member this.SetError(error: string) =
        this.FindControl<TextBlock>("ErrorText").Text <- error

    member private this.RefreshControls() =
        match form with
        | Some value ->
            updatingControls <- true
            let preset = this.FindControl<ComboBox>("ThemePreset")
            preset.SelectedItem <- selectedThemeName |> Option.defaultValue value.ThemePreset
            setText (textBox "ThemeName") themeName
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
            setNumericText (numericUpDown "EditorBorderWidth") value.EditorBorderWidth
            setFontText (fontCombo "EditorFontFamily") value.EditorFontFamily
            setFontText (fontCombo "EditorFallbackFontFamily") value.EditorFallbackFontFamily
            setFontText (fontCombo "UiFontFamily") value.UiFontFamily
            setFontText (fontCombo "IconFontFamily") value.IconFontFamily
            setNumericText (numericUpDown "TabCloseIconSize") value.TabCloseIconSize
            setNumericText (numericUpDown "WorkspaceFontSize") value.WorkspaceFontSize
            setNumericText (numericUpDown "EditorFontSize") value.EditorFontSize
            setNumericText (numericUpDown "EditorLineHeight") value.EditorLineHeight
            setNumericText (numericUpDown "EditorTabSize") value.EditorTabSize
            setNumericText (numericUpDown "CursorWidth") value.CursorWidth
            setNumericText (numericUpDown "GutterSeparatorWidth") value.GutterSeparatorWidth
            setNumericText (numericUpDown "GutterPadding") value.GutterPadding
            setNumericText (numericUpDown "GutterMinimumWidth") value.GutterMinimumWidth
            setNumericText (numericUpDown "DocumentTabMinHeight") value.DocumentTabMinHeight
            setNumericText (numericUpDown "DocumentTabCloseButtonSize") value.DocumentTabCloseButtonSize
            setNumericText (numericUpDown "DocumentTabPaddingHorizontal") value.DocumentTabPaddingHorizontal
            setNumericText (numericUpDown "DocumentTabPaddingVertical") value.DocumentTabPaddingVertical
            setNumericText (numericUpDown "DocumentTabSpacing") value.DocumentTabSpacing
            setNumericText (numericUpDown "CommandPaletteFontSize") value.CommandPaletteFontSize
            setNumericText (numericUpDown "CommandPaletteWidth") value.CommandPaletteWidth
            setNumericText (numericUpDown "CommandPaletteTopMargin") value.CommandPaletteTopMargin
            setNumericText (numericUpDown "CommandPalettePadding") value.CommandPalettePadding
            setNumericText (numericUpDown "CommandPaletteMaxHeight") value.CommandPaletteMaxHeight
            setNumericText (numericUpDown "CommandPaletteItemMarginHorizontal") value.CommandPaletteItemMarginHorizontal
            setNumericText (numericUpDown "CommandPaletteItemMarginVertical") value.CommandPaletteItemMarginVertical
            setNumericText (numericUpDown "CommandPaletteGestureMargin") value.CommandPaletteGestureMargin
            setNumericText (numericUpDown "WelcomeTitleFontSize") value.WelcomeTitleFontSize
            setNumericText (numericUpDown "TextMutedOpacity") value.TextMutedOpacity
            setNumericText (numericUpDown "ControlCornerRadius") value.ControlCornerRadius
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
