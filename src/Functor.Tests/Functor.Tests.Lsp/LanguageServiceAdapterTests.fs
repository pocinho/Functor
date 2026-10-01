namespace Functor.Tests.Lsp

open System
open System.Threading
open System.Threading.Tasks
open Functor.Application
open Functor.Domain.Diagnostics
open Functor.Lsp
open Xunit

type private FakeTransport(responses: Result<LspResponse, string> list) =
    let mutable remaining = responses
    let requests = ResizeArray<LspRequest>()

    member _.Requests = List.ofSeq requests

    interface ILspTransport with
        member _.SendAsync(request, _) =
            requests.Add request

            match remaining with
            | response :: rest ->
                remaining <- rest
                Task.FromResult response
            | [] -> Task.FromResult(Result.Error "No response configured")

module private TestData =
    let document =
        { DocumentId = Guid.NewGuid()
          Path = Some "C:\\work\\sample.fs"
          Version = 3L
          Lines = [ "let value = 1" ] }

type LanguageServiceAdapterTests() =
    [<Fact>]
    member _.``rejects document operations before startup``() =
        let adapter = LanguageServiceAdapter(FakeTransport([])) :> ILanguageService

        let result = adapter.PublishDocumentAsync(TestData.document, CancellationToken.None).Result

        Assert.Equal(Result.Error LanguageServiceFailure.NotInitialized, result)

    [<Fact>]
    member _.``translates diagnostics and preserves range metadata``() =
        let diagnostic: LspDiagnostic = { Range = { Start = { Line = 2; Character = 4 }; End = { Line = 2; Character = 9 } }; Severity = LspDiagnosticSeverity.Warning; Message = "value is unused"; Code = Some "FS1183"; Source = Some "fsharp" }

        let transport =
            FakeTransport([ Result.Ok LanguageStarted; Result.Ok(Diagnostics [ diagnostic ]) ])

        let adapter = LanguageServiceAdapter(transport) :> ILanguageService
        Assert.Equal(Result.Ok(), (adapter.StartAsync("fsharp", CancellationToken.None).Result))

        match adapter.RequestDiagnosticsAsync(TestData.document, CancellationToken.None).Result with
        | Result.Ok [ diagnostic ] ->
            Assert.Equal(DiagnosticSeverity.Warning, diagnostic.Severity)
            Assert.Equal(2, diagnostic.RangeStart.Line)
            Assert.Equal(4, diagnostic.RangeStart.Column)
            Assert.Equal(Some "FS1183", diagnostic.Code)
        | result -> failwithf "Expected one diagnostic, got %A" result

    [<Fact>]
    member _.``maps cancelled transport failures explicitly``() =
        let transport = FakeTransport([ Result.Ok LanguageStarted; Result.Error "connection closed" ])
        let adapter = LanguageServiceAdapter(transport) :> ILanguageService
        let cancellation = new CancellationTokenSource()

        Assert.Equal(Result.Ok(), (adapter.StartAsync("fsharp", CancellationToken.None).Result))
        cancellation.Cancel()

        let result = adapter.PublishDocumentAsync(TestData.document, cancellation.Token).Result

        Assert.Equal(Result.Error LanguageServiceFailure.RequestCancelled, result)

    [<Fact>]
    member _.``rejects an unexpected diagnostics response``() =
        let transport = FakeTransport([ Result.Ok LanguageStarted; Result.Ok DocumentPublished ])
        let adapter = LanguageServiceAdapter(transport) :> ILanguageService

        Assert.Equal(Result.Ok(), (adapter.StartAsync("fsharp", CancellationToken.None).Result))

        match adapter.RequestDiagnosticsAsync(TestData.document, CancellationToken.None).Result with
        | Result.Error(LanguageServiceFailure.ProtocolFailure message) ->
            Assert.Contains("Expected diagnostics response", message)
        | result -> failwithf "Expected a protocol failure, got %A" result

    [<Fact>]
    member _.``rejects a document publish older than the latest published version``() =
        let transport = FakeTransport([ Result.Ok LanguageStarted; Result.Ok DocumentPublished ])
        let adapter = LanguageServiceAdapter(transport) :> ILanguageService
        let newer = { TestData.document with Version = 4L }
        let older = { TestData.document with Version = 3L }

        Assert.Equal(Result.Ok(), (adapter.StartAsync("fsharp", CancellationToken.None).Result))
        Assert.Equal(Result.Ok(), (adapter.PublishDocumentAsync(newer, CancellationToken.None).Result))

        Assert.Equal(
            Result.Error LanguageServiceFailure.StaleResult,
            adapter.PublishDocumentAsync(older, CancellationToken.None).Result
        )