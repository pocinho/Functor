namespace Functor.Application

open Functor.Domain.Core
open Functor.Domain.Editing
open System.Threading

/// Executes application effects through injected platform-neutral services.
type AppEffectInterpreter(
    clipboardService: IClipboardService,
    fileService: IFileService,
    dialogService: IDialogService,
    dispatch: AppCommand -> unit,
    ?tokenizerService: ITokenizerService
) =
    let reportFailure message =
        dispatch (AppCommand.fileOperationFailed message)

    member this.Execute(effect: AppEffect) =
        async {
            try
                match effect with
                | NoEffect ->
                    ()
                | NotifyStatus message ->
                    dispatch (AppCommand.setStatus message)
                | NotifyError error ->
                    dispatch (AppCommand.reportError error)
                | WriteClipboard text ->
                    do! clipboardService.SetText text
                | ReadClipboard ->
                    let! text = clipboardService.GetText()

                    match text with
                    | Some value ->
                        dispatch (AppCommand.toCoreEvent (ApplyEditingEvent(InsertString value)))
                    | None ->
                        ()
                | PasteText text ->
                    dispatch (AppCommand.toCoreEvent (ApplyEditingEvent(InsertString text)))
                | OpenFile ->
                    let! path = dialogService.OpenFile()

                    match path with
                    | Some value ->
                        do! this.Execute (AppEffect.readFile value)
                    | None ->
                        ()
                | OpenFolder ->
                    let! path = dialogService.OpenFolder()

                    match path with
                    | Some value ->
                        dispatch (AppCommand.folderOpened value)
                    | None ->
                        ()
                | ReadFile path ->
                    let! result = fileService.ReadText path

                    match result with
                    | Ok contents ->
                        dispatch (AppCommand.fileOpened path contents)
                    | Error message ->
                        reportFailure message
                | SaveFile(suggestedName, contents) ->
                    let! path = dialogService.SaveFile suggestedName

                    match path with
                    | Some value ->
                        do! this.Execute (AppEffect.writeFile value contents)
                    | None ->
                        ()
                | SaveFileForDocument(documentId, revision, suggestedName, contents) ->
                    let! path = dialogService.SaveFile suggestedName

                    match path with
                    | Some value ->
                        let! result = fileService.WriteText(value, contents)

                        match result with
                        | Ok () -> dispatch (AppCommand.fileSavedForDocument documentId revision value)
                        | Error message -> reportFailure message
                    | None ->
                        ()
                | WriteFile(path, contents) ->
                    let! result = fileService.WriteText(path, contents)

                    match result with
                    | Ok () ->
                        dispatch (AppCommand.fileSaved path)
                    | Error message ->
                        reportFailure message
                | WriteFileForDocument(documentId, revision, path, contents) ->
                    let! result = fileService.WriteText(path, contents)

                    match result with
                    | Ok () -> dispatch (AppCommand.fileSavedForDocument documentId revision path)
                    | Error message -> reportFailure message
                | Tokenize request ->
                    match tokenizerService with
                    | Some service ->
                        let coordinator = TokenizationCoordinator(service, dispatch)
                        let! result = coordinator.Execute(request, CancellationToken.None)

                        match result with
                        | Ok () ->
                            ()
                        | Error message ->
                            dispatch (AppCommand.reportError message)
                    | None ->
                        dispatch (AppCommand.reportError "No tokenizer service is configured.")
            with ex ->
                reportFailure ex.Message
        }
