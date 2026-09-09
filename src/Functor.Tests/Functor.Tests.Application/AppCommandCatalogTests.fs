namespace Functor.Tests.Application

open Functor.Application
open Functor.Domain.Core
open Xunit

type AppCommandCatalogTests() =
    [<Fact>]
    member _.``filters commands by title category and gesture``() =
        let settingsMatches = AppCommandCatalog.all |> AppCommandCatalog.filter "settings"
        let gestureMatches = AppCommandCatalog.all |> AppCommandCatalog.filter "ctrl+shift+p"

        Assert.Contains(settingsMatches, fun command -> command.Id = "workbench.settings")
        Assert.Contains(gestureMatches, fun command -> command.Id = "workbench.commandPalette")

    [<Fact>]
    member _.``save command is disabled without an active document``() =
        let saveCommand = AppCommandCatalog.tryFindById "file.save" |> Option.defaultWith (fun () -> failwith "Missing save command")
        let state = AppSessionState.empty CoreModel.empty

        Assert.False(saveCommand.IsEnabled state)