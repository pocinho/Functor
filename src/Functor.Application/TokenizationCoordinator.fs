namespace Functor.Application

open System.Threading

type TokenizationCoordinator(tokenizerService: ITokenizerService, dispatch: AppCommand -> unit) =
    member _.Execute(request: TokenizationRequest, cancellationToken: CancellationToken) =
        async {
            let! result = tokenizerService.Tokenize(request, cancellationToken)

            match result with
            | Ok output ->
                let completion: TokenizationResult =
                    { DocumentId = request.DocumentId
                      Revision = request.Revision
                      Scope = request.Scope
                      Provider = output.Provider
                      Layer = output.Layer
                      Tokens = output.Tokens
                      Snapshots = output.Snapshots
                      FinalState = output.FinalState }

                dispatch (AppCommand.tokenizationCompleted completion)

                return Ok()
            | Error message ->
                return Error message
        }