namespace Functor.Avalonia.Views

open Avalonia.Controls
open Avalonia.Media
open Avalonia.Markup.Xaml
open Functor.Application
open Functor.Rendering

type SettingsView() as this =
    inherit UserControl()

    do this.InitializeComponent()

    member this.ApplyTheme(themeSettings: ThemeSettings) =
        let palette = themeSettings.ThemeSource.Resolve()
        let colorFromArgb (argb: uint32) =
            Color.FromArgb(byte (argb >>> 24), byte (argb >>> 16), byte (argb >>> 8), byte argb)

        this.Background <- SolidColorBrush(colorFromArgb palette.Background)
        this.Foreground <- SolidColorBrush(colorFromArgb palette.Foreground)
        this.FindControl<TextBlock>("ErrorText").Foreground <- SolidColorBrush(colorFromArgb palette.DiagnosticError)

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)