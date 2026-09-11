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
    let mutable pendingTokenization: CancellationTokenSource option = None
    let mutable incrementalState = IncrementalTokenizationState.empty
    let mutable pendingSaves: Map<DocumentId, int64 * string option> = Map.empty
    let tokenizationDelay = 150

    let publishState() = stateChanged.Trigger(state)

    let publishEditorStatus() = editorStatusChanged.Trigger(EditorStatus.fromState state)

    let publishStatus() =
        statusChanged.Trigger(state.Status)
        publishEditorStatus()

    let syncActiveDocumentToWorkspace currentState =
        match currentState.Model.ActiveDocument with
        | Some document when Functor.Workspace.WorkspaceModel.containsDocument document.Id currentState.Workspace ->
            let documentState: Functor.Workspace.PerDocumentSessionState =
                { Document = document
                  Editing = currentState.Model.Editing
                  Syntax = currentState.Model.Syntax
                  Navigation = currentState.Model.Navigation
                  Diagnostics = currentState.Model.Diagnostics
                  Mode = currentState.Model.Mode
                  View = currentState.Model.View }

            { currentState with
                Workspace =
                    Functor.Workspace.WorkspaceLogic.update
                        (Functor.Workspace.ReplaceDocumentState documentState)
                        currentState.Workspace }
        | _ -> currentState

    let modelFromWorkspace (workspace: Functor.Workspace.WorkspaceModel) : CoreModel =
        let openDocuments: Functor.Workspace.PerDocumentSessionState list =
            workspace.TabOrder
            |> List.choose (fun documentId -> workspace.Documents |> Map.tryFind documentId)

        match Functor.Workspace.WorkspaceModel.activeDocument workspace with
        | Some documentState ->
            { ActiveDocument = Some documentState.Document
              OpenDocuments = openDocuments |> List.map (fun value -> value.Document)
              Editing = documentState.Editing
              Syntax = documentState.Syntax
              Navigation = documentState.Navigation
              Diagnostics = documentState.Diagnostics
              Mode = documentState.Mode
              View = documentState.View }
        | None ->
            { CoreModel.empty with OpenDocuments = openDocuments |> List.map (fun value -> value.Document) }

    let requestEffects (effects: AppEffect list) =
        if not effects.IsEmpty then
            effectsRequested.Trigger(effects)

    let cancelPendingTokenization() =
        match pendingTokenization with
        | Some cancellation ->
            cancellation.Cancel()
            cancellation.Dispose()
            pendingTokenization <- None
        | None ->
            ()

    let currentTokenizationRequest() =
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

                            if startLine = 0 && endLine = System.Int32.MaxValue then
                                FullDocument
                            else
                                LineRange(startLine, endLine)

                let initialState =
                    IncrementalTokenizationState.initialState document.Id state.Model.Editing.Revision scope incrementalState

                let request: TokenizationRequest =
                    { DocumentId = document.Id
                      Revision = state.Model.Editing.Revision
                      Language = language
                      Scope = scope
                      Lines = state.Model.Editing.Buffer
                      InitialState = initialState }

                request))

    let requestCurrentTokenization() =
        match currentTokenizationRequest() with
        | Some request -> requestEffects [ AppEffect.tokenize request ]
        | None ->
            cancelPendingTokenization()

    let scheduleTokenization() =
        cancelPendingTokenization()

        match currentTokenizationRequest() with
        | Some _ ->
            let cancellation = new CancellationTokenSource()
            pendingTokenization <- Some cancellation

            Async.StartImmediate(async {
                try
                    do! Task.Delay(tokenizationDelay, cancellation.Token) |> Async.AwaitTask

                    if not cancellation.IsCancellationRequested then
                        requestCurrentTokenization()

                    if pendingTokenization = Some cancellation then
                        pendingTokenization <- None
                        cancellation.Dispose()
                with
                | :? TaskCanceledException ->
                    ()
                | :? OperationCanceledException ->
                    ()
            })
        | None ->
            ()

    let updateModel event =
        state <- syncActiveDocumentToWorkspace state
        state <- { state with Model = CoreLogic.update event state.Model }

        state <-
            match event with
            | NewDocument _
            | LoadDocument _ ->
                match state.Model.ActiveDocument with
                | Some document ->
                    { state with
                        Workspace = Functor.Workspace.WorkspaceLogic.update (Functor.Workspace.AddDocument document) state.Workspace }
                | None -> state
            | CloseDocument documentId ->
                { state with
                    Workspace = Functor.Workspace.WorkspaceLogic.update (Functor.Workspace.RemoveDocument documentId) state.Workspace }
            | SwitchDocument documentId ->
                { state with
                    Workspace = Functor.Workspace.WorkspaceLogic.update (Functor.Workspace.ActivateDocument documentId) state.Workspace }
            | _ -> state

        match event with
        | CloseDocument _
        | SwitchDocument _ ->
            state <- { state with Model = modelFromWorkspace state.Workspace }
        | _ ->
            state <- syncActiveDocumentToWorkspace state

        publishState()
        publishEditorStatus()

        match event with
        | LoadDocument _
        | SwitchDocument _
        | NewDocument _ ->
            cancelPendingTokenization()
            requestCurrentTokenization()
        | CloseDocument _ ->
            cancelPendingTokenization()
        | ApplyEditingEvent _ ->
            match state.Model.Editing.LastChange with
            | Some change ->
                 incrementalState <-
                    IncrementalTokenizationState.beginEdit
                        state.Model.ActiveDocument.Value.Id
                        (state.Model.Editing.Revision - 1L)
                        state.Model.Editing.Revision
                        change
                        incrementalState
            | None ->
                ()

            scheduleTokenization()
        | ApplySyntaxEvent(SetLanguage _) ->
            requestCurrentTokenization()
        | _ ->
            ()

    let isDirty() = state.Model.Editing.IsDirty

    let setPendingAction action message =
        state <-
            { state with
                Status =
                    { state.Status with
                        Message = Some message
                        PendingAction = Some action } }

        publishState()
        publishStatus()

    let clearPendingAction() =
        state <-
            { state with
                Status =
                    { state.Status with
                        Message = None
                        PendingAction = None } }

        publishState()
        publishStatus()

    let requestOpenFile() =
        if isDirty() then
            setPendingAction PendingAction.OpenFile "Unsaved changes must be confirmed before opening another file."
        else
            requestEffects [ AppEffect.openFile ]

    let requestOpenFolder() =
        requestEffects [ AppEffect.openFolder ]

    let replaceWorkspace path =
        cancelPendingTokenization()
        incrementalState <- IncrementalTokenizationState.reset
        state <-
            { state with
                Model = CoreModel.empty
                Workspace = Functor.Workspace.WorkspaceModel.create (Some path) }
        publishState()
        publishEditorStatus()

    let requestCloseDocument() =
        match state.Model.ActiveDocument with
        | Some document when isDirty() ->
            setPendingAction (PendingAction.CloseDocument document.Id) "Unsaved changes must be confirmed before closing the document."
        | Some document ->
            updateModel (CoreEvent.CloseDocument document.Id)
        | None ->
            ()

    let requestReopenClosedTab() =
        match state.Workspace.RecentlyClosedDocuments with
        | closed :: _ -> requestEffects [ AppEffect.readFile closed.Path ]
        | [] ->
            state <- AppSessionState.withMessage "No recently closed tab is available." state
            publishState()
            publishStatus()

    new() = EditorSession(CoreModel.empty)

    member _.State = state

    member _.Model = state.Model

    member _.Status = state.Status

    member _.EditorStatus = EditorStatus.fromState state

    member _.StateChanged = stateChanged.Publish

    member _.StatusChanged = statusChanged.Publish

    member _.EditorStatusChanged = editorStatusChanged.Publish

    member _.EffectsRequested = effectsRequested.Publish

    member _.Dispatch(event: CoreEvent) =
        updateModel event

    member _.DispatchCommand(command: AppCommand) =
        match command with
        | ExecuteCoreEvent evt ->
            updateModel evt
        | SetStatus message ->
            state <- AppSessionState.withMessage message state
            publishState()
            publishStatus()
        | ClearStatus ->
            state <- { state with Status = { state.Status with Message = None } }
            publishState()
            publishStatus()
        | ReportError error ->
            state <- AppSessionState.withError error state
            publishState()
            publishStatus()
        | NewDocumentRequested ->
            if isDirty() then
                setPendingAction PendingAction.NewDocument "Unsaved changes must be confirmed before creating a new document."
            else
                updateModel (CoreEvent.NewDocument "untitled")
        | OpenFileRequested ->
            requestOpenFile()
        | OpenFolderRequested ->
            requestOpenFolder()
        | SaveFileRequested ->
            let effect =
                match state.Model.ActiveDocument with
                | Some document when document.Metadata.Path.IsSome ->
                    let path = document.Metadata.Path.Value
                    let revision = state.Model.Editing.Revision
                    pendingSaves <- pendingSaves.Add(document.Id, (revision, Some path))
                    let contents = String.concat "\n" state.Model.Editing.Buffer
                    AppEffect.writeFileForDocument document.Id revision path contents
                | Some document ->
                    let contents = String.concat "\n" state.Model.Editing.Buffer
                    pendingSaves <- pendingSaves.Add(document.Id, (state.Model.Editing.Revision, None))
                    AppEffect.saveFile (Some document.Metadata.Name) contents
                | None ->
                    let contents = String.concat "\n" state.Model.Editing.Buffer
                    AppEffect.saveFile None contents

            requestEffects [ effect ]
        | SaveFileAsRequested ->
            let contents = String.concat "\n" state.Model.Editing.Buffer
            match state.Model.ActiveDocument with
            | Some document ->
                pendingSaves <- pendingSaves.Add(document.Id, (state.Model.Editing.Revision, None))
                let suggestedName = Some document.Metadata.Name
                requestEffects [ AppEffect.saveFileForDocument document.Id state.Model.Editing.Revision suggestedName contents ]
            | None ->
                requestEffects [ AppEffect.saveFile None contents ]
        | CloseDocumentRequested ->
            requestCloseDocument()
        | ReopenClosedTabRequested ->
            requestReopenClosedTab()
        | ReopenRecentDocument path ->
            requestEffects [ AppEffect.readFile path ]
        | ClearRecentDocumentsRequested ->
            state <-
                { state with
                    Workspace = Functor.Workspace.WorkspaceLogic.update Functor.Workspace.ClearRecentlyClosed state.Workspace }
            publishState()
        | OpenCommandPaletteRequested ->
            ()
        | OpenSettingsRequested ->
            ()
        | ConfirmDiscardChanges ->
            match state.Status.PendingAction with
            | Some action ->
                clearPendingAction()

                match action with
                | PendingAction.NewDocument -> updateModel (CoreEvent.NewDocument "untitled")
                | PendingAction.OpenFile -> requestEffects [ AppEffect.openFile ]
                | PendingAction.OpenWorkspace path -> replaceWorkspace path
                | PendingAction.CloseDocument id -> updateModel (CoreEvent.CloseDocument id)
            | None ->
                ()
        | CancelPendingOperation ->
            if state.Status.PendingAction.IsSome then
                clearPendingAction()
        | FileOpened(path, contents) ->
            clearPendingAction()
            match Functor.Workspace.WorkspaceModel.tryFindDocumentByPath path state.Workspace with
            | Some existing ->
                updateModel (SwitchDocument existing.Document.Id)
            | None ->
                updateModel (LoadDocument(path, contents))

            state <-
                { state with
                    Workspace = Functor.Workspace.WorkspaceLogic.update (Functor.Workspace.ConsumeRecentlyClosed path) state.Workspace }
            publishState()
        | FolderOpened path ->
            if isDirty() then
                setPendingAction (PendingAction.OpenWorkspace path) "Unsaved changes must be confirmed before opening another workspace."
            else
                replaceWorkspace path
        | FileSaved path ->
            clearPendingAction()
            let canonicalPath = DocumentModel.canonicalizePath path
            let pendingDocument =
                pendingSaves
                |> Map.toList
                |> List.tryFind (fun (_, (_, expectedPath)) -> expectedPath = Some canonicalPath)

            match pendingDocument, state.Model.ActiveDocument with
            | Some(documentId, (revision, _)), Some document when document.Id = documentId && state.Model.Editing.Revision = revision ->
                pendingSaves <- pendingSaves.Remove documentId
                updateModel (ApplyDocumentEvent(SetDocumentPath(Some canonicalPath)))
                updateModel (ApplyDocumentEvent MarkDocumentClean)
            | Some _, _ ->
                ()
            | None, Some document ->
                updateModel (ApplyDocumentEvent(SetDocumentPath(Some canonicalPath)))
                updateModel (ApplyDocumentEvent MarkDocumentClean)
            | None, None -> ()
        | FileSavedForDocument(documentId, revision, path) ->
            clearPendingAction()
            let canonicalPath = DocumentModel.canonicalizePath path

            match Functor.Workspace.WorkspaceModel.tryFindDocument documentId state.Workspace with
            | Some documentState when documentState.Editing.Revision = revision ->
                let savedDocument =
                    documentState.Document
                    |> DocumentLogic.update (SetDocumentPath(Some canonicalPath))
                    |> DocumentLogic.update MarkDocumentClean

                let savedEditing =
                    { documentState.Editing with
                        SavedBuffer = documentState.Editing.Buffer
                        IsDirty = false }

                let savedState =
                    { documentState with
                        Document = savedDocument
                        Editing = savedEditing }

                pendingSaves <- pendingSaves.Remove documentId
                state <-
                    { state with
                        Workspace =
                            Functor.Workspace.WorkspaceLogic.update
                                (Functor.Workspace.ReplaceDocumentState savedState)
                                state.Workspace }

                if state.Model.ActiveDocument |> Option.exists (fun document -> document.Id = documentId) then
                    state <- { state with Model = modelFromWorkspace state.Workspace }

                publishState()
                publishEditorStatus()
            | _ -> ()
        | FileOperationFailed message ->
            state <- AppSessionState.withError message state
            publishState()
            publishStatus()
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

                requestEffects
                    [ AppEffect.tokenize request ]
            | None ->
                ()
        | TokenizationCompleted result ->
            let matchesCurrentDocument =
                state.Model.ActiveDocument
                |> Option.exists (fun document ->
                    document.Id = result.DocumentId
                    && state.Model.Editing.Revision = result.Revision)

            let stable =
                match IncrementalTokenizationState.requestRange incrementalState, matchesCurrentDocument with
                | Some(_, endLine), true ->
                    IncrementalTokenizationState.isStable
                        result.DocumentId
                        result.Revision
                        state.Model.Editing.Buffer.Length
                        endLine
                        result.FinalState
                        (IncrementalTokenizationState.recordCompletion
                            result.DocumentId
                            result.Revision
                            result.Scope
                            result.Snapshots
                            result.FinalState
                            incrementalState)
                | _ -> false

            if matchesCurrentDocument then
                incrementalState <-
                    IncrementalTokenizationState.recordCompletion
                        result.DocumentId
                        result.Revision
                        result.Scope
                        result.Snapshots
                        result.FinalState
                        incrementalState

            let syntaxEvent =
                match result.Scope with
                | FullDocument ->
                    SetTokens(result.DocumentId, result.Revision, result.Tokens)
                | Line line ->
                    SetTokenRange(result.DocumentId, result.Revision, line, line, result.Tokens)
                | LineRange(startLine, endLine) ->
                    SetTokenRange(result.DocumentId, result.Revision, startLine, endLine, result.Tokens)

            if matchesCurrentDocument then
                updateModel (ApplySyntaxEvent syntaxEvent)

                match IncrementalTokenizationState.requestRange incrementalState, stable with
                | Some(_, endLine), true ->
                    incrementalState <- IncrementalTokenizationState.clearRange incrementalState
                    updateModel (ApplySyntaxEvent(MarkSyntaxCleanFrom(endLine + 1)))
                | Some(startLine, endLine), false ->
                    let nextEndLine = min (state.Model.Editing.Buffer.Length - 1) (endLine + 1)
                    incrementalState <- IncrementalTokenizationState.extend startLine endLine incrementalState
                    requestCurrentTokenization()
                | None, _ ->
                    ()

    member _.DispatchEffect(effect: AppEffect) =
        match effect with
        | NoEffect -> ()
        | NotifyStatus message ->
            state <- AppSessionState.withMessage message state
            publishState()
            publishStatus()
        | NotifyError error ->
            state <- AppSessionState.withError error state
            publishState()
            publishStatus()
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
        publishState()
        publishStatus()

    member _.ClearStatus() =
        state <- { state with Status = { state.Status with Message = None } }
        publishState()
        publishStatus()

    member _.ReportError(error: string) =
        state <- AppSessionState.withError error state
        publishState()
        publishStatus()
