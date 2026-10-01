namespace Functor.Application

open System
open System.Collections.Concurrent
open System.Threading
open Functor.Domain.Document

type TokenizationCoordinator(tokenizerService: ITokenizerService, dispatch: AppCommand -> unit) =
    let documentGates = ConcurrentDictionary<DocumentId, SemaphoreSlim>()
    let latestRevisions = ConcurrentDictionary<DocumentId, int64>()

    let isStale (request: TokenizationRequest) =
        latestRevisions.TryGetValue(request.DocumentId)
        |> function
            | true, latest -> request.Revision < latest
            | false, _ -> false

    member _.Execute(request: TokenizationRequest, cancellationToken: CancellationToken) =
        async {
            latestRevisions.AddOrUpdate(request.DocumentId, request.Revision, fun _ latest -> max latest request.Revision)
            |> ignore

            let gate = documentGates.GetOrAdd(request.DocumentId, fun _ -> new SemaphoreSlim(1, 1))

            do! gate.WaitAsync(cancellationToken) |> Async.AwaitTask

            try
                if isStale request then
                    return Error "Tokenization result is stale."
                else
                    let! result = tokenizerService.Tokenize(request, cancellationToken)

                    if isStale request then
                        return Error "Tokenization result is stale."
                    else
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
            finally
                gate.Release() |> ignore
        }