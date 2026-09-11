namespace Functor.Tests.Domain

open Functor.Domain.Syntax
open Functor.Domain.Document
open Xunit

type SyntaxTests() =
    [<Fact>]
    member _.``setting language invalidates syntax``() =
        let model = SyntaxModel.create () |> SyntaxLogic.update (SetLanguage "fsharp")

        Assert.Equal(Some "fsharp", model.Language)
        Assert.True(model.IsDirty)

    [<Fact>]
    member _.``setting tokens clears syntax dirty state``() =
        let document = DocumentModel.createUntitled "test.fs"
        let token =
            { Kind = "identifier"
              Line = 0
              Column = 0
              Length = 3 }

        let tokens = [ { Line = 0; Tokens = [ token ] } ]
        let model =
            SyntaxModel.createForDocument document.Id 3L
            |> SyntaxModel.markDirty 3L
            |> SyntaxLogic.update (SetTokens(document.Id, 3L, tokens))

        Assert.True((tokens = model.Tokens), "Token cache did not match")
        Assert.False(model.IsDirty)
        Assert.Equal(Some 3L, model.TokenizedRevision)
        Assert.Empty(model.DirtyRanges)

    [<Fact>]
    member _.``setting token range replaces only matching cached lines``() =
        let document = DocumentModel.createUntitled "test.fs"
        let existing =
            [ { Line = 0; Tokens = [ { Kind = "identifier"; Line = 0; Column = 0; Length = 3 } ] }
              { Line = 1; Tokens = [ { Kind = "identifier"; Line = 1; Column = 0; Length = 3 } ] }
              { Line = 2; Tokens = [ { Kind = "identifier"; Line = 2; Column = 0; Length = 3 } ] } ]
        let replacement =
            [ { Line = 1; Tokens = [ { Kind = "keyword"; Line = 1; Column = 0; Length = 3 } ] } ]

        let model =
            { SyntaxModel.createForDocument document.Id 4L with
                Tokens = existing
                IsDirty = true }
            |> SyntaxLogic.update (SetTokenRange(document.Id, 4L, 1, 1, replacement))

        Assert.True([ existing.[0]; replacement.[0]; existing.[2] ] = model.Tokens)
        Assert.False(model.IsDirty)
        Assert.Equal(Some 4L, model.TokenizedRevision)
        Assert.Empty(model.DirtyRanges)

    [<Fact>]
    member _.``setting a token range clears only that dirty range``() =
        let document = DocumentModel.createUntitled "test.fs"
        let model =
            SyntaxModel.createForDocument document.Id 4L
            |> SyntaxModel.markDirtyRange 4L 1 1
            |> SyntaxModel.markDirtyRange 4L 3 4

        let updated =
            model
            |> SyntaxLogic.update (SetTokenRange(document.Id, 4L, 1, 1, []))

        Assert.True(updated.IsDirty)
        Assert.True([ (3, 4) ] = updated.DirtyRanges)

    [<Fact>]
    member _.``dirty ranges merge adjacent lines``() =
        let document = DocumentModel.createUntitled "test.fs"
        let model =
            SyntaxModel.createForDocument document.Id 4L
            |> SyntaxModel.markDirtyRange 4L 3 4
            |> SyntaxModel.markDirtyRange 4L 1 2

        Assert.True([ (1, 4) ] = model.DirtyRanges)

    [<Fact>]
    member _.``stale token results are ignored``() =
        let document = DocumentModel.createUntitled "test.fs"
        let model = SyntaxModel.createForDocument document.Id 4L |> SyntaxModel.markDirty 4L
        let updated = model |> SyntaxLogic.update (SetTokens(document.Id, 3L, []))

        Assert.Equal(model, updated)

    [<Fact>]
    member _.``stale token range results are ignored``() =
        let document = DocumentModel.createUntitled "test.fs"
        let model = SyntaxModel.createForDocument document.Id 4L |> SyntaxModel.markDirty 4L
        let replacement = [ { Line = 0; Tokens = [ { Kind = "keyword"; Line = 0; Column = 0; Length = 3 } ] } ]

        let updated = model |> SyntaxLogic.update (SetTokenRange(document.Id, 3L, 0, 0, replacement))

        Assert.Equal(model, updated)
