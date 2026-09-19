namespace Functor.Tests.Workspace

open System
open System.IO
open Xunit
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Workspace

module WorkspaceProjectionTests =

    let private workspace rootPath documents tabOrder activeDocumentId =
        { WorkspaceModel.create rootPath with
            ActiveDocumentId = activeDocumentId
            Documents = documents |> Map.ofList
            TabOrder = tabOrder }

    [<Fact>]
    let ``tabs follow tab order and expose active and dirty state`` () =
        let firstDocument = DocumentModel.createFromFile "C:\\work\\first.fs" "content"
        let secondDocument = DocumentModel.createFromFile "C:\\work\\second.fs" "content"

        let first =
            (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument firstDocument)).Documents[firstDocument.Id]

        let second =
            (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument secondDocument)).Documents[secondDocument.Id]

        let firstId = firstDocument.Id
        let secondId = secondDocument.Id

        let second =
            { second with
                Editing = EditingLogic.update (InsertString "!") second.Editing }

        let model =
            workspace (Some "C:\\work") [ firstId, first; secondId, second ] [ secondId; firstId ] (Some secondId)

        let tabs = WorkspaceProjection.tabs model

        Assert.Equal<DocumentId list>([ secondId; firstId ], tabs |> List.map _.DocumentId)
        Assert.Equal<int list>([ 0; 1 ], tabs |> List.map _.Index)
        Assert.True(tabs[0].IsDirty)
        Assert.False(tabs[1].IsDirty)
        Assert.True(tabs[0].IsActive)
        Assert.False(tabs[1].IsActive)

    [<Fact>]
    let ``file tree preserves document ids and sorts nested paths`` () =
        let first = DocumentModel.createFromFile "C:\\work\\src\\z.fs" "content"
        let second = DocumentModel.createFromFile "C:\\work\\src\\a.fs" "content"
        let firstId = first.Id
        let secondId = second.Id

        let firstState =
            (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument first)).Documents[firstId]

        let secondState =
            (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument second)).Documents[secondId]

        let model =
            workspace (Some "C:\\work") [ firstId, firstState; secondId, secondState ] [ firstId; secondId ] None

        let tree = WorkspaceFileTree.create model
        let src = tree.Children |> List.exactlyOne
        let files = src.Children

        Assert.True(src.IsDirectory)
        Assert.Equal<string list>([ "a.fs"; "z.fs" ], files |> List.map _.Name)
        Assert.Equal(Some secondId, files[0].DocumentId)
        Assert.Equal(Some firstId, files[1].DocumentId)
        Assert.All(files, fun file -> Assert.StartsWith("file:", file.Key))

    [<Fact>]
    let ``file tree is flat when no workspace root is selected`` () =
        let first = DocumentModel.createFromFile "C:\\work\\src\\first.fs" "content"
        let second = DocumentModel.createFromFile "D:\\other\\second.fs" "content"
        let firstId = first.Id
        let secondId = second.Id

        let firstState =
            (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument first)).Documents[firstId]

        let secondState =
            (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument second)).Documents[secondId]

        let model =
            workspace None [ firstId, firstState; secondId, secondState ] [ firstId; secondId ] None

        let tree = WorkspaceFileTree.create model

        Assert.True(tree.IsDirectory)
        Assert.Equal<string list>([ "first.fs"; "second.fs" ], tree.Children |> List.map _.Name)
        Assert.All(tree.Children, fun file -> Assert.False(file.IsDirectory))

    [<Fact>]
    let ``folder workspace tree includes closed files and nested directories`` () =
        let root =
            Path.Combine(Path.GetTempPath(), "functor-projection-" + Guid.NewGuid().ToString("N"))

        let source = Path.Combine(root, "src")
        let openPath = Path.Combine(root, "open.fs")
        let closedPath = Path.Combine(source, "closed.fs")

        try
            Directory.CreateDirectory(source) |> ignore
            File.WriteAllText(openPath, "open")
            File.WriteAllText(closedPath, "closed")

            let openDocument = DocumentModel.createFromFile openPath "open"

            let openDocumentState =
                (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument openDocument)).Documents[openDocument.Id]

            let model =
                workspace (Some root) [ openDocument.Id, openDocumentState ] [ openDocument.Id ] None

            let tree = WorkspaceFileTree.create model
            let sourceNode = tree.Children |> List.find (fun node -> node.Name = "src")
            let closedNode = sourceNode.Children |> List.exactlyOne
            let openNode = tree.Children |> List.find (fun node -> node.Name = "open.fs")

            Assert.True(sourceNode.IsDirectory)
            Assert.Equal("closed.fs", closedNode.Name)
            Assert.True(closedNode.DocumentId.IsNone)
            Assert.Equal(Some openDocument.Id, openNode.DocumentId)
        finally
            if Directory.Exists root then
                Directory.Delete(root, true)

    [<Fact>]
    let ``large workspace tree builds a complete sorted snapshot`` () =
        let root =
            Path.Combine(Path.GetTempPath(), "functor-large-projection-" + Guid.NewGuid().ToString("N"))

        try
            for directoryIndex in 0..19 do
                let directory = Path.Combine(root, sprintf "dir-%02d" directoryIndex)
                Directory.CreateDirectory(directory) |> ignore

                for fileIndex in 0..19 do
                    File.WriteAllText(Path.Combine(directory, sprintf "file-%02d.fs" fileIndex), "content")

            let tree = WorkspaceFileTree.create (workspace (Some root) [] [] None)
            let files = tree.Children |> List.collect (fun directory -> directory.Children)

            Assert.Equal(20, tree.Children.Length)
            Assert.Equal(400, files.Length)

            Assert.Equal<string list>(
                [ "dir-00"; "dir-01"; "dir-02"; "dir-03"; "dir-04" ],
                tree.Children |> List.take 5 |> List.map _.Name
            )

            Assert.All(files, fun file -> Assert.EndsWith(".fs", file.Name))
        finally
            if Directory.Exists root then
                Directory.Delete(root, true)

    [<Fact>]
    let ``active projection returns document session state`` () =
        let document = DocumentModel.createUntitled "untitled.fs"
        let documentId = document.Id

        let state =
            (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument document)).Documents[documentId]

        let state =
            { state with
                Editing = EditingLogic.update (InsertString "!") state.Editing }

        let model = workspace None [ documentId, state ] [ documentId ] (Some documentId)

        let active = WorkspaceProjection.activeDocument model

        Assert.True(active.IsSome)
        let projection = active.Value
        Assert.Equal(documentId, projection.DocumentId)
        Assert.Equal("untitled.fs", projection.Name)
        Assert.True(projection.Path.IsNone)
        Assert.True(projection.IsDirty)
        Assert.Same(state, projection.Session)

    [<Fact>]
    let ``active projection is empty without an active document`` () =
        let document = DocumentModel.createUntitled "file.fs"
        let documentId = document.Id

        let state =
            (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument document)).Documents[documentId]

        let model = workspace None [ documentId, state ] [ documentId ] None

        Assert.True((WorkspaceProjection.activeDocument model).IsNone)

    [<Fact>]
    let ``tab projection exposes independent auxiliary state`` () =
        let first = DocumentModel.createUntitled "first.fs"
        let second = DocumentModel.createUntitled "second.fs"

        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument first)
            |> WorkspaceLogic.update (AddDocument second)

        let workspace = WorkspaceLogic.update (SetAgentOpen(second.Id, true)) workspace
        let tabs = WorkspaceProjection.tabs workspace

        Assert.False(
            tabs
            |> List.find (fun tab -> tab.DocumentId = first.Id)
            |> fun tab -> tab.AgentIsOpen
        )

        Assert.True(
            tabs
            |> List.find (fun tab -> tab.DocumentId = second.Id)
            |> fun tab -> tab.AgentIsOpen
        )

        Assert.False(workspace.Documents[first.Id].Auxiliary.Agent.IsOpen)
