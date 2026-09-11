namespace Functor.Tests.Workspace

open System
open System.IO
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Workspace
open Xunit

module private Helpers =
    let document name text =
        { Id = Guid.NewGuid()
          Metadata =
              { Path = None
                Name = name
                IsDirty = false
                CreatedAt = DateTime.UtcNow
                ModifiedAt = None }
          InitialText = text }

type WorkspaceTests() =
    [<Fact>]
    member _.``workspace canonicalizes its root path``() =
        let root = Path.Combine(Path.GetTempPath(), "functor-workspace", "..", "workspace")
        let workspace = WorkspaceModel.create (Some root)

        Assert.Equal(Path.GetFullPath (root.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)), workspace.RootPath.Value)

    [<Fact>]
    member _.``adding documents creates independent per-document state``() =
        let first = Helpers.document "first.fs" "first"
        let second = Helpers.document "second.fs" "second"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument first)
            |> WorkspaceLogic.update (AddDocument second)

        let firstState = WorkspaceModel.tryFindDocument first.Id workspace |> Option.get
        let secondState = WorkspaceModel.tryFindDocument second.Id workspace |> Option.get
        let changedFirst = EditingLogic.update (InsertString "!") firstState.Editing
        let updated = { workspace with Documents = workspace.Documents.Add(first.Id, { firstState with Editing = changedFirst }) }

        Assert.Equal(Some first.Id, workspace.ActiveDocumentId)
        Assert.Equal<string list>([ "first" ], firstState.Editing.Buffer)
        Assert.Equal<string list>([ "second" ], secondState.Editing.Buffer)
        Assert.Equal<string list>([ "!first" ], (WorkspaceModel.tryFindDocument first.Id updated |> Option.get).Editing.Buffer)
        Assert.Equal<string list>([ "second" ], (WorkspaceModel.tryFindDocument second.Id updated |> Option.get).Editing.Buffer)

    [<Fact>]
    member _.``duplicate document identity does not replace existing state``() =
        let document = Helpers.document "same.fs" "original"
        let replacement = { document with InitialText = "replacement" }
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument document)
            |> WorkspaceLogic.update (AddDocument replacement)

        Assert.Equal(1, workspace.Documents.Count)
        Assert.Equal<string list>([ "original" ], (WorkspaceModel.tryFindDocument document.Id workspace |> Option.get).Editing.Buffer)

    [<Fact>]
    member _.``duplicate canonical file paths activate only one tab``() =
        let first = DocumentModel.createFromFile (Path.Combine(Path.GetTempPath(), "functor", "file.fs")) "original"
        let equivalent = DocumentModel.createFromFile (Path.Combine(Path.GetTempPath(), "functor", ".", "file.fs")) "replacement"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument first)
            |> WorkspaceLogic.update (AddDocument equivalent)

        Assert.Single(workspace.Documents) |> ignore
        Assert.Equal(Some first.Id, workspace.ActiveDocumentId)
        Assert.Equal(Some first.Id, WorkspaceModel.tryFindDocumentByPath equivalent.Metadata.Path.Value workspace |> Option.map (fun document -> document.Document.Id))

    [<Fact>]
    member _.``removing active document falls back to the first remaining tab``() =
        let first = Helpers.document "first.fs" "first"
        let second = Helpers.document "second.fs" "second"
        let third = Helpers.document "third.fs" "third"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument first)
            |> WorkspaceLogic.update (AddDocument second)
            |> WorkspaceLogic.update (AddDocument third)
            |> WorkspaceLogic.update (ActivateDocument third.Id)
            |> WorkspaceLogic.update (RemoveDocument third.Id)

        Assert.Equal(Some first.Id, workspace.ActiveDocumentId)

    [<Fact>]
    member _.``file policy distinguishes root files from outside files``() =
        let workspace = WorkspaceModel.create (Some (Path.Combine(Path.GetTempPath(), "functor-root")))

        Assert.True(WorkspaceModel.isPathWithinRoot (Path.Combine(Path.GetTempPath(), "functor-root", "file.fs")) workspace)
        Assert.False(WorkspaceModel.isPathWithinRoot (Path.Combine(Path.GetTempPath(), "functor-root-other", "file.fs")) workspace)

    [<Fact>]
    member _.``closing a saved document records it for reopening``() =
        let document = DocumentModel.createFromFile (Path.Combine(Path.GetTempPath(), "functor", "closed.fs")) "text"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument document)
            |> WorkspaceLogic.update (RemoveDocument document.Id)

        Assert.Single(workspace.RecentlyClosedDocuments) |> ignore
        Assert.Equal(document.Metadata.Path.Value, workspace.RecentlyClosedDocuments.Head.Path)

    [<Fact>]
    member _.``closing an untitled document does not create reopen history``() =
        let document = Helpers.document "untitled" "text"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument document)
            |> WorkspaceLogic.update (RemoveDocument document.Id)

        Assert.Empty(workspace.RecentlyClosedDocuments)

    [<Fact>]
    member _.``consuming a recently closed path removes it from history``() =
        let document = DocumentModel.createFromFile (Path.Combine(Path.GetTempPath(), "functor", "closed.fs")) "text"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument document)
            |> WorkspaceLogic.update (RemoveDocument document.Id)
            |> WorkspaceLogic.update (ConsumeRecentlyClosed document.Metadata.Path.Value)

        Assert.Empty(workspace.RecentlyClosedDocuments)

    [<Fact>]
    member _.``removing active document selects the remaining document``() =
        let first = Helpers.document "first.fs" "first"
        let second = Helpers.document "second.fs" "second"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument first)
            |> WorkspaceLogic.update (AddDocument second)
            |> WorkspaceLogic.update (RemoveDocument first.Id)

        Assert.Equal(Some second.Id, workspace.ActiveDocumentId)
        Assert.False(WorkspaceModel.containsDocument first.Id workspace)

    [<Fact>]
    member _.``editing event routes to the addressed document``() =
        let first = Helpers.document "first.fs" "first"
        let second = Helpers.document "second.fs" "second"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument first)
            |> WorkspaceLogic.update (AddDocument second)
            |> WorkspaceLogic.update (ApplyEditingEvent(first.Id, InsertString "!"))

        Assert.Equal<string list>([ "!first" ], (WorkspaceModel.tryFindDocument first.Id workspace |> Option.get).Editing.Buffer)
        Assert.Equal<string list>([ "second" ], (WorkspaceModel.tryFindDocument second.Id workspace |> Option.get).Editing.Buffer)

    [<Fact>]
    member _.``revision mismatched routed syntax event is ignored``() =
        let document = Helpers.document "file.fs" "text"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument document)
            |> WorkspaceLogic.update (ApplySyntaxEvent(document.Id, 1L, SetTokens(document.Id, 1L, [])))

        let syntax = WorkspaceModel.tryFindDocument document.Id workspace |> Option.get |> fun state -> state.Syntax
        Assert.Empty(syntax.Tokens)
        Assert.Equal(0L, syntax.DocumentRevision)

    [<Fact>]
    member _.``workspace queries identify dirty and untitled documents``() =
        let document = Helpers.document "untitled" "text"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument document)
            |> WorkspaceLogic.update (ApplyEditingEvent(document.Id, InsertString "!"))

        Assert.Equal<DocumentId list>([ document.Id ], WorkspaceQuery.unsavedDocumentIds workspace)
        Assert.True(WorkspaceQuery.hasUnsavedDocuments workspace)
        Assert.False(WorkspaceQuery.canSafelyReplaceWorkspace workspace)

    [<Fact>]
    member _.``replacing workspace resets identity documents and settings``() =
        let document = Helpers.document "file.fs" "text"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument document)
            |> fun current -> { current with Settings = { current.Settings with IndentSize = 2 } }
        let replacement = WorkspaceLogic.update (ReplaceWorkspace(Some "C:\\workspace")) workspace

        Assert.NotEqual(workspace.Id, replacement.Id)
        Assert.Equal(Some(WorkspaceModel.canonicalizeRootPath "C:\\workspace"), replacement.RootPath)
        Assert.Empty(replacement.Documents)
        Assert.Equal(WorkspaceSettings.defaults, replacement.Settings)

    [<Fact>]
    member _.``clearing documents preserves workspace identity and root``() =
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (SetRootPath(Some "C:\\workspace"))
            |> WorkspaceLogic.update (AddDocument(Helpers.document "file.fs" "text"))
        let cleared = WorkspaceLogic.update ClearDocuments workspace

        Assert.Equal(workspace.Id, cleared.Id)
        Assert.Equal(workspace.RootPath, cleared.RootPath)
        Assert.True(cleared.Documents.IsEmpty)
        Assert.True(cleared.ActiveDocumentId.IsNone)

    [<Fact>]
    member _.``workspace replacement requests confirmation for unsaved documents``() =
        let document = Helpers.document "untitled" "text"
        let workspace =
            WorkspaceModel.empty
            |> WorkspaceLogic.update (AddDocument document)
            |> WorkspaceLogic.update (ApplyEditingEvent(document.Id, InsertString "!"))

        match WorkspaceLifecycle.requestReplacement (Some "C:\\workspace") workspace with
        | RequiresDiscardConfirmation documentIds ->
            Assert.Equal<DocumentId list>([ document.Id ], documentIds)
        | WorkspaceReady _ ->
            Assert.True(false, "Expected replacement to require confirmation")

    [<Fact>]
    member _.``workspace replacement is immediate when all documents are clean``() =
        let workspace = WorkspaceModel.empty

        match WorkspaceLifecycle.requestReplacement (Some "C:\\workspace") workspace with
        | WorkspaceReady replacement ->
            Assert.Equal(Some(WorkspaceModel.canonicalizeRootPath "C:\\workspace"), replacement.RootPath)
            Assert.True(replacement.Documents.IsEmpty)
        | RequiresDiscardConfirmation _ ->
            Assert.True(false, "Expected clean replacement to proceed")
