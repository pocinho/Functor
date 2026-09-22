namespace Functor.Tests.Application

open System
open Functor.Application
open Functor.Domain.Search
open Xunit

type WorkspaceReplacementTests() =
    [<Fact>]
    member _.``replacement groups files and applies ranges from the end``() =
        let firstPath = "C:\work\first.fs"
        let secondPath = "C:\work\second.fs"

        let matches =
            [ { DocumentId = Guid.NewGuid()
                Path = Some firstPath
                Name = "first.fs"
                Line = 0
                Column = 0
                Length = 4
                Range = Unchecked.defaultof<_>
                Preview = "term" }
              { DocumentId = Guid.NewGuid()
                Path = Some firstPath
                Name = "first.fs"
                Line = 0
                Column = 5
                Length = 4
                Range = Unchecked.defaultof<_>
                Preview = "term" }
              { DocumentId = Guid.NewGuid()
                Path = Some secondPath
                Name = "second.fs"
                Line = 1
                Column = 0
                Length = 4
                Range = Unchecked.defaultof<_>
                Preview = "term" } ]

        let result =
            WorkspaceReplacement.apply
                "🚧"
                (Map.ofList [ firstPath, "term term"; secondPath, "line\nterm" ])
                matches

        match result with
        | Ok replacements ->
            Assert.Equal("🚧 🚧", replacements[firstPath])
            Assert.Equal("line\n🚧", replacements[secondPath])
        | Error errors -> Assert.True(false, String.concat "; " errors)

    [<Fact>]
    member _.``replacement rejects missing source snapshots``() =
        let path = "C:\work\missing.fs"

        let matchValue =
            { DocumentId = Guid.NewGuid()
              Path = Some path
              Name = "missing.fs"
              Line = 0
              Column = 0
              Length = 4
              Range = Unchecked.defaultof<_>
              Preview = "term" }

        match WorkspaceReplacement.apply "word" Map.empty [ matchValue ] with
        | Ok _ -> Assert.True(false, "Expected missing source error.")
        | Error errors -> Assert.Contains("No source snapshot", errors.Head)