namespace Functor.Application

open Functor.Domain.Core
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Search
open System.Collections.Concurrent
open System.IO
open System
open System.Threading

/// Executes application effects through injected platform-neutral services.
type AppEffectInterpreter(services: EditorServices, dispatch: AppCommand -> unit) =
    let writeGates = ConcurrentDictionary<string, SemaphoreSlim>(StringComparer.OrdinalIgnoreCase)

    let writeText path contents =
        async {
            let gate =
                writeGates.GetOrAdd(
                    DocumentModel.canonicalizePath path,
                    fun _ -> new SemaphoreSlim(1, 1)
                )

            do! gate.WaitAsync(CancellationToken.None) |> Async.AwaitTask

            try
                return! services.File.WriteText(path, contents, CancellationToken.None)
            finally
                gate.Release() |> ignore
        }

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
                | AppEffect.NoEffect -> ()
                | AppEffect.NotifyStatus message -> dispatch (AppCommand.setStatus message)
                | AppEffect.NotifyError error -> dispatch (AppCommand.reportError error)
                | AppEffect.WriteClipboard text -> do! services.Clipboard.SetText(text, CancellationToken.None)
                | AppEffect.ReadClipboard ->
                    let! text = services.Clipboard.GetText(CancellationToken.None)

                    match text with
                    | Some value ->
                        dispatch (
                            AppCommand.toCoreEvent (ApplyEditingEvent(TextInput(TextInputEvent.InsertString value)))
                        )
                    | None -> ()
                | AppEffect.PasteText text ->
                    dispatch (AppCommand.toCoreEvent (ApplyEditingEvent(TextInput(TextInputEvent.InsertString text))))
                | AppEffect.OpenFile ->
                    let! path = services.Dialog.OpenFile(CancellationToken.None)

                    match path with
                    | Some value -> do! this.Execute(AppEffect.readFile value)
                    | None -> ()
                | AppEffect.OpenFolder ->
                    let! path = services.Dialog.OpenFolder(CancellationToken.None)

                    match path with
                    | Some value -> dispatch (AppCommand.folderOpened value)
                    | None -> ()
                | AppEffect.ReadFile path ->
                    let! result = services.File.ReadText(path, CancellationToken.None)

                    match result with
                    | Ok contents -> dispatch (AppCommand.fileOpened path contents)
                    | Error message -> reportFailure message
                | AppEffect.SaveFile(suggestedName, contents) ->
                    let! path = services.Dialog.SaveFile(suggestedName, CancellationToken.None)

                    match path with
                    | Some value -> do! this.Execute(AppEffect.writeFile value contents)
                    | None -> ()
                | AppEffect.SaveFileForDocument(documentId, revision, suggestedName, contents) ->
                    let! path = services.Dialog.SaveFile(suggestedName, CancellationToken.None)

                    match path with
                    | Some value ->
                        let! result = writeText value contents

                        match result with
                        | Ok() -> dispatch (AppCommand.fileSavedForDocument documentId revision value)
                        | Error message -> reportFailure message
                    | None -> ()
                | AppEffect.WriteFile(path, contents) ->
                    let! result = writeText path contents

                    match result with
                    | Ok() -> dispatch (AppCommand.fileSaved path)
                    | Error message -> reportFailure message
                | AppEffect.WriteFileForDocument(documentId, revision, path, contents) ->
                    let! result = writeText path contents

                    match result with
                    | Ok() -> dispatch (AppCommand.fileSavedForDocument documentId revision path)
                    | Error message -> reportFailure message
                | AppEffect.SearchWorkspace(request, cancellationToken) ->
                    cancellationToken.ThrowIfCancellationRequested()

                    let! pathsResult = services.File.EnumerateFiles(request.RootPath, cancellationToken)

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
                                  OpenDocumentRevisions = request.OpenDocumentRevisions }
                        )
                    | Ok paths ->
                        let openPaths = request.OpenDocumentPaths

                        let candidatePaths =
                            paths
                            |> List.distinct
                            |> List.sortWith (fun left right -> StringComparer.OrdinalIgnoreCase.Compare(left, right))

                        let matches = ResizeArray<SearchMatch>()
                        let documents = ResizeArray<SearchDocument>()

                        documents.AddRange(request.OpenDocuments)

                        matches.AddRange(
                            request.OpenDocuments
                            |> List.collect (SearchEngine.findInDocument request.Options)
                        )

                        let mutable sources =
                            request.OpenDocuments
                            |> List.choose (fun document ->
                                document.Path
                                |> Option.map (fun path ->
                                    DocumentModel.canonicalizePath path, String.concat "\n" document.Lines))
                            |> Map.ofList

                        let errors = ResizeArray<string>()

                        for path in candidatePaths do
                            cancellationToken.ThrowIfCancellationRequested()
                            let canonicalPath = DocumentModel.canonicalizePath path

                            let shouldSearch =
                                isWithinRoot canonicalPath
                                && FileType.isSearchablePath canonicalPath
                                && not (openPaths.Contains canonicalPath)

                            if shouldSearch then
                                let! readResult = services.File.ReadText(canonicalPath, cancellationToken)

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
                                  OpenDocumentRevisions = request.OpenDocumentRevisions }
                        )
                | AppEffect.ReplaceWorkspace(request, cancellationToken) ->
                    cancellationToken.ThrowIfCancellationRequested()
                    let replacedPaths = ResizeArray<string>(request.AlreadyReplacedPaths)
                    let mutable replacedMatches = request.AlreadyReplacedMatches
                    let mutable replacedFiles = request.AlreadyReplacedFiles
                    let stalePaths = ResizeArray<string>(request.StalePaths)
                    let errors = ResizeArray<string>()

                    for path, replacement in request.Replacements |> Map.toList do
                        cancellationToken.ThrowIfCancellationRequested()

                        match request.Sources.TryFind path with
                        | None -> errors.Add(sprintf "%s: source snapshot is unavailable." path)
                        | Some snapshot ->
                            let! readResult = services.File.ReadText(path, cancellationToken)

                            match readResult with
                            | Error message -> errors.Add(sprintf "%s: %s" path message)
                            | Ok current when current <> snapshot -> stalePaths.Add path
                            | Ok _ ->
                                let! writeResult = services.File.WriteText(path, replacement, cancellationToken)

                                match writeResult with
                                | Ok() ->
                                    replacedPaths.Add path

                                    replacedMatches <-
                                        replacedMatches
                                        + (request.MatchCounts |> Map.tryFind path |> Option.defaultValue 0)

                                    replacedFiles <- replacedFiles + 1
                                | Error message -> errors.Add(sprintf "%s: %s" path message)

                    dispatch (
                        AppCommand.workspaceReplacementCompleted
                            { RequestId = request.RequestId
                              WorkspaceId = request.WorkspaceId
                              ReplacedPaths = List.ofSeq replacedPaths
                              ReplacedMatches = replacedMatches
                              ReplacedFiles = replacedFiles
                              StalePaths = List.ofSeq stalePaths
                              Errors = List.ofSeq errors }
                    )
                | AppEffect.Tokenize(request, cancellationToken) ->
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
