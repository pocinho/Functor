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

    [<Fact>]
    member _.``next and previous search results remain bounded with no results``() =
        let model =
            NavigationModel.create ()
            |> applyNavigation NavigationEvent.NextSearchResult
            |> applyNavigation NavigationEvent.PrevSearchResult

        Assert.Null(model.Search.Index)
        Assert.Empty(model.Search.Matches)

    [<Fact>]
    member _.``changing search options resets results and selection``() =
        let model =
            NavigationModel.create ()
            |> applyNavigation (NavigationEvent.SetSearchQuery "term")
            |> applyNavigation (
                NavigationEvent.SetSearchResults
                    [ { Line = 0
                        Column = 0
                        Preview = "term" } ]
            )
            |> applyNavigation (NavigationEvent.SetSearchOptions { Query = "term"; CaseSensitive = true })

        Assert.True(model.Search.Options.CaseSensitive)
        Assert.Empty(model.Search.Results)
        Assert.Empty(model.Search.Matches)
        Assert.Null(model.Search.Index)
        Assert.True(model.IsDirty)

    [<Fact>]
    member _.``search history is bounded, deduplicated, and excludes empty queries``() =
        let first =
            { Query = "first"
              CaseSensitive = false }

        let second =
            { Query = "second"
              CaseSensitive = true }

        let model =
            NavigationModel.create ()
            |> applyNavigation (NavigationEvent.SetSearchOptions first)
            |> applyNavigation (NavigationEvent.SetSearchMatches(1L, []))
            |> applyNavigation (NavigationEvent.SetSearchOptions second)
            |> applyNavigation (NavigationEvent.SetSearchMatches(2L, []))
            |> applyNavigation (NavigationEvent.SetSearchOptions first)
            |> applyNavigation (NavigationEvent.SetSearchMatches(3L, []))

        Assert.Equal<SearchOptions list>([ first; second ], model.Search.History)

        let historyModel =
            [ 1..21 ]
            |> List.fold
                (fun current index ->
                    let options = SearchOptions.create (sprintf "term-%d" index)

                    current
                    |> applyNavigation (NavigationEvent.SetSearchOptions options)
                    |> applyNavigation (NavigationEvent.SetSearchMatches(int64 index, [])))
                (NavigationModel.create ())

        Assert.Equal(20, historyModel.Search.History.Length)
        Assert.Equal("term-21", historyModel.Search.History.Head.Query)

    [<Fact>]
    member _.``clearing search preserves session history``() =
        let options = SearchOptions.create "term"

        let model =
            NavigationModel.create ()
            |> applyNavigation (NavigationEvent.SetSearchOptions options)
            |> applyNavigation (NavigationEvent.SetSearchMatches(1L, []))
            |> applyNavigation NavigationEvent.ClearSearch

        Assert.Equal<SearchOptions list>([ options ], model.Search.History)

    [<Fact>]
    member _.``clearing search history preserves the active search``() =
        let options = SearchOptions.create "term"

        let model =
            NavigationModel.create ()
            |> applyNavigation (NavigationEvent.SetSearchOptions options)
            |> applyNavigation (NavigationEvent.SetSearchMatches(1L, []))
            |> applyNavigation NavigationEvent.ClearSearchHistory

        Assert.Equal(Some "term", model.Search.Query)
        Assert.Empty(model.Search.Results)
        Assert.Empty(model.Search.History)
