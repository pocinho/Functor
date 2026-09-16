namespace Functor.Tests.Application

open Functor.Application
open Functor.Domain.Core
open Xunit

type CommandPaletteStateTests() =
    let state = AppSessionState.empty CoreModel.empty

    [<Fact>]
    member _.``creates palette with enabled commands selected``() =
        let palette = CommandPaletteState.create AppCommandCatalog.all state

        Assert.Equal(Some 0, palette.SelectedIndex)
        Assert.Contains(palette.Items, fun command -> command.Id = "file.new")

    [<Fact>]
    member _.``updates query and filters commands``() =
        let palette = CommandPaletteState.setQuery "settings" AppCommandCatalog.all state

        Assert.Equal("settings", palette.Query)
        Assert.Single(palette.Items) |> ignore
        Assert.Equal("workbench.settings", palette.Items.Head.Id)

    [<Fact>]
    member _.``clamps selection and returns selected command``() =
        let palette =
            CommandPaletteState.create AppCommandCatalog.all state
            |> CommandPaletteState.select 100

        let selected = CommandPaletteState.selected palette

        Assert.Equal(Some "workbench.settings", selected |> Option.map (fun command -> command.Id))

    [<Fact>]
    member _.``empty command lists have no selection``() =
        let palette = CommandPaletteState.create [] state

        Assert.Empty(palette.Items)
        Assert.Equal(None, palette.SelectedIndex)
        Assert.Equal(None, CommandPaletteState.selected palette)

    [<Fact>]
    member _.``filtering to no matches clears the selection``() =
        let palette =
            CommandPaletteState.setQuery "does-not-exist" AppCommandCatalog.all state

        Assert.Empty(palette.Items)
        Assert.Equal(None, palette.SelectedIndex)

    [<Fact>]
    member _.``negative selection indexes clamp to the first command``() =
        let palette =
            CommandPaletteState.create AppCommandCatalog.all state
            |> CommandPaletteState.select -10

        Assert.Equal(Some 0, palette.SelectedIndex)
