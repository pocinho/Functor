namespace Functor.Lsp

open System
open System.Collections.Concurrent
open System.Threading
open System.Threading.Tasks
open Functor.Application
open Functor.Domain.Editing

type LanguageServiceAdapter(transport: ILspTransport) =
    let mutable isStarted = false
    let latestVersions = ConcurrentDictionary<Guid, int64>()
    let lifecycleGate = new SemaphoreSlim(1, 1)
    let documentGates = ConcurrentDictionary<Guid, SemaphoreSlim>()

    let documentGate (documentId: Guid) =
        documentGates.GetOrAdd(documentId, fun _ -> new SemaphoreSlim(1, 1))

    let isStale (document: LanguageDocument) =
        match latestVersions.TryGetValue(document.DocumentId) with
        | true, version -> document.Version < version
        | false, _ -> false

    let runBounded (gate: SemaphoreSlim) (cancellationToken: CancellationToken) operation =
        task {
            try
                do! gate.WaitAsync(cancellationToken)

                try
                    return! operation ()
                finally
                    gate.Release() |> ignore
            with :? OperationCanceledException ->
                return Result.Error LanguageServiceFailure.RequestCancelled
        }

    let mapFailure (cancellationToken: CancellationToken) (error: string) =
        if cancellationToken.IsCancellationRequested then
            LanguageServiceFailure.RequestCancelled
        else
            LanguageServiceFailure.TransportFailure error

    let mapResponse (expected: string) (response: LspResponse) : Result<unit, LanguageServiceFailure> =
        match expected, response with
        | "start", LanguageStarted
        | "publish", DocumentPublished
        | "stop", LanguageStopped -> Result.Ok()
        | expected, _ -> Result.Error(LanguageServiceFailure.ProtocolFailure(sprintf "Unexpected response for %s." expected))

    let mapSeverity (severity: LspDiagnosticSeverity) =
        match severity with
        | LspDiagnosticSeverity.Error -> Functor.Domain.Diagnostics.DiagnosticSeverity.Error
        | LspDiagnosticSeverity.Warning -> Functor.Domain.Diagnostics.DiagnosticSeverity.Warning
        | LspDiagnosticSeverity.Information -> Functor.Domain.Diagnostics.DiagnosticSeverity.Information
        | LspDiagnosticSeverity.Hint -> Functor.Domain.Diagnostics.DiagnosticSeverity.Hint

    let mapDiagnostic (diagnostic: LspDiagnostic) : Functor.Domain.Diagnostics.Diagnostic =
        { Severity = mapSeverity diagnostic.Severity
          Message = diagnostic.Message
          RangeStart =
            { Line = diagnostic.Range.Start.Line
              Column = diagnostic.Range.Start.Character }
          RangeEnd =
            { Line = diagnostic.Range.End.Line
              Column = diagnostic.Range.End.Character }
          Code = diagnostic.Code
          Source = diagnostic.Source }

    interface ILanguageService with
        member _.StartAsync(language, cancellationToken) =
            runBounded lifecycleGate cancellationToken (fun () ->
                task {
                    if isStarted then
                        return Result.Error(LanguageServiceFailure.ProtocolFailure "Language service is already started.")
                    else
                        let! result = transport.SendAsync(StartLanguage language, cancellationToken)

                        match result with
                        | Result.Ok response ->
                            match mapResponse "start" response with
                            | Result.Ok() ->
                                isStarted <- true
                                return Result.Ok()
                            | Result.Error error -> return Result.Error error
                        | Result.Error error -> return Result.Error(mapFailure cancellationToken error)
                })

        member _.PublishDocumentAsync(document, cancellationToken) =
            runBounded (documentGate document.DocumentId) cancellationToken (fun () ->
                task {
                    if not isStarted then
                        return Result.Error LanguageServiceFailure.NotInitialized
                    elif isStale document then
                        return Result.Error LanguageServiceFailure.StaleResult
                    else
                        let! result = transport.SendAsync(PublishDocument document, cancellationToken)

                        match result with
                        | Result.Ok response ->
                            match mapResponse "publish" response with
                            | Result.Ok() ->
                                latestVersions.AddOrUpdate(
                                    document.DocumentId,
                                    document.Version,
                                    fun _ version -> max version document.Version
                                )
                                |> ignore
                                return Result.Ok()
                            | Result.Error error -> return Result.Error error
                        | Result.Error error -> return Result.Error(mapFailure cancellationToken error)
                })

        member _.RequestDiagnosticsAsync(document, cancellationToken) =
            runBounded (documentGate document.DocumentId) cancellationToken (fun () ->
                task {
                    if not isStarted then
                        return Result.Error LanguageServiceFailure.NotInitialized
                    elif isStale document then
                        return Result.Error LanguageServiceFailure.StaleResult
                    else
                        let! result = transport.SendAsync(RequestDiagnostics document, cancellationToken)

                        if isStale document then
                            return Result.Error LanguageServiceFailure.StaleResult
                        else
                            match result with
                            | Result.Ok(Diagnostics diagnostics) -> return Result.Ok(List.map mapDiagnostic diagnostics)
                            | Result.Ok _ -> return Result.Error(LanguageServiceFailure.ProtocolFailure "Expected diagnostics response.")
                            | Result.Error error -> return Result.Error(mapFailure cancellationToken error)
                })

        member _.StopAsync(cancellationToken) =
            runBounded lifecycleGate cancellationToken (fun () ->
                task {
                    if not isStarted then
                        return Result.Ok()
                    else
                        let! result = transport.SendAsync(StopLanguage, cancellationToken)

                        match result with
                        | Result.Ok response ->
                            match mapResponse "stop" response with
                            | Result.Ok() ->
                                isStarted <- false
                                return Result.Ok()
                            | Result.Error error -> return Result.Error error
                        | Result.Error error -> return Result.Error(mapFailure cancellationToken error)
                })
