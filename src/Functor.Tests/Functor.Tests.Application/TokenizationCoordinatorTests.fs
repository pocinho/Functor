namespace Functor.Tests.Application

open System.Threading
open Functor.Application
open Functor.Domain.Syntax
open Xunit

type private CoordinatorTokenizerService(tokens: LineTokens list) =
    let mutable received: (TokenizationRequest * CancellationToken) option = None

    member _.Received = received

    interface ITokenizerService with
        member _.Tokenize(request, cancellationToken) =
            async {
                received <- Some(request, cancellationToken)
                return Ok { Provider = LocalLexical; Layer = Lexical; Tokens = tokens; Snapshots = []; FinalState = Initial }
            }

type TokenizationCoordinatorTests() =
    [<Fact>]
    member _.``preserves request identity and revision``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "let value")
        let document = session.Model.ActiveDocument.Value
        let token = { Kind = "keyword"; Line = 0; Column = 0; Length = 3 }
        let service = CoordinatorTokenizerService([ { Line = 0; Tokens = [ token ] } ])
        let coordinator = TokenizationCoordinator(service, fun command -> session.DispatchCommand(command) |> ignore)
        use cancellation = new CancellationTokenSource()
        let request =
            { DocumentId = document.Id
              Revision = session.Model.Editing.Revision
              Language = "fsharp"
              Scope = FullDocument
              Lines = session.Model.Editing.Buffer
              InitialState = Initial }

        let result = coordinator.Execute(request, cancellation.Token) |> Async.RunSynchronously

        Assert.True(Result.isOk result)
        Assert.Equal(Some 0L, session.Model.Syntax.TokenizedRevision)
        Assert.Single(session.Model.Syntax.Tokens) |> ignore
        Assert.Equal(FullDocument, (fst service.Received.Value).Scope)
        Assert.True(service.Received.Value |> snd |> fun token -> token.CanBeCanceled)

    [<Fact>]
    member _.``does not dispatch a tokenization result for an older revision``() =
        let documentId = System.Guid.NewGuid()
        let service = CoordinatorTokenizerService([])
        let dispatched = ResizeArray<AppCommand>()
        let coordinator = TokenizationCoordinator(service, dispatched.Add)
        let request revision =
            { DocumentId = documentId
              Revision = revision
              Language = "fsharp"
              Scope = FullDocument
              Lines = [ "let value" ]
              InitialState = Initial }

        let newerResult = coordinator.Execute(request 2L, CancellationToken.None) |> Async.RunSynchronously
        let olderResult = coordinator.Execute(request 1L, CancellationToken.None) |> Async.RunSynchronously

        Assert.True(Result.isOk newerResult)
        Assert.Equal(Error "Tokenization result is stale.", olderResult)
        Assert.Single(dispatched) |> ignore
