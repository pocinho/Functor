namespace Functor.Application

open System
open System.Threading
open System.Threading.Tasks
open Functor.Domain.Core
open Functor.Domain.Document
open Functor.Domain.Syntax

type EditorSession(initialModel: CoreModel) =
    let mutable state = AppSessionState.empty initialModel
    let stateChanged = Event<AppSessionState>()
    let statusChanged = Event<SessionStatus>()
    let editorStatusChanged = Event<EditorStatus>()
    let effectsRequested = Event<AppEffect list>()

    let pendingSaves = EditorSessionPersistence()

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

    let tokenization = EditorSessionTokenization((fun () -> state), requestEffects)

    let updateModel event =
        state <- EditorSessionUpdate.apply event state

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
        | ApplySyntaxEvent(SetLanguage _) -> tokenization.RequestCurrent(CancellationToken.None)
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
        match Functor.Workspace.WorkspaceModel.tryFindDocumentByPath path state.Workspace with
        | Some existing -> updateModel (SwitchDocument existing.Document.Id)
        | None when isDirty () ->
            setPendingAction
                (PendingAction.OpenDocument path)
                "Unsaved changes must be confirmed before opening another file."
        | None -> requestEffects [ AppEffect.readFile path ]

    let requestOpenFolder () = requestEffects [ AppEffect.openFolder ]

    let replaceWorkspace path =
        tokenization.Reset()
        pendingSaves.Reset()

        state <-
            { state with
                Model = CoreModel.empty
                Workspace = Functor.Workspace.WorkspaceModel.create (Some path) }

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
