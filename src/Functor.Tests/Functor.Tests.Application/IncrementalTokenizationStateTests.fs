namespace Functor.Tests.Application

open System
open Functor.Application
open Functor.Domain.Editing
open Functor.Domain.Document
open Xunit

type IncrementalTokenizationStateTests() =
    [<Fact>]
    member _.``remaps snapshots to the new revision and line positions``() =
        let documentId = Guid.NewGuid()
        let state =
            { IncrementalTokenizationState.empty with
                Snapshots =
                    Map.ofList
                        [ (documentId, 3L, 1), FSharpState 1
                          (documentId, 3L, 4), FSharpState 0 ] }

        let updated =
            IncrementalTokenizationState.beginEdit
                documentId
                3L
                4L
                { StartLine = 2
                  EndLine = 3
                  OldEndLine = 3
                  LineDelta = 1 }
                state

        Assert.Equal(Some(FSharpState 1), IncrementalTokenizationState.snapshotAt documentId 4L 1 updated)
        Assert.Equal(Some(FSharpState 0), IncrementalTokenizationState.snapshotAt documentId 4L 5 updated)
        Assert.Equal(Some(2, 3), IncrementalTokenizationState.requestRange updated)

    [<Fact>]
    member _.``does not reuse snapshots from another document or revision``() =
        let documentId = Guid.NewGuid()
        let otherDocumentId = Guid.NewGuid()
        let state =
            { IncrementalTokenizationState.empty with
                Snapshots = Map.ofList [ (documentId, 2L, 1), FSharpState 1 ] }

        Assert.True((IncrementalTokenizationState.snapshotAt otherDocumentId 2L 1 state).IsNone)
        Assert.True((IncrementalTokenizationState.snapshotAt documentId 3L 1 state).IsNone)

    [<Fact>]
    member _.``stabilizes when the next cached state matches the final state``() =
        let documentId = Guid.NewGuid()
        let state =
            { IncrementalTokenizationState.empty with
                Snapshots = Map.ofList [ (documentId, 4L, 2), FSharpState 0 ] }

        Assert.True(IncrementalTokenizationState.isStable documentId 4L 5 1 (FSharpState 0) state)
