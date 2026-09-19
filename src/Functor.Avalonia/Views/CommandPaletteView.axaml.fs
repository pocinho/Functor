namespace Functor.Avalonia.Views

open Avalonia
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
    member _.Description = descriptor.Description
    member _.Category = descriptor.Category
    member _.GestureText = descriptor.GestureText |> Option.defaultValue ""

type CommandPaletteView() as this =
    inherit UserControl()

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

        commandList.Value.Tapped.Add(fun _ ->
            if commandList.Value.SelectedIndex >= 0 then
                executeSelected ())

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
        refreshItems ""
        executeSelected <- execute

    member _.SetQuery(query: string) = refreshItems query

    member _.MoveSelection(offset: int) =
        match paletteState with
        | Some state ->
            let currentIndex = state.SelectedIndex |> Option.defaultValue 0
            let updated = CommandPaletteState.select (currentIndex + offset) state
            paletteState <- Some updated
            commandList.Value.SelectedIndex <- updated.SelectedIndex |> Option.defaultValue -1
        | None -> ()

    member _.SelectedDescriptor = paletteState |> Option.bind CommandPaletteState.selected

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
