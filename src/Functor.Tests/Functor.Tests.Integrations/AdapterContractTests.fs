namespace Functor.Tests.Integrations

open System
open System.Threading
open System.Threading.Tasks
open Functor.Agent
open Functor.PluginHost
open Xunit

type AdapterContractTests() =
    [<Fact>]
    member _.``plugin context denies capabilities before invoking callbacks``() =
        let readDocument _ =
            Task.FromResult<Result<PluginDocumentSnapshot, PluginFailure>>(Result.Error(InvalidRequest "should not run"))

        let executeCommand _ =
            Task.FromResult<Result<unit, PluginFailure>>(Result.Ok())

        let context =
            PluginContext(Set [ ReadDocument ], readDocument, executeCommand, fun _ -> Result.Ok())
            :> IPluginContext

        let result =
            context.ExecuteCommandAsync({ Id = "test.command"; Title = "Test" }, CancellationToken.None).Result

        Assert.Equal(Result.Error(CapabilityDenied ExecuteCommand), result)

    [<Fact>]
    member _.``agent adapter rejects unsupported commands and missing capabilities``() =
        let execute _ = Task.FromResult<Result<string, AgentCommandFailure>>(Result.Ok "done")

        let adapter =
            AgentCommandAdapter(Set [ ReadWorkspace ], (fun () -> true), execute)
            :> IAgentCommandAdapter

        let request name =
            { CorrelationId = Guid.NewGuid()
              CommandName = name
              Arguments = Map.empty }

        let unsupported = adapter.ExecuteAsync(request "unknown", CancellationToken.None).Result
        let denied = adapter.ExecuteAsync(request "document.edit", CancellationToken.None).Result

        Assert.Equal(Result.Error(UnsupportedCommand "unknown"), unsupported)
        Assert.Equal(Result.Error(CapabilityNotGranted EditDocument), denied)

    [<Fact>]
    member _.``agent adapter returns an audit event for authorized execution``() =
        let execute _ = Task.FromResult<Result<string, AgentCommandFailure>>(Result.Ok "completed")

        let adapter =
            AgentCommandAdapter(Set [ SearchWorkspace ], (fun () -> true), execute)
            :> IAgentCommandAdapter

        let request =
            { CorrelationId = Guid.NewGuid()
              CommandName = "search.workspace"
              Arguments = Map.empty }

        match adapter.ExecuteAsync(request, CancellationToken.None).Result with
        | Result.Ok response ->
            Assert.Equal("completed", response.Output)
            Assert.Equal("agent.command.completed:search.workspace", response.AuditEvent)
        | Result.Error error -> failwithf "Expected successful agent execution, got %A" error

    [<Fact>]
    member _.``plugin and agent adapters report cancellation without invoking callbacks``() =
        use cancellation = new CancellationTokenSource()
        cancellation.Cancel()

        let mutable pluginCalled = false
        let plugin =
            PluginContext(
                Set [ ReadDocument ],
                (fun _ ->
                    pluginCalled <- true
                    Task.FromResult<Result<PluginDocumentSnapshot, PluginFailure>>(Result.Ok Unchecked.defaultof<_>)),
                (fun _ -> Task.FromResult<Result<unit, PluginFailure>>(Result.Ok())),
                fun _ -> Result.Ok()
            )
            :> IPluginContext

        let agent =
            AgentCommandAdapter(
                Set [ ReadWorkspace ],
                (fun () -> true),
                (fun _ -> Task.FromResult<Result<string, AgentCommandFailure>>(Result.Ok "unexpected"))
            )
            :> IAgentCommandAdapter

        let request =
            { CorrelationId = Guid.NewGuid()
              CommandName = "workspace.read"
              Arguments = Map.empty }

        let pluginResult = plugin.ReadDocumentAsync(Guid.NewGuid(), cancellation.Token).Result
        let agentResult = agent.ExecuteAsync(request, cancellation.Token).Result

        Assert.Equal(Result.Error PluginFailure.Cancelled, pluginResult)
        Assert.Equal(Result.Error AgentCommandFailure.Cancelled, agentResult)
        Assert.False(pluginCalled)