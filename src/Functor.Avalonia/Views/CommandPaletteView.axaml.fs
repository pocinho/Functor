namespace Functor.Avalonia.Views

open Avalonia.Collections
open Avalonia.Controls
open Avalonia.Input
open Avalonia.Media
open Avalonia.Markup.Xaml
open Functor.Application
open Functor.Rendering

[<AllowNullLiteral>]
type CommandPaletteItem(descriptor: AppCommandDescriptor) =
    member _.Descriptor = descriptor
    member _.Title = descriptor.Title
    member _.Category = descriptor.Category
    member _.GestureText = descriptor.GestureText |> Option.defaultValue ""

type CommandPaletteView() as this =
    inherit UserControl()

    let searchBox = lazy (this.FindControl<TextBox>("SearchBox"))
    let commandList = lazy (this.FindControl<ListBox>("CommandList"))
    let paletteBorder = lazy (this.FindControl<Border>("PaletteBorder"))
    let mutable executeSelected: unit -> unit = ignore
    let mutable paletteState: CommandPaletteState option = None
    let mutable commands: AppCommandDescriptor list = []
    let mutable sessionState: AppSessionState option = None
    let closeRequested = Event<unit>()

    let refreshItems query =
        match sessionState with
        | Some state ->
            let updated = CommandPaletteState.setQuery query commands state
            paletteState <- Some updated
            commandList.Value.Items.Clear()

            updated.Items
            |> List.map CommandPaletteItem
            |> List.iter (fun item -> commandList.Value.Items.Add(item) |> ignore)

            commandList.Value.SelectedIndex <- updated.SelectedIndex |> Option.defaultValue -1
        | None -> ()

    do
        this.InitializeComponent()

        commandList.Value.DoubleTapped.Add(fun _ -> executeSelected ())
        searchBox.Value.TextChanged.Add(fun _ -> refreshItems searchBox.Value.Text)

        commandList.Value.SelectionChanged.Add(fun _ ->
            match paletteState, commandList.Value.SelectedIndex with
            | Some state, index when index >= 0 -> paletteState <- Some(CommandPaletteState.select index state)
            | _ -> ())

        this.KeyDown.Add(fun args ->
            match args.Key with
            | Key.Escape ->
                closeRequested.Trigger()
                args.Handled <- true
            | Key.Enter ->
                executeSelected ()
                args.Handled <- true
            | _ -> ())

    member _.CloseRequested = closeRequested.Publish

    member _.Configure(availableCommands, state: AppSessionState, execute: unit -> unit) =
        commands <- availableCommands
        sessionState <- Some state
        paletteState <- Some(CommandPaletteState.create commands state)
        searchBox.Value.Text <- ""
        refreshItems ""
        executeSelected <- execute

    member _.SelectedDescriptor = paletteState |> Option.bind CommandPaletteState.selected

    member _.FocusSearch() = searchBox.Value.Focus() |> ignore

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
