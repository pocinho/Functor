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

                    let! pathsResult =
                        if request.CandidatePaths.IsEmpty then
                            services.File.EnumerateFiles request.RootPath
                        else
                            async { return Ok request.CandidatePaths }

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
                        let openDocumentMatches =
                            request.OpenDocuments
                            |> List.collect (SearchEngine.findInDocument request.Options)

                        let sources =
                            request.OpenDocuments
                            |> List.choose (fun document ->
                                document.Path
                                |> Option.map (fun path ->
                                    DocumentModel.canonicalizePath path, String.concat "\n" document.Lines))
                            |> Map.ofList

                        dispatch (
                            AppCommand.workspaceSearchCompleted
                                { RequestId = request.RequestId
                                  WorkspaceId = request.WorkspaceId
                                  Options = request.Options
                                  Documents = request.OpenDocuments
                                  Sources = sources
                                  Matches = openDocumentMatches
                                  Errors = [ message ]
                                  OpenDocumentRevisions = request.OpenDocumentRevisions
                                  CandidatePaths = []
                                  NextOffset = request.Offset
                                  IsComplete = true }
                        )
                    | Ok paths ->
                        let openPaths = request.OpenDocumentPaths

                        let candidatePaths =
                            if request.CandidatePaths.IsEmpty then
                                paths
                                |> List.distinct
                                |> List.sortWith (fun left right ->
                                    StringComparer.OrdinalIgnoreCase.Compare(left, right))
                            else
                                request.CandidatePaths

                        let batchPaths =
                            candidatePaths
                            |> List.skip (min request.Offset candidatePaths.Length)
                            |> List.truncate (max 1 request.BatchSize)

                        let includeOpenDocuments = request.Offset = 0

                        let matches = ResizeArray<SearchMatch>()
                        let documents = ResizeArray<SearchDocument>()

                        if includeOpenDocuments then
                            documents.AddRange(request.OpenDocuments)

                            matches.AddRange(
                                request.OpenDocuments
                                |> List.collect (SearchEngine.findInDocument request.Options)
                            )

                        let mutable sources =
                            if includeOpenDocuments then
                                request.OpenDocuments
                                |> List.choose (fun document ->
                                    document.Path
                                    |> Option.map (fun path ->
                                        DocumentModel.canonicalizePath path, String.concat "\n" document.Lines))
                                |> Map.ofList
                            else
                                Map.empty

                        let errors = ResizeArray<string>()

                        for path in batchPaths do
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

                                    documents.Add(document)
                                    sources <- sources.Add(canonicalPath, contents)
                                    matches.AddRange(SearchEngine.findInDocument request.Options document)
                                | Error message -> errors.Add(sprintf "%s: %s" canonicalPath message)

                        dispatch (
                            AppCommand.workspaceSearchCompleted
                                { RequestId = request.RequestId
                                  WorkspaceId = request.WorkspaceId
                                  Options = request.Options
                                  Documents = List.ofSeq documents
                                  Sources = sources
                                  Matches = List.ofSeq matches
                                  Errors = List.ofSeq errors
                                  OpenDocumentRevisions = request.OpenDocumentRevisions
                                  CandidatePaths = candidatePaths
                                  NextOffset = request.Offset + batchPaths.Length
                                  IsComplete = request.Offset + batchPaths.Length >= candidatePaths.Length }
                        )
                | ReplaceWorkspace(request, cancellationToken) ->
                    cancellationToken.ThrowIfCancellationRequested()
                    let replacedPaths = ResizeArray<string>()
                    let stalePaths = ResizeArray<string>(request.StalePaths)
                    let errors = ResizeArray<string>()

                    for path, replacement in request.Replacements |> Map.toList do
                        cancellationToken.ThrowIfCancellationRequested()

                        match request.Sources.TryFind path with
                        | None -> errors.Add(sprintf "%s: source snapshot is unavailable." path)
                        | Some snapshot ->
                            let! readResult = services.File.ReadText path

                            match readResult with
                            | Error message -> errors.Add(sprintf "%s: %s" path message)
                            | Ok current when current <> snapshot -> stalePaths.Add path
                            | Ok _ ->
                                let! writeResult = services.File.WriteText(path, replacement)

                                match writeResult with
                                | Ok() -> replacedPaths.Add path
                                | Error message -> errors.Add(sprintf "%s: %s" path message)

                    dispatch (
                        AppCommand.workspaceReplacementCompleted
                            { RequestId = request.RequestId
                              WorkspaceId = request.WorkspaceId
                              ReplacedPaths = List.ofSeq replacedPaths
                              StalePaths = List.ofSeq stalePaths
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
