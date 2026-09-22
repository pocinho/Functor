namespace Functor.Tests.Domain

open System
open Functor.Domain.Editing
open Functor.Domain.Search
open Xunit

type SearchTests() =
    let document lines : SearchDocument =
        { Id = Guid.Parse("11111111-1111-1111-1111-111111111111")
          Path = Some "C:\\work\\sample.fs"
          Name = "sample.fs"
          Lines = lines }

    [<Fact>]
    member _.``search returns exact UTF-16 ranges``() =
        let results =
            SearchEngine.findInDocument (SearchOptions.create "term") (document [ "before term"; "term after" ])

        Assert.Equal(2, results.Length)
        Assert.Equal(0, results[0].Line)
        Assert.Equal(7, results[0].Column)
        Assert.Equal(4, results[0].Length)
        Assert.Equal(11, results[0].Range.End.Column)
        Assert.Equal(1, results[1].Line)
        Assert.Equal(0, results[1].Column)

    [<Fact>]
    member _.``search uses ordinal case insensitive matching by default``() =
        let results =
            SearchEngine.findInDocument (SearchOptions.create "term") (document [ "TERM term" ])

        Assert.Equal(2, results.Length)

    [<Fact>]
    member _.``search can use case sensitive matching``() =
        let options = { Query = "term"; CaseSensitive = true }
        let results = SearchEngine.findInDocument options (document [ "TERM term" ])

        Assert.Single(results) |> ignore
        Assert.Equal(5, results[0].Column)

    [<Fact>]
    member _.``search reports emoji columns as UTF-16 offsets``() =
        let results =
            SearchEngine.findInDocument (SearchOptions.create "term") (document [ "\uD83D\uDEA7 term" ])

        Assert.Single(results) |> ignore
        Assert.Equal(3, results[0].Column)
        Assert.Equal({ Line = 0; Column = 3 }, results[0].Range.Start)

    [<Fact>]
    member _.``empty queries return no results``() =
        let results =
            SearchEngine.findInDocument (SearchOptions.create "") (document [ "term" ])

        Assert.Empty(results)

    [<Fact>]
    member _.``multiline literal queries are deferred and return no results``() =
        let results =
            SearchEngine.findInDocument (SearchOptions.create "first\nsecond") (document [ "first"; "second" ])

        Assert.Empty(results)

    [<Fact>]
    member _.``previews are bounded and include context markers``() =
        let line = String.replicate 100 "x" + "term" + String.replicate 100 "y"

        let results =
            SearchEngine.findInDocument (SearchOptions.create "term") (document [ line ])

        Assert.Single(results) |> ignore
        Assert.StartsWith("...", results[0].Preview)
        Assert.EndsWith("...", results[0].Preview)

    [<Fact>]
    member _.``search preserves exact UTF-16 boundaries around combining marks``() =
        let results =
            SearchEngine.findInDocument (SearchOptions.create "e") (document [ "cafe\u0301" ])

        Assert.Single(results) |> ignore
        Assert.Equal(3, results[0].Column)
        Assert.Equal(1, results[0].Length)

        Assert.Equal(
            { Start = { Line = 0; Column = 3 }
              End = { Line = 0; Column = 4 } },
            results[0].Range
        )

    [<Fact>]
    member _.``search results are deterministic by document order and source order``() =
        let first =
            { document [ "term term" ] with
                Id = Guid.Parse("11111111-1111-1111-1111-111111111111") }

        let second =
            { document [ "term" ] with
                Id = Guid.Parse("22222222-2222-2222-2222-222222222222") }

        let results =
            SearchEngine.findInDocuments (SearchOptions.create "term") [ first; second ]

        Assert.Equal<int list>([ 0; 0; 0 ], results |> List.map (fun result -> result.Line))
        Assert.Equal<int list>([ 0; 5; 0 ], results |> List.map (fun result -> result.Column))

        Assert.Equal<Guid list>(
            [ first.Id; first.Id; second.Id ],
            results |> List.map (fun result -> result.DocumentId)
        )

    [<Fact>]
    member _.``search handles large plain text without indexing``() =
        let source = String.replicate 10000 "term "

        let results =
            SearchEngine.findInDocument (SearchOptions.create "term") (document [ source ])

        Assert.Equal(10000, results.Length)
