namespace Functor.Application

open Functor.Domain.Core
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Search
open System.IO
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
                    | Some value ->
                        dispatch (
                            AppCommand.toCoreEvent (ApplyEditingEvent(TextInput(TextInputEvent.InsertString value)))
                        )
                    | None -> ()
                | PasteText text ->
                    dispatch (AppCommand.toCoreEvent (ApplyEditingEvent(TextInput(TextInputEvent.InsertString text))))
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
                | SearchWorkspace(request, cancellationToken) ->
                    cancellationToken.ThrowIfCancellationRequested()
                    let! pathsResult = services.File.EnumerateFiles request.RootPath
                    let canonicalRoot = DocumentModel.canonicalizePath request.RootPath

                    let isWithinRoot path =
                        let relativePath = Path.GetRelativePath(canonicalRoot, path)

                        not (Path.IsPathRooted relativePath)
                        && relativePath <> ".."
                        && not (
                            relativePath.StartsWith(".." + string Path.DirectorySeparatorChar, StringComparison.Ordinal)
                        )
                        && not (
                            relativePath.StartsWith(
                                ".." + string Path.AltDirectorySeparatorChar,
                                StringComparison.Ordinal
                            )
                        )

                    match pathsResult with
                    | Error message ->
                        dispatch (
                            AppCommand.workspaceSearchCompleted
                                { RequestId = request.RequestId
                                  WorkspaceId = request.WorkspaceId
                                  Options = request.Options
                                  Matches =
                                    request.OpenDocuments
                                    |> List.collect (SearchEngine.findInDocument request.Options)
                                  Errors = [ message ] }
                        )
                    | Ok paths ->
                        let openPaths = request.OpenDocumentPaths

                        let orderedPaths =
                            paths
                            |> List.distinct
                            |> List.sortWith (fun left right -> StringComparer.OrdinalIgnoreCase.Compare(left, right))

                        let mutable matches =
                            request.OpenDocuments
                            |> List.collect (SearchEngine.findInDocument request.Options)

                        let errors = ResizeArray<string>()

                        for path in orderedPaths do
                            cancellationToken.ThrowIfCancellationRequested()
                            let canonicalPath = DocumentModel.canonicalizePath path

                            let shouldSearch =
                                isWithinRoot canonicalPath
                                && FileType.isSearchablePath canonicalPath
                                && not (openPaths.Contains canonicalPath)

                            if shouldSearch then
                                let! readResult = services.File.ReadText canonicalPath

                                match readResult with
                                | Ok contents ->
                                    let document: SearchDocument =
                                        { Id = Guid.NewGuid()
                                          Path = Some canonicalPath
                                          Name = Path.GetFileName canonicalPath
                                          Lines =
                                            contents.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n')
                                            |> Array.toList }

                                    matches <- matches @ SearchEngine.findInDocument request.Options document
                                | Error message -> errors.Add(sprintf "%s: %s" canonicalPath message)

                        dispatch (
                            AppCommand.workspaceSearchCompleted
                                { RequestId = request.RequestId
                                  WorkspaceId = request.WorkspaceId
                                  Options = request.Options
                                  Matches = matches
                                  Errors = List.ofSeq errors }
                        )
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
