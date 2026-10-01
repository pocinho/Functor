namespace Functor.PluginHost

open System
open System.Threading
open System.Threading.Tasks

type PluginCapability =
    | ReadDocument
    | ExecuteCommand
    | PublishNotification

type PluginDocumentSnapshot =
    { DocumentId: Guid
      Path: string option
      Version: int64
      Text: string }

type PluginCommand =
    { Id: string
      Title: string }

type PluginFailure =
    | CapabilityDenied of capability: PluginCapability
    | DocumentNotFound of documentId: Guid
    | InvalidRequest of message: string
    | Cancelled
    | HostUnavailable of message: string

type IPluginContext =
    abstract Capabilities: Set<PluginCapability>
    abstract ReadDocumentAsync: documentId: Guid * cancellationToken: CancellationToken -> Task<Result<PluginDocumentSnapshot, PluginFailure>>
    abstract ExecuteCommandAsync: command: PluginCommand * cancellationToken: CancellationToken -> Task<Result<unit, PluginFailure>>
    abstract PublishNotification: message: string -> Result<unit, PluginFailure>

type PluginContext(
    capabilities: Set<PluginCapability>,
    readDocument: Guid * CancellationToken -> Task<Result<PluginDocumentSnapshot, PluginFailure>>,
    executeCommand: PluginCommand * CancellationToken -> Task<Result<unit, PluginFailure>>,
    publishNotification: string -> Result<unit, PluginFailure>
) =
    let denied capability = Result.Error(PluginFailure.CapabilityDenied capability)
    let requestGate = new SemaphoreSlim(1, 1)

    let runBounded (cancellationToken: CancellationToken) operation =
        task {
            try
                do! requestGate.WaitAsync(cancellationToken)

                try
                    return! operation ()
                finally
                    requestGate.Release() |> ignore
            with :? OperationCanceledException ->
                return Result.Error PluginFailure.Cancelled
        }

    interface IPluginContext with
        member _.Capabilities = capabilities

        member _.ReadDocumentAsync(documentId, cancellationToken) =
            runBounded cancellationToken (fun () ->
                task {
                    if capabilities.Contains ReadDocument then
                        return! readDocument(documentId, cancellationToken)
                    else
                        return denied ReadDocument
                })

        member _.ExecuteCommandAsync(command, cancellationToken) =
            runBounded cancellationToken (fun () ->
                task {
                    if capabilities.Contains ExecuteCommand then
                        return! executeCommand(command, cancellationToken)
                    else
                        return denied ExecuteCommand
                })

        member _.PublishNotification(message) =
            if capabilities.Contains PublishNotification then
                publishNotification message
            else
                denied PublishNotification
