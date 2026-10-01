namespace Functor.Agent

open System
open System.Threading
open System.Threading.Tasks

type AgentCapability =
    | ReadWorkspace
    | EditDocument
    | SearchWorkspace
    | ExecuteApplicationCommand

type AgentCommandRequest =
    { CorrelationId: Guid
      CommandName: string
      Arguments: Map<string, string> }

type AgentCommandResponse =
    { CorrelationId: Guid
      Output: string
      AuditEvent: string }

type AgentCommandFailure =
    | Unauthorized
    | CapabilityNotGranted of capability: AgentCapability
    | UnsupportedCommand of commandName: string
    | InvalidArguments of message: string
    | Cancelled
    | ExecutionFailure of message: string

type IAgentCommandAdapter =
    abstract Capabilities: Set<AgentCapability>
    abstract ExecuteAsync: request: AgentCommandRequest * cancellationToken: CancellationToken -> Task<Result<AgentCommandResponse, AgentCommandFailure>>

type AgentCommandAdapter(
    capabilities: Set<AgentCapability>,
    isAuthorized: unit -> bool,
    execute: AgentCommandRequest * CancellationToken -> Task<Result<string, AgentCommandFailure>>
) =
    let requestGate = new SemaphoreSlim(1, 1)

    let requiredCapability commandName =
        match commandName with
        | "workspace.read" -> Some AgentCapability.ReadWorkspace
        | "document.edit" -> Some AgentCapability.EditDocument
        | "search.workspace" -> Some AgentCapability.SearchWorkspace
        | "application.command" -> Some AgentCapability.ExecuteApplicationCommand
        | _ -> None

    let runBounded (cancellationToken: CancellationToken) operation =
        task {
            try
                do! requestGate.WaitAsync(cancellationToken)

                try
                    return! operation ()
                finally
                    requestGate.Release() |> ignore
            with :? OperationCanceledException ->
                return Result.Error AgentCommandFailure.Cancelled
        }

    interface IAgentCommandAdapter with
        member _.Capabilities = capabilities

        member _.ExecuteAsync(request, cancellationToken) =
            runBounded cancellationToken (fun () ->
                task {
                    if not (isAuthorized ()) then
                        return Result.Error AgentCommandFailure.Unauthorized
                    else
                        match requiredCapability request.CommandName with
                        | None -> return Result.Error(AgentCommandFailure.UnsupportedCommand request.CommandName)
                        | Some capability when not (capabilities.Contains capability) ->
                            return Result.Error(AgentCommandFailure.CapabilityNotGranted capability)
                        | Some _ ->
                            let! result = execute(request, cancellationToken)

                            match result with
                            | Result.Ok output ->
                                return
                                    Result.Ok
                                        { CorrelationId = request.CorrelationId
                                          Output = output
                                          AuditEvent = sprintf "agent.command.completed:%s" request.CommandName }
                            | Result.Error error -> return Result.Error error
                })