namespace Functor.Application

open System
open System.Threading
open System.Threading.Tasks
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Syntax

type EditorSessionTokenization(getState: unit -> AppSessionState, requestEffects: AppEffect list -> unit) =
    let mutable pendingTokenization: CancellationTokenSource option = None
    let mutable incrementalState = IncrementalTokenizationState.empty
    let tokenizationDelay = 150

    let cancelPending () =
        match pendingTokenization with
        | Some cancellation ->
            cancellation.Cancel()
            cancellation.Dispose()
            pendingTokenization <- None
        | None -> ()

    let currentRequest () =
        let state = getState ()

        state.Model.ActiveDocument
        |> Option.bind (fun document ->
            FileType.languageId document.Metadata.Path
            |> Option.map (fun language ->
                let scope =
                    match IncrementalTokenizationState.requestRange incrementalState with
                    | Some(startLine, endLine) when startLine = endLine -> Line startLine
                    | Some(startLine, endLine) -> LineRange(startLine, endLine)
                    | None ->
                        match state.Model.Syntax.DirtyRanges with
                        | [] -> FullDocument
                        | ranges ->
                            let startLine = ranges |> List.head |> fst
                            let endLine = ranges |> List.last |> snd

                            if startLine = 0 && endLine = Int32.MaxValue then
                                FullDocument
                            else
                                LineRange(startLine, endLine)

                let initialState =
                    IncrementalTokenizationState.initialState
                        document.Id
                        state.Model.Editing.Revision
                        scope
                        incrementalState

                { DocumentId = document.Id
                  Revision = state.Model.Editing.Revision
                  Language = language
                  Scope = scope
                  Lines = state.Model.Editing.Buffer
                  InitialState = initialState }))

    member _.Cancel() = cancelPending ()

    member _.RequestCurrent(cancellationToken) =
        match currentRequest () with
        | Some request -> requestEffects [ AppEffect.tokenizeWithCancellation request cancellationToken ]
        | None -> cancelPending ()

    member this.Schedule() =
        cancelPending ()

        match currentRequest () with
        | Some _ ->
            let cancellation = new CancellationTokenSource()
            pendingTokenization <- Some cancellation

            Async.StartImmediate(
                async {
                    try
                        do! Task.Delay(tokenizationDelay, cancellation.Token) |> Async.AwaitTask

                        if not cancellation.IsCancellationRequested then
                            this.RequestCurrent(cancellation.Token)
                    with
                    | :? TaskCanceledException -> ()
                    | :? OperationCanceledException -> ()
                }
            )
        | None -> ()

    member _.Reset() =
        cancelPending ()
        incrementalState <- IncrementalTokenizationState.reset

    member _.RemoveDocument(documentId) =
        cancelPending ()
        incrementalState <- IncrementalTokenizationState.removeDocument documentId incrementalState

    member _.BeginEdit(documentId, oldRevision, newRevision, change) =
        incrementalState <-
            IncrementalTokenizationState.beginEdit documentId oldRevision newRevision change incrementalState

    member _.RequestRange = IncrementalTokenizationState.requestRange incrementalState

    member _.IsStable(result: TokenizationResult, bufferLength, endLine) =
        let completedState =
            IncrementalTokenizationState.recordCompletion
                result.DocumentId
                result.Revision
                result.Scope
                result.Snapshots
                result.FinalState
                incrementalState

        IncrementalTokenizationState.isStable
            result.DocumentId
            result.Revision
            bufferLength
            endLine
            result.FinalState
            completedState

    member _.RecordCompletion(result: TokenizationResult) =
        incrementalState <-
            IncrementalTokenizationState.recordCompletion
                result.DocumentId
                result.Revision
                result.Scope
                result.Snapshots
                result.FinalState
                incrementalState

    member _.ClearRange() =
        incrementalState <- IncrementalTokenizationState.clearRange incrementalState

    member _.Extend(startLine, endLine) =
        incrementalState <- IncrementalTokenizationState.extend startLine endLine incrementalState
