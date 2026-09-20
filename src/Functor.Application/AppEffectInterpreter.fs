namespace Functor.Application

open Functor.Domain.Core
open Functor.Domain.Editing
open System
open System.Threading

/// Executes application effects through injected platform-neutral services.
type AppEffectInterpreter(services: EditorServices, dispatch: AppCommand -> unit) =
    let reportFailure message =
        dispatch (AppCommand.fileOperationFailed message)

    new(clipboardService, fileService, dialogService, dispatch, ?tokenizerService) =
        AppEffectInterpreter(
            EditorServices.create clipboardService fileService dialogService tokenizerService,
            dispatch
        )

    member this.Execute(effect: AppEffect) =
        async {
            try
                match effect with
                | NoEffect -> ()
                | NotifyStatus message -> dispatch (AppCommand.setStatus message)
                | NotifyError error -> dispatch (AppCommand.reportError error)
                | WriteClipboard text -> do! services.Clipboard.SetText text
                | ReadClipboard ->
                    let! text = services.Clipboard.GetText()

                    match text with
                    | Some value -> dispatch (AppCommand.toCoreEvent (ApplyEditingEvent(InsertString value)))
                    | None -> ()
                | PasteText text -> dispatch (AppCommand.toCoreEvent (ApplyEditingEvent(InsertString text)))
                | OpenFile ->
                    let! path = services.Dialog.OpenFile()

                    match path with
                    | Some value -> do! this.Execute(AppEffect.readFile value)
                    | None -> ()
                | OpenFolder ->
                    let! path = services.Dialog.OpenFolder()

                    match path with
                    | Some value -> dispatch (AppCommand.folderOpened value)
                    | None -> ()
                | ReadFile path ->
                    let! result = services.File.ReadText path

                    match result with
                    | Ok contents -> dispatch (AppCommand.fileOpened path contents)
                    | Error message -> reportFailure message
                | SaveFile(suggestedName, contents) ->
                    let! path = services.Dialog.SaveFile suggestedName

                    match path with
                    | Some value -> do! this.Execute(AppEffect.writeFile value contents)
                    | None -> ()
                | SaveFileForDocument(documentId, revision, suggestedName, contents) ->
                    let! path = services.Dialog.SaveFile suggestedName

                    match path with
                    | Some value ->
                        let! result = services.File.WriteText(value, contents)

                        match result with
                        | Ok() -> dispatch (AppCommand.fileSavedForDocument documentId revision value)
                        | Error message -> reportFailure message
                    | None -> ()
                | WriteFile(path, contents) ->
                    let! result = services.File.WriteText(path, contents)

                    match result with
                    | Ok() -> dispatch (AppCommand.fileSaved path)
                    | Error message -> reportFailure message
                | WriteFileForDocument(documentId, revision, path, contents) ->
                    let! result = services.File.WriteText(path, contents)

                    match result with
                    | Ok() -> dispatch (AppCommand.fileSavedForDocument documentId revision path)
                    | Error message -> reportFailure message
                | Tokenize(request, cancellationToken) ->
                    match services.Tokenizer with
                    | Some service ->
                        let coordinator = TokenizationCoordinator(service, dispatch)
                        let! result = coordinator.Execute(request, cancellationToken)

                        match result with
                        | Ok() -> ()
                        | Error message -> dispatch (AppCommand.reportError message)
                    | None -> dispatch (AppCommand.reportError "No tokenizer service is configured.")
            with
            | :? OperationCanceledException -> ()
            | ex -> reportFailure ex.Message
        }
