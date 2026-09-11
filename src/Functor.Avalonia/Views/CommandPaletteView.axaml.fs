namespace Functor.Avalonia.Views

open Avalonia.Controls
open Avalonia.Input
open Avalonia.Media
open Avalonia.Markup.Xaml
open Functor.Application
open Functor.Rendering
open Functor.Avalonia.ViewModels

type CommandPaletteView() as this =
    inherit UserControl()

    let searchBox = lazy (this.FindControl<TextBox>("SearchBox"))
    let commandList = lazy (this.FindControl<ListBox>("CommandList"))
    let paletteBorder = lazy (this.FindControl<Border>("PaletteBorder"))
    let mutable executeSelected: unit -> unit = ignore
    let closeRequested = Event<unit>()

    do
        this.InitializeComponent()

        commandList.Value.DoubleTapped.Add(fun _ -> executeSelected())

        this.KeyDown.Add(fun args ->
            match args.Key with
            | Key.Escape ->
                closeRequested.Trigger()
                args.Handled <- true
            | Key.Enter ->
                executeSelected()
                args.Handled <- true
            | _ -> ())

    member _.CloseRequested = closeRequested.Publish

    member _.Configure(viewModel: CommandPaletteViewModel, execute: unit -> unit) =
        this.DataContext <- viewModel
        executeSelected <- execute

    member _.FocusSearch() =
        searchBox.Value.Focus() |> ignore

    member _.ApplyTheme(themeSettings: ThemeSettings) =
        let palette = themeSettings.ThemeSource.Resolve()

        let colorFromArgb (argb: uint32) =
            Color.FromArgb(
                byte ((argb >>> 24) &&& 0xFFu),
                byte ((argb >>> 16) &&& 0xFFu),
                byte ((argb >>> 8) &&& 0xFFu),
                byte (argb &&& 0xFFu)
            )

        let foreground = SolidColorBrush(colorFromArgb palette.Foreground)
        let background = SolidColorBrush(colorFromArgb palette.Background)
        let inputBackground = SolidColorBrush(colorFromArgb palette.GutterBackground)
        let borderColor = palette.GutterSeparator |> Option.defaultValue palette.Foreground

        paletteBorder.Value.Background <- background
        paletteBorder.Value.BorderBrush <- SolidColorBrush(colorFromArgb borderColor)
        this.Foreground <- foreground
        searchBox.Value.Background <- inputBackground
        searchBox.Value.Foreground <- foreground
        commandList.Value.Background <- background
        commandList.Value.Foreground <- foreground

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)