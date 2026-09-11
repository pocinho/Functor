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
        let first = (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument firstDocument)).Documents[firstDocument.Id]
        let second = (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument secondDocument)).Documents[secondDocument.Id]
        let firstId = firstDocument.Id
        let secondId = secondDocument.Id
        let second = { second with Editing = EditingLogic.update (InsertString "!") second.Editing }
        let model = workspace (Some "C:\\work") [ firstId, first; secondId, second ] [ secondId; firstId ] (Some secondId)

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
        let firstState = (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument first)).Documents[firstId]
        let secondState = (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument second)).Documents[secondId]
        let model = workspace (Some "C:\\work") [ firstId, firstState; secondId, secondState ] [ firstId; secondId ] None

        let tree = WorkspaceProjection.fileTree model
        let src = tree.Children |> List.exactlyOne
        let files = src.Children

        Assert.True(src.IsDirectory)
        Assert.Equal<string list>([ "a.fs"; "z.fs" ], files |> List.map _.Name)
        Assert.Equal(Some secondId, files[0].DocumentId)
        Assert.Equal(Some firstId, files[1].DocumentId)
        Assert.All(files, fun file -> Assert.StartsWith("file:", file.Key))

    [<Fact>]
    let ``active projection returns document session state`` () =
        let document = DocumentModel.createUntitled "untitled.fs"
        let documentId = document.Id
        let state = (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument document)).Documents[documentId]
        let state = { state with Editing = EditingLogic.update (InsertString "!") state.Editing }
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
        let state = (WorkspaceModel.empty |> WorkspaceLogic.update (AddDocument document)).Documents[documentId]
        let model = workspace None [ documentId, state ] [ documentId ] None

        Assert.True((WorkspaceProjection.activeDocument model).IsNone)
