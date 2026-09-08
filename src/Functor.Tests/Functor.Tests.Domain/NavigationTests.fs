namespace Functor.Tests.Domain

open Functor.Domain.Navigation
open Xunit
open TestFixtures

type NavigationTests() =
    [<Fact>]
    member _.``jump list moves back and forward``() =
        let first =
            { Position = position 1 2
              Description = Some "first" }

        let second =
            { Position = position 3 4
              Description = None }

        let model =
            NavigationModel.create ()
            |> applyNavigation (PushJump(first.Position, first.Description))
            |> applyNavigation (PushJump(second.Position, second.Description))

        Assert.Equal(Some 0, model.JumpIndex)
        Assert.True([ second; first ] = model.JumpList)

        let back = model |> applyNavigation JumpBack
        Assert.Equal(Some 1, back.JumpIndex)

        let forward = back |> applyNavigation JumpForward
        Assert.Equal(Some 0, forward.JumpIndex)

    [<Fact>]
    member _.``search results are indexed and bounded``() =
        let results =
            [ { Line = 1
                Column = 2
                Preview = "one" }
              { Line = 4
                Column = 5
                Preview = "two" } ]

        let model =
            NavigationModel.create ()
            |> applyNavigation (SetSearchQuery "term")
            |> applyNavigation (SetSearchResults results)
            |> applyNavigation NextSearchResult
            |> applyNavigation NextSearchResult

        Assert.Equal(Some "term", model.SearchQuery)
        Assert.Equal(Some 1, model.SearchIndex)
        Assert.False(model.IsDirty)
