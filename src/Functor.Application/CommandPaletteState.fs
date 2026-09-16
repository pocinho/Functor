namespace Functor.Application

type CommandPaletteState =
    { Query: string
      Items: AppCommandDescriptor list
      SelectedIndex: int option }

module CommandPaletteState =
    let private selectedIndex (items: AppCommandDescriptor list) = if items.IsEmpty then None else Some 0

    let create commands state =
        let items = commands |> List.filter (fun command -> command.IsEnabled state)

        { Query = ""
          Items = items
          SelectedIndex = selectedIndex items }

    let setQuery query commands state =
        let items =
            commands
            |> List.filter (fun command -> command.IsEnabled state)
            |> AppCommandCatalog.filter query

        { Query = query
          Items = items
          SelectedIndex = selectedIndex items }

    let select index palette =
        let selectedIndex =
            if palette.Items.IsEmpty then
                None
            else
                Some(max 0 (min (palette.Items.Length - 1) index))

        { palette with
            SelectedIndex = selectedIndex }

    let selected palette =
        palette.SelectedIndex
        |> Option.bind (fun index -> palette.Items |> List.tryItem index)
