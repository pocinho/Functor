namespace Functor.Application

open System
open System.Threading
open System.Threading.Tasks
open Functor.Domain.Core
open Functor.Domain.Document
open Functor.Domain.Syntax
open Functor.Domain.Navigation
open Functor.Domain.Search

type EditorSession(initialModel: CoreModel) =
    let mutable state = AppSessionState.empty initialModel
    let stateChanged = Event<AppSessionState>()
    let statusChanged = Event<SessionStatus>()
    let editorStatusChanged = Event<EditorStatus>()
    let effectsRequested = Event<AppEffect list>()

    let pendingSaves = EditorSessionPersistence()
    let mutable workspaceSearchRequestId: Guid option = None
    let mutable workspaceSearchCancellation = new CancellationTokenSource()
    let mutable workspaceReplacementRequestId: Guid option = None
    let mutable workspaceSearchResult: WorkspaceSearchResult option = None
    let mutable pendingSearchActivation: SearchMatch option = None
    let mutable globalSearch = initialModel.Navigation.Search

    let queuePendingSave documentId revision expectedPath buffer =
        pendingSaves.Queue(documentId, revision, expectedPath, buffer)

    let removePendingSave documentId revision expectedPath =
        pendingSaves.Remove(documentId, revision, expectedPath)

    let publishState () = stateChanged.Trigger(state)

    let publishEditorStatus () =
        editorStatusChanged.Trigger(EditorStatus.fromState state)

    let publishStatus () =
        statusChanged.Trigger(state.Status)
        publishEditorStatus ()

    let requestEffects (effects: AppEffect list) =
        if not effects.IsEmpty then
            effectsRequested.Trigger(effects)

    let cancelWorkspaceSearch () =
        workspaceSearchCancellation.Cancel()
        workspaceSearchCancellation.Dispose()
        workspaceSearchCancellation <- new CancellationTokenSource()

    let tokenization = EditorSessionTokenization((fun () -> state), requestEffects)

    let updateModel event =
        state <- EditorSessionUpdate.apply event state

        match event with
        | ApplyNavigationEvent _
        | ApplyEditingEvent _ -> globalSearch <- state.Model.Navigation.Search
        | _ -> ()

        state <-
            { state with
                Model =
                    { state.Model with
                        Navigation =
                            { state.Model.Navigation with
                                Search = globalSearch } } }

        publishState ()
        publishEditorStatus ()

        match event with
        | LoadDocument _
        | SwitchDocument _
        | NewDocument _ ->
            tokenization.Cancel()
            tokenization.RequestCurrent(CancellationToken.None)
        | CloseDocument documentId ->
            tokenization.RemoveDocument(documentId)
            pendingSaves.RemoveDocument(documentId)
        | ApplyEditingEvent _ ->
            cancelWorkspaceSearch ()

            match state.Model.Editing.LastChange with
            | Some change ->
                tokenization.BeginEdit(
                    state.Model.ActiveDocument.Value.Id,
                    state.Model.Editing.Revision - 1L,
                    state.Model.Editing.Revision,
                    change
                )
            | None -> ()

            tokenization.Schedule()
        | ApplySyntaxEvent(SyntaxEvent.Language(LanguageEvent.SetLanguage _)) ->
            tokenization.RequestCurrent(CancellationToken.None)
        | _ -> ()

    let isDirty () = state.Model.Editing.IsDirty

    let setPendingAction action message =
        state <-
            { state with
                Status =
                    { state.Status with
                        Message = Some message
                        PendingAction = Some action } }

        publishState ()
        publishStatus ()

    let clearPendingAction () =
        state <-
            { state with
                Status =
                    { state.Status with
                        Message = None
                        PendingAction = None } }

        publishState ()
        publishStatus ()

    let requestOpenFile () =
        if isDirty () then
            setPendingAction PendingAction.OpenFile "Unsaved changes must be confirmed before opening another file."
        else
            requestEffects [ AppEffect.openFile ]

    let requestOpenDocument path =
        let canonicalPath = DocumentModel.canonicalizePath path

        if not (FileType.isOpenablePath canonicalPath) then
            state <-
                AppSessionState.withMessage
                    (sprintf "Unsupported or binary file cannot be opened: %s" canonicalPath)
                    state

            publishState ()
            publishStatus ()
        else
            match Functor.Workspace.WorkspaceModel.tryFindDocumentByPath canonicalPath state.Workspace with
            | Some existing -> updateModel (SwitchDocument existing.Document.Id)
            | None when isDirty () ->
                setPendingAction
                    (PendingAction.OpenDocument canonicalPath)
                    "Unsaved changes must be confirmed before opening another file."
            | None -> requestEffects [ AppEffect.readFile canonicalPath ]

    let requestOpenFolder () = requestEffects [ AppEffect.openFolder ]

    let refreshActiveSearch () =
        match state.Model.ActiveDocument with
        | Some document ->
            let options = state.Model.Navigation.Search.Options

            let searchDocument: SearchDocument =
                { Id = document.Id
                  Path = document.Metadata.Path
                  Name = document.Metadata.Name
                  Lines = state.Model.Editing.Buffer }

            let matches = SearchEngine.findInDocument options searchDocument

            updateModel (ApplyNavigationEvent(NavigationEvent.SetSearchMatches(state.Model.Editing.Revision, matches)))
        | None -> updateModel (ApplyNavigationEvent NavigationEvent.InvalidateSearch)

    let revealActiveSearchMatch () =
        match state.Model.Navigation.Search.Index with
        | Some index when index >= 0 && index < state.Model.Navigation.Search.Matches.Length ->
            let matchValue = state.Model.Navigation.Search.Matches[index]
            updateModel (ApplyEditingEvent(Functor.Domain.Editing.EditingEvent.SetSelection(Some matchValue.Range)))

            state <-
                { state with
                    Model =
                        { state.Model with
                            View =
                                { state.Model.View with
                                    VerticalOffset = matchValue.Line } } }

            publishState ()
        | _ -> ()

    let activateSearchMatch (matchValue: SearchMatch) =
        state.Model.Navigation.Search.Matches
        |> List.tryFindIndex ((=) matchValue)
        |> Option.iter (fun index -> updateModel (ApplyNavigationEvent(NavigationEvent.SetSearchIndex index)))

        let activate documentId =
            updateModel (SwitchDocument documentId)
            updateModel (ApplyEditingEvent(Functor.Domain.Editing.EditingEvent.SetSelection(Some matchValue.Range)))

            state <-
                { state with
                    Model =
                        { state.Model with
                            View =
                                { state.Model.View with
                                    VerticalOffset = matchValue.Line } } }

            publishState ()

        match matchValue.Path with
        | Some path ->
            let canonicalPath = DocumentModel.canonicalizePath path

            match Functor.Workspace.WorkspaceModel.tryFindDocumentByPath canonicalPath state.Workspace with
            | Some document -> activate document.Document.Id
            | None ->
                pendingSearchActivation <- Some matchValue
                requestOpenDocument canonicalPath
        | None ->
            match Functor.Workspace.WorkspaceModel.tryFindDocument matchValue.DocumentId state.Workspace with
            | Some document -> activate document.Document.Id
            | None -> ()

    let replacementMatchesCurrentDocument matches =
        match state.Model.ActiveDocument, state.Model.Navigation.Search.Revision with
        | Some document, Some revision when revision = state.Model.Editing.Revision ->
            matches
            |> List.forall (fun matchValue -> matchValue.DocumentId = document.Id)
            |> fun matchesCurrentDocument -> matchesCurrentDocument, document
        | _ -> false, Unchecked.defaultof<_>

    let replaceMatches (matches: SearchMatch list) replacement =
        let matchesCurrentDocument, _ = replacementMatchesCurrentDocument matches

        if matchesCurrentDocument then
            matches
            |> List.sortByDescending (fun matchValue -> matchValue.Line, matchValue.Column)
            |> List.iter (fun matchValue ->
                updateModel (
                    ApplyEditingEvent(Functor.Domain.Editing.EditingEvent.SetSelection(Some matchValue.Range))
                )

                updateModel (ApplyEditingEvent(Functor.Domain.Editing.EditingEvent.InsertString replacement)))

            refreshActiveSearch ()

    let replaceCurrentSearch replacement =
        match state.Model.Navigation.Search.Index with
        | Some index when index >= 0 && index < state.Model.Navigation.Search.Matches.Length ->
            replaceMatches [ state.Model.Navigation.Search.Matches[index] ] replacement
        | _ -> ()

    let replaceCurrentDocumentAllSearch replacement =
        replaceMatches state.Model.Navigation.Search.Matches replacement

    let replaceOpenWorkspaceMatches replacement (result: WorkspaceSearchResult) =
        let openDocumentIds =
            state.Workspace.Documents |> Map.toSeq |> Seq.map fst |> Set.ofSeq

        let originalDocumentId = state.Workspace.ActiveDocumentId
        let mutable replacedCount = 0
        let stalePaths = ResizeArray<string>()

        result.Matches
        |> List.filter (fun matchValue -> openDocumentIds.Contains matchValue.DocumentId)
        |> List.groupBy (fun matchValue -> matchValue.DocumentId)
        |> List.iter (fun (documentId, matches) ->
            let revisionIsCurrent =
                result.OpenDocumentRevisions
                |> Map.tryFind documentId
                |> Option.exists (fun revision -> state.Workspace.Documents[documentId].Editing.Revision = revision)

            if revisionIsCurrent then
                if state.Workspace.ActiveDocumentId <> Some documentId then
                    updateModel (SwitchDocument documentId)

                matches
                |> List.sortByDescending (fun matchValue -> matchValue.Line, matchValue.Column)
                |> List.iter (fun matchValue ->
                    updateModel (
                        ApplyEditingEvent(Functor.Domain.Editing.EditingEvent.SetSelection(Some matchValue.Range))
                    )

                    updateModel (ApplyEditingEvent(Functor.Domain.Editing.EditingEvent.InsertString replacement))
                    replacedCount <- replacedCount + 1)
            else
                let path =
                    state.Workspace.Documents[documentId].Document.Metadata.Path
                    |> Option.defaultValue (sprintf "document %O" documentId)

                stalePaths.Add path)

        match originalDocumentId with
        | Some documentId when state.Workspace.ActiveDocumentId <> Some documentId ->
            updateModel (SwitchDocument documentId)
        | _ -> ()

        replacedCount, List.ofSeq stalePaths

    let replaceWorkspaceSearch replacement =
        match state.Workspace.RootPath, workspaceSearchResult with
        | Some _, Some result ->
            let openDocumentIds =
                state.Workspace.Documents |> Map.toSeq |> Seq.map fst |> Set.ofSeq

            let unopenedMatches =
                result.Matches
                |> List.filter (fun matchValue -> not (openDocumentIds.Contains matchValue.DocumentId))

            let openReplacementCount, staleOpenPaths =
                replaceOpenWorkspaceMatches replacement result

            if unopenedMatches.IsEmpty then
                if openReplacementCount > 0 then
                    updateModel (ApplyNavigationEvent NavigationEvent.InvalidateSearch)

                if not staleOpenPaths.IsEmpty then
                    state <-
                        AppSessionState.withMessage (sprintf "Skipped %d stale file(s)." staleOpenPaths.Length) state

                    publishState ()
                    publishStatus ()
            else
                match WorkspaceReplacement.apply replacement result.Sources unopenedMatches with
                | Error errors ->
                    state <- AppSessionState.withError (String.concat "\n" errors) state
                    publishState ()
                    publishStatus ()
                | Ok replacements ->
                    let requestId = Guid.NewGuid()

                    workspaceReplacementRequestId <- Some requestId

                    requestEffects
                        [ AppEffect.replaceWorkspace
                              { RequestId = requestId
                                WorkspaceId = state.Workspace.Id
                                StalePaths = staleOpenPaths
                                Sources = result.Sources
                                Replacements = replacements } ]
        | _ -> replaceCurrentDocumentAllSearch replacement

    let replaceAllSearch replacement = replaceWorkspaceSearch replacement

    let requestWorkspaceSearch () =
        cancelWorkspaceSearch ()

        match state.Workspace.RootPath with
        | None ->
            state <- AppSessionState.withMessage "Open a workspace before searching workspace files." state
            publishState ()
            publishStatus ()
        | Some rootPath ->
            let requestId = Guid.NewGuid()

            let openDocuments =
                state.Workspace.TabOrder
                |> List.choose (fun documentId ->
                    state.Workspace.Documents
                    |> Map.tryFind documentId
                    |> Option.map (fun documentState ->
                        { Id = documentState.Document.Id
                          Path = documentState.Document.Metadata.Path
                          Name = documentState.Document.Metadata.Name
                          Lines = documentState.Editing.Buffer }))

            let openDocumentPaths =
                openDocuments
                |> List.choose (fun document -> document.Path)
                |> List.map DocumentModel.canonicalizePath
                |> Set.ofList

            let openDocumentRevisions =
                openDocuments
                |> List.choose (fun document ->
                    state.Workspace.Documents
                    |> Map.tryFind document.Id
                    |> Option.map (fun documentState -> document.Id, documentState.Editing.Revision))
                |> Map.ofList

            let request: WorkspaceSearchRequest =
                { RequestId = requestId
                  WorkspaceId = state.Workspace.Id
                  RootPath = rootPath
                  Options = state.Model.Navigation.Search.Options
                  OpenDocuments = openDocuments
                  OpenDocumentPaths = openDocumentPaths
                  OpenDocumentRevisions = openDocumentRevisions }

            workspaceSearchRequestId <- Some requestId
            requestEffects [ AppEffect.searchWorkspaceWithCancellation request workspaceSearchCancellation.Token ]

    let searchOpenDocuments () =
        workspaceSearchResult <- None

        let documents =
            state.Workspace.TabOrder
            |> List.choose (fun documentId ->
                state.Workspace.Documents
                |> Map.tryFind documentId
                |> Option.map (fun documentState ->
                    { Id = documentState.Document.Id
                      Path = documentState.Document.Metadata.Path
                      Name = documentState.Document.Metadata.Name
                      Lines = documentState.Editing.Buffer }))

        let matches =
            SearchEngine.findInDocuments state.Model.Navigation.Search.Options documents

        updateModel (ApplyNavigationEvent(NavigationEvent.SetSearchMatches(state.Model.Editing.Revision, matches)))

    let searchCurrentScope () =
        match state.Workspace.RootPath with
        | Some _ -> requestWorkspaceSearch ()
        | None -> searchOpenDocuments ()

    let replaceWorkspace path =
        cancelWorkspaceSearch ()
        tokenization.Reset()
        pendingSaves.Reset()

        state <-
            { state with
                Model = CoreModel.empty
                Workspace = Functor.Workspace.WorkspaceModel.create (Some path) }

        globalSearch <- SearchModel.create ()

        workspaceSearchRequestId <- None
        workspaceReplacementRequestId <- None
        workspaceSearchResult <- None

        publishState ()
        publishEditorStatus ()

    let requestCloseDocument () =
        match state.Model.ActiveDocument with
        | Some document when isDirty () ->
            setPendingAction
                (PendingAction.CloseDocument document.Id)
                "Unsaved changes must be confirmed before closing the document."
        | Some document -> updateModel (CoreEvent.CloseDocument document.Id)
        | None -> ()

    let requestReopenClosedTab () =
        match state.Workspace.RecentlyClosedDocuments with
        | closed :: _ -> requestEffects [ AppEffect.readFile closed.Path ]
        | [] ->
            state <- AppSessionState.withMessage "No recently closed tab is available." state
            publishState ()
            publishStatus ()

    new() = EditorSession(CoreModel.empty)

    member _.State = state

    member _.Model = state.Model

    member _.Status = state.Status

    member _.EditorStatus = EditorStatus.fromState state

    member _.StateChanged = stateChanged.Publish

    member _.StatusChanged = statusChanged.Publish

    member _.EditorStatusChanged = editorStatusChanged.Publish

    member _.EffectsRequested = effectsRequested.Publish

    member _.Dispatch(event: CoreEvent) = updateModel event

    member _.DispatchCommand(command: AppCommand) =
        match command with
        | ExecuteCoreEvent evt -> updateModel evt
        | SetStatus message ->
            state <- AppSessionState.withMessage message state
            publishState ()
            publishStatus ()
        | ClearStatus ->
            state <-
                { state with
                    Status = { state.Status with Message = None } }

            publishState ()
            publishStatus ()
        | ReportError error ->
            state <- AppSessionState.withError error state
            publishState ()
            publishStatus ()
        | NewDocumentRequested -> updateModel (CoreEvent.NewDocument "untitled")
        | OpenFileRequested -> requestOpenFile ()
        | OpenDocumentRequested path -> requestOpenDocument path
        | OpenFolderRequested -> requestOpenFolder ()
        | SaveFileRequested ->
            match state.Model.ActiveDocument with
            | Some document when document.Metadata.Path.IsSome ->
                let path = document.Metadata.Path.Value
                let revision = state.Model.Editing.Revision
                let contents = String.concat "\n" state.Model.Editing.Buffer
                queuePendingSave document.Id revision (Some path) state.Model.Editing.Buffer
                requestEffects [ AppEffect.writeFileForDocument document.Id revision path contents ]
            | Some document ->
                let contents = String.concat "\n" state.Model.Editing.Buffer
                queuePendingSave document.Id state.Model.Editing.Revision None state.Model.Editing.Buffer
                requestEffects [ AppEffect.saveFile (Some document.Metadata.Name) contents ]
            | None -> ()
        | SaveFileAsRequested ->
            let contents = String.concat "\n" state.Model.Editing.Buffer

            match state.Model.ActiveDocument with
            | Some document ->
                queuePendingSave document.Id state.Model.Editing.Revision None state.Model.Editing.Buffer
                let suggestedName = Some document.Metadata.Name

                requestEffects
                    [ AppEffect.saveFileForDocument document.Id state.Model.Editing.Revision suggestedName contents ]
            | None -> ()
        | CloseDocumentRequested -> requestCloseDocument ()
        | ReopenClosedTabRequested -> requestReopenClosedTab ()
        | ReopenRecentDocument path -> requestEffects [ AppEffect.readFile path ]
        | ClearRecentDocumentsRequested ->
            state <-
                { state with
                    Workspace =
                        Functor.Workspace.WorkspaceLogic.update Functor.Workspace.ClearRecentlyClosed state.Workspace }

            publishState ()
        | OpenCommandPaletteRequested -> ()
        | OpenSearchRequested -> ()
        | OpenSearchReplaceRequested -> ()
        | OpenSearchReplaceAllRequested -> ()
        | OpenSettingsRequested -> ()
        | ConfirmDiscardChanges ->
            match state.Status.PendingAction with
            | Some action ->
                clearPendingAction ()

                match action with
                | PendingAction.NewDocument -> updateModel (CoreEvent.NewDocument "untitled")
                | PendingAction.OpenFile -> requestEffects [ AppEffect.openFile ]
                | PendingAction.OpenDocument path -> requestEffects [ AppEffect.readFile path ]
                | PendingAction.OpenWorkspace path -> replaceWorkspace path
                | PendingAction.CloseDocument id -> updateModel (CoreEvent.CloseDocument id)
            | None -> ()
        | CancelPendingOperation ->
            if state.Status.PendingAction.IsSome then
                clearPendingAction ()
        | FileOpened(path, contents) ->
            clearPendingAction ()

            match Functor.Workspace.WorkspaceModel.tryFindDocumentByPath path state.Workspace with
            | Some existing -> updateModel (SwitchDocument existing.Document.Id)
            | None -> updateModel (LoadDocument(path, contents))

            state <-
                { state with
                    Workspace =
                        Functor.Workspace.WorkspaceLogic.update
                            (Functor.Workspace.ConsumeRecentlyClosed path)
                            state.Workspace }

            publishState ()

            match pendingSearchActivation with
            | Some matchValue when
                matchValue.Path
                |> Option.map DocumentModel.canonicalizePath
                |> Option.exists (fun expected -> expected = DocumentModel.canonicalizePath path)
                ->
                pendingSearchActivation <- None

                activateSearchMatch
                    { matchValue with
                        Path = Some(DocumentModel.canonicalizePath path) }
            | _ -> ()
        | FolderOpened path ->
            if isDirty () then
                setPendingAction
                    (PendingAction.OpenWorkspace path)
                    "Unsaved changes must be confirmed before opening another workspace."
            else
                replaceWorkspace path
        | FileSaved path ->
            clearPendingAction ()
            let canonicalPath = DocumentModel.canonicalizePath path

            let pendingDocument =
                pendingSaves.TryFindFileCompletion(
                    path,
                    state.Model.ActiveDocument |> Option.map (fun document -> document.Id),
                    state.Model.Editing.Revision
                )

            match pendingDocument, state.Model.ActiveDocument with
            | Some(documentId, revision, _), Some document when
                document.Id = documentId && state.Model.Editing.Revision = revision
                ->
                removePendingSave documentId revision (Some canonicalPath)
                updateModel (ApplyDocumentEvent(SetDocumentPath(Some canonicalPath)))
                updateModel (ApplyDocumentEvent MarkDocumentClean)
            | _ -> ()
        | FileSavedForDocument(documentId, revision, path) ->
            clearPendingAction ()
            let canonicalPath = DocumentModel.canonicalizePath path

            let matchesPendingSave =
                pendingSaves.TryFindDocumentCompletion(documentId, revision, path)

            match Functor.Workspace.WorkspaceModel.tryFindDocument documentId state.Workspace, matchesPendingSave with
            | Some documentState, Some(pendingRevision, expectedPath, savedBuffer) when pendingRevision = revision ->
                let savedDocument =
                    documentState.Document
                    |> DocumentLogic.update (SetDocumentPath(Some canonicalPath))

                let savedDocument =
                    if documentState.Editing.Buffer = savedBuffer then
                        DocumentLogic.update MarkDocumentClean savedDocument
                    else
                        DocumentLogic.update MarkDocumentDirty savedDocument

                let savedEditing =
                    { documentState.Editing with
                        SavedBuffer = savedBuffer
                        IsDirty = documentState.Editing.Buffer <> savedBuffer }

                let savedState =
                    { documentState with
                        Document = savedDocument
                        Editing = savedEditing }

                removePendingSave documentId revision expectedPath

                state <-
                    { state with
                        Workspace =
                            Functor.Workspace.WorkspaceLogic.update
                                (Functor.Workspace.ReplaceDocumentState savedState)
                                state.Workspace }

                if
                    state.Model.ActiveDocument
                    |> Option.exists (fun document -> document.Id = documentId)
                then
                    state <-
                        { state with
                            Model = EditorSessionUpdate.modelFromWorkspace state.Workspace }

                publishState ()
                publishEditorStatus ()
            | _ -> ()
        | FileOperationFailed message ->
            state <- AppSessionState.withError message state
            publishState ()
            publishStatus ()
        | RequestTokenization language ->
            match state.Model.ActiveDocument with
            | Some document ->
                let request: TokenizationRequest =
                    { DocumentId = document.Id
                      Revision = state.Model.Editing.Revision
                      Language = language
                      Scope = FullDocument
                      Lines = state.Model.Editing.Buffer
                      InitialState = Initial }

                requestEffects [ AppEffect.tokenize request ]
            | None -> ()
        | TokenizationCompleted result ->
            let matchesCurrentDocument =
                state.Model.ActiveDocument
                |> Option.exists (fun document ->
                    document.Id = result.DocumentId
                    && state.Model.Editing.Revision = result.Revision)

            let stable =
                match tokenization.RequestRange, matchesCurrentDocument with
                | Some(_, endLine), true -> tokenization.IsStable(result, state.Model.Editing.Buffer.Length, endLine)
                | _ -> false

            if matchesCurrentDocument then
                tokenization.RecordCompletion(result)

            let syntaxEvent =
                match result.Scope with
                | FullDocument -> SetTokens(result.DocumentId, result.Revision, result.Tokens)
                | Line line -> SetTokenRange(result.DocumentId, result.Revision, line, line, result.Tokens)
                | LineRange(startLine, endLine) ->
                    SetTokenRange(result.DocumentId, result.Revision, startLine, endLine, result.Tokens)

            if matchesCurrentDocument then
                updateModel (ApplySyntaxEvent syntaxEvent)

                match tokenization.RequestRange, stable with
                | Some(_, endLine), true ->
                    tokenization.ClearRange()
                    updateModel (ApplySyntaxEvent(MarkSyntaxCleanFrom(endLine + 1)))
                | Some(startLine, endLine), false ->
                    tokenization.Extend(startLine, endLine)
                    tokenization.RequestCurrent(CancellationToken.None)
                | None, _ -> ()
        | ToggleAgentPanel ->
            match state.Workspace.ActiveDocumentId with
            | Some documentId ->
                let isOpen = state.Workspace.Documents[documentId].Auxiliary.Agent.IsOpen

                state <-
                    { state with
                        Workspace =
                            Functor.Workspace.WorkspaceLogic.update
                                (Functor.Workspace.SetAgentOpen(documentId, not isOpen))
                                state.Workspace }

                publishState ()
            | None -> ()
        | SetAgentOpen(documentId, isOpen) ->
            state <-
                { state with
                    Workspace =
                        Functor.Workspace.WorkspaceLogic.update
                            (Functor.Workspace.SetAgentOpen(documentId, isOpen))
                            state.Workspace }

            publishState ()
        | SearchQueryChanged query ->
            workspaceSearchResult <- None
            updateModel (ApplyNavigationEvent(NavigationEvent.SetSearchQuery query))
            refreshActiveSearch ()
            searchCurrentScope ()
        | SearchOptionsChanged options ->
            workspaceSearchResult <- None
            updateModel (ApplyNavigationEvent(NavigationEvent.SetSearchOptions options))
            refreshActiveSearch ()
            searchCurrentScope ()
        | RefreshSearchRequested -> searchCurrentScope ()
        | NextSearchResultRequested ->
            updateModel (ApplyNavigationEvent NavigationEvent.NextSearchResult)
            revealActiveSearchMatch ()
        | PreviousSearchResultRequested ->
            updateModel (ApplyNavigationEvent NavigationEvent.PrevSearchResult)
            revealActiveSearchMatch ()
        | SearchResultActivated result -> activateSearchMatch result
        | ReplaceCurrentSearch replacement -> replaceCurrentSearch replacement
        | ReplaceAllSearch replacement -> replaceAllSearch replacement
        | ClearSearchRequested ->
            workspaceSearchResult <- None
            updateModel (ApplyNavigationEvent NavigationEvent.ClearSearch)
        | ClearSearchHistoryRequested -> updateModel (ApplyNavigationEvent NavigationEvent.ClearSearchHistory)
        | OpenDocumentsSearchRequested -> searchOpenDocuments ()
        | WorkspaceSearchRequested -> requestWorkspaceSearch ()
        | WorkspaceSearchCompleted result ->
            let isCurrentRequest = workspaceSearchRequestId = Some result.RequestId
            let isCurrentWorkspace = state.Workspace.Id = result.WorkspaceId
            let isCurrentOptions = state.Model.Navigation.Search.Options = result.Options

            let areOpenDocumentRevisionsCurrent =
                result.OpenDocumentRevisions
                |> Map.forall (fun documentId revision ->
                    state.Workspace.Documents
                    |> Map.tryFind documentId
                    |> Option.exists (fun documentState -> documentState.Editing.Revision = revision))

            if
                isCurrentRequest
                && isCurrentWorkspace
                && isCurrentOptions
                && areOpenDocumentRevisionsCurrent
            then
                workspaceSearchResult <- Some result

                updateModel (
                    ApplyNavigationEvent(NavigationEvent.SetSearchMatches(state.Model.Editing.Revision, result.Matches))
                )

                if not result.Errors.IsEmpty then
                    state <- AppSessionState.withMessage (String.concat "\n" result.Errors) state
                    publishState ()
                    publishStatus ()
        | WorkspaceReplacementCompleted result when workspaceReplacementRequestId = Some result.RequestId ->
            workspaceReplacementRequestId <- None
            workspaceSearchResult <- None
            updateModel (ApplyNavigationEvent NavigationEvent.InvalidateSearch)

            let messages =
                [ if not result.ReplacedPaths.IsEmpty then
                      yield sprintf "Replaced matches in %d file(s)." result.ReplacedPaths.Length
                  if not result.StalePaths.IsEmpty then
                      yield sprintf "Skipped %d stale file(s)." result.StalePaths.Length
                  if not result.Errors.IsEmpty then
                      yield String.concat "\n" result.Errors ]

            if not messages.IsEmpty then
                state <- AppSessionState.withMessage (String.concat " " messages) state
                publishState ()
                publishStatus ()

            if result.StalePaths.IsEmpty && result.Errors.IsEmpty then
                requestWorkspaceSearch ()
        | WorkspaceReplacementCompleted _ -> ()

    member _.DispatchEffect(effect: AppEffect) =
        match effect with
        | NoEffect -> ()
        | NotifyStatus message ->
            state <- AppSessionState.withMessage message state
            publishState ()
            publishStatus ()
        | NotifyError error ->
            state <- AppSessionState.withError error state
            publishState ()
            publishStatus ()
        | WriteClipboard _ -> ()
        | ReadClipboard -> ()
        | PasteText _ -> ()
        | OpenFile -> ()
        | OpenFolder -> ()
        | ReadFile _ -> ()
        | SaveFile _ -> ()
        | SaveFileForDocument _ -> ()
        | WriteFile _ -> ()
        | WriteFileForDocument _ -> ()
        | SearchWorkspace _ -> ()
        | ReplaceWorkspace _ -> ()
        | Tokenize _ -> ()

    member _.SetStatus(message: string) =
        state <- AppSessionState.withMessage message state
        publishState ()
        publishStatus ()

    member _.ClearStatus() =
        state <-
            { state with
                Status = { state.Status with Message = None } }

        publishState ()
        publishStatus ()

    member _.ReportError(error: string) =
        state <- AppSessionState.withError error state
        publishState ()
        publishStatus ()
