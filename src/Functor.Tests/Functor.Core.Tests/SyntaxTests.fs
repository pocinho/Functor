namespace Functor.Core.Tests

open Functor.Domain.Syntax
open Xunit

type SyntaxTests() =
    [<Fact>]
    member _.``setting language invalidates syntax``() =
        let model = SyntaxModel.create () |> SyntaxLogic.update (SetLanguage "fsharp")

        Assert.Equal(Some "fsharp", model.Language)
        Assert.True(model.IsDirty)

    [<Fact>]
    member _.``setting tokens clears syntax dirty state``() =
        let token =
            { Kind = "identifier"
              Line = 0
              Column = 0
              Length = 3 }

        let tokens = [ { Line = 0; Tokens = [ token ] } ]
        let model = SyntaxModel.create () |> SyntaxLogic.update (SetTokens tokens)

        Assert.True((tokens = model.Tokens), "Token cache did not match")
        Assert.False(model.IsDirty)
        Assert.True(model.LastTokenized.IsSome)
