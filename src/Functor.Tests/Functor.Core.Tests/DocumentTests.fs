namespace Functor.Core.Tests

open System
open Functor.Domain.Document
open Xunit

type DocumentTests() =
    [<Fact>]
    member _.``file document derives name from path``() =
        let document = DocumentModel.createFromFile "C:\\work\\notes.txt" "contents"

        Assert.Equal(Some "C:\\work\\notes.txt", document.Metadata.Path)
        Assert.Equal("notes.txt", document.Metadata.Name)
        Assert.Equal("contents", document.InitialText)
        Assert.False(document.Metadata.IsDirty)

    [<Fact>]
    member _.``document events update metadata without changing identity``() =
        let document = DocumentModel.createUntitled "untitled"
        let renamed = DocumentLogic.update (RenameDocument "renamed") document
        let dirty = DocumentLogic.update MarkDocumentDirty renamed
        let timestamp = DateTime(2024, 1, 2, 3, 4, 5, DateTimeKind.Utc)
        let updated = DocumentLogic.update (UpdateModifiedTimestamp timestamp) dirty

        Assert.Equal(document.Id, updated.Id)
        Assert.Equal("renamed", updated.Metadata.Name)
        Assert.True(updated.Metadata.IsDirty)
        Assert.Equal(Some timestamp, updated.Metadata.ModifiedAt)
