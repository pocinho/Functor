namespace Functor.Avalonia.ViewModels

open System.Collections.ObjectModel
open Functor.Application

[<AllowNullLiteral>]
type CommandPaletteItem(descriptor: AppCommandDescriptor) =
    member _.Descriptor = descriptor
    member _.Title = descriptor.Title
    member _.Category = descriptor.Category
    member _.GestureText = descriptor.GestureText |> Option.defaultValue ""

type CommandPaletteViewModel(commands: AppCommandDescriptor list, state: AppSessionState) =
    inherit ViewModelBase()

    let mutable searchText = ""
    let mutable selectedItem: CommandPaletteItem = null
    let items = ObservableCollection<CommandPaletteItem>()

    let enabledCommands() =
        commands |> List.filter (fun command -> command.IsEnabled state)

    let refreshItems query =
        items.Clear()

        enabledCommands()
        |> AppCommandCatalog.filter query
        |> List.map CommandPaletteItem
        |> List.iter items.Add

        selectedItem <- if items.Count > 0 then items[0] else null

    do refreshItems searchText

    member this.SearchText
        with get () = searchText
        and set value =
            if this.SetProperty(&searchText, value) then
                refreshItems searchText
                this.OnPropertyChanged(nameof this.SelectedItem)

    member _.Items = items

    member this.SelectedItem
        with get () = selectedItem
        and set value = this.SetProperty(&selectedItem, value) |> ignore

    member _.SelectedDescriptor =
        if isNull selectedItem then None else Some selectedItem.Descriptor