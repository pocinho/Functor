namespace Functor.Tests.Application

open System.IO
open Xunit
open Functor.Application
open Functor.Workspace

module WorkspaceLayoutStoreTests =
    type private FakeFileService() =
        let mutable readResult = Ok ""
        let mutable written: (string * string) option = None

        member _.ReadResult
            with set value = readResult <- value

        member _.Written = written

        interface IFileService with
            member _.ReadText _ = async { return readResult }

            member _.WriteText(path, contents) =
                written <- Some(path, contents)
                async { return Ok() }

    [<Fact>]
    let ``store loads and saves workspace layout at workspace path`` () =
        let fileService = FakeFileService()

        let layout = { SidePanelWidth = 480.0 }

        fileService.ReadResult <- Ok(WorkspaceLayout.document layout |> WorkspaceLayout.toJson)
        let store = WorkspaceLayoutStore(fileService)
        let root = Some "C:\\work"
        let loaded = store.Load(root) |> Async.RunSynchronously
        let saved = store.Save(root, layout) |> Async.RunSynchronously
        let expectedPath = Path.Combine("C:\\work", ".functor", "layout.json")

        Assert.Equal(Ok(WorkspaceLayout.document layout), loaded)
        Assert.Equal(Ok(), saved)
        Assert.Equal(Some(expectedPath, WorkspaceLayout.document layout |> WorkspaceLayout.toJson), fileService.Written)

    [<Fact>]
    let ``store rejects layout operations without workspace root`` () =
        let store = WorkspaceLayoutStore(FakeFileService())
        let layout = WorkspaceLayout.defaults

        let loaded = store.Load(None) |> Async.RunSynchronously
        let saved = store.Save(None, layout) |> Async.RunSynchronously

        Assert.Equal(Error "Cannot load workspace layout without a workspace root.", loaded)
        Assert.Equal(Error "Cannot save workspace layout without a workspace root.", saved)
