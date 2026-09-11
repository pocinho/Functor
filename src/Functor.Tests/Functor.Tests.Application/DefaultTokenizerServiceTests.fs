namespace Functor.Tests.Application

open System
open System.Threading
open Functor.Application
open Xunit

module private DefaultTokenizerServiceTest =
    let tokenize language lines =
        let service = DefaultTokenizerService() :> ITokenizerService
        let request =
            { DocumentId = Guid.NewGuid()
              Revision = 1L
              Language = language
              Scope = FullDocument
              Lines = lines
              InitialState = Initial }

        service.Tokenize(request, CancellationToken.None) |> Async.RunSynchronously

type DefaultTokenizerServiceTests() =
    [<Fact>]
    member _.``reports unsupported languages``() =
        let result = DefaultTokenizerServiceTest.tokenize "unknown" [ "text" ]

        match result with
        | Ok _ -> Assert.True(false, "Expected unsupported language to fail")
        | Error message -> Assert.Contains("Unsupported language", message)

    [<Fact>]
    member _.``identifies local lexical output``() =
        let result = DefaultTokenizerServiceTest.tokenize "fsharp" [ "let value" ]

        match result with
        | Ok output ->
            Assert.Equal(LocalLexical, output.Provider)
            Assert.Equal(Lexical, output.Layer)
        | Error message -> Assert.True(false, message)

    [<Fact>]
    member _.``honors cancellation``() =
        use cancellation = new CancellationTokenSource()
        cancellation.Cancel()
        let service = DefaultTokenizerService() :> ITokenizerService
        let request =
            { DocumentId = Guid.NewGuid()
              Revision = 1L
              Language = "fsharp"
              Scope = FullDocument
              Lines = [ "let value" ]
              InitialState = Initial }

        Assert.Throws<OperationCanceledException>(fun () ->
            service.Tokenize(request, cancellation.Token) |> Async.RunSynchronously |> ignore)
        |> ignore