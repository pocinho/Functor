namespace Functor.Avalonia.Views

open Avalonia.Controls
open Avalonia.Input
open Avalonia.Markup.Xaml
open Functor.Avalonia.ViewModels

type CommandPaletteView() as this =
    inherit UserControl()

    let searchBox = lazy (this.FindControl<TextBox>("SearchBox"))
    let commandList = lazy (this.FindControl<ListBox>("CommandList"))
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

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)