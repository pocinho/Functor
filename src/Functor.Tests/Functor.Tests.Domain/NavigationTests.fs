namespace Functor.Tests.Domain

open Functor.Domain.Navigation
open Functor.Domain.Search
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
            |> applyNavigation (NavigationEvent.SetSearchQuery "term")
            |> applyNavigation (NavigationEvent.SetSearchResults results)
            |> applyNavigation NavigationEvent.NextSearchResult
            |> applyNavigation NavigationEvent.NextSearchResult

        Assert.Equal(Some "term", model.Search.Query)
        Assert.Equal(Some 1, model.Search.Index)
        Assert.False(model.IsDirty)

    [<Fact>]
    member _.``editing invalidates exact search matches``() =
        let document: SearchDocument =
            { Id = System.Guid.NewGuid()
              Path = None
              Name = "untitled"
              Lines = [ "term" ] }

        let matches = SearchEngine.findInDocument (SearchOptions.create "term") document

        let model =
            NavigationModel.create ()
            |> applyNavigation (NavigationEvent.SetSearchMatches(0L, matches))
            |> applyNavigation NavigationEvent.InvalidateSearch

        Assert.Empty(model.Search.Matches)
        Assert.True(model.IsDirty)

    [<Fact>]
    member _.``empty search query clears the query and exact matches``() =
        let model =
            NavigationModel.create ()
            |> applyNavigation (NavigationEvent.SetSearchQuery "term")
            |> applyNavigation (NavigationEvent.SetSearchQuery "")

        Assert.Equal(None, model.Search.Query)
        Assert.Empty(model.Search.Matches)
        Assert.Equal("", model.Search.Options.Query)
