namespace Functor.Avalonia.Views

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
        updateTextField "EditorBorder" (fun value current -> { current with EditorBorder = value })

        updateTextField "EditorBorderWidth" (fun value current ->
            { current with
                EditorBorderWidth = value })

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
            setText (textBox "EditorBorder") value.EditorBorder
            setText (textBox "EditorBorderWidth") value.EditorBorderWidth
            updatingControls <- false
        | None -> ()

    member this.ApplyTheme(themeSettings: ThemeSettings) =
        let palette = themeSettings.ThemeSource.Resolve()

        let colorFromArgb (argb: uint32) =
            Color.FromArgb(byte (argb >>> 24), byte (argb >>> 16), byte (argb >>> 8), byte argb)

        this.Background <- SolidColorBrush(colorFromArgb palette.Background)
        this.Foreground <- SolidColorBrush(colorFromArgb palette.Foreground)
        this.FindControl<TextBlock>("ErrorText").Foreground <- SolidColorBrush(colorFromArgb palette.DiagnosticError)

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
