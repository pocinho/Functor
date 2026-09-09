namespace Functor.Application

open Functor.Domain.Core
open Functor.Domain.Document

type EditorSession(initialModel: CoreModel) =
    let mutable state = AppSessionState.empty initialModel
    let stateChanged = Event<AppSessionState>()
    let statusChanged = Event<SessionStatus>()
    let editorStatusChanged = Event<EditorStatus>()
    let effectsRequested = Event<AppEffect list>()

    let publishState() = stateChanged.Trigger(state)

    let publishEditorStatus() = editorStatusChanged.Trigger(EditorStatus.fromState state)

    let publishStatus() =
        statusChanged.Trigger(state.Status)
        publishEditorStatus()

    let updateModel event =
        state <- { state with Model = CoreLogic.update event state.Model }
        publishState()
        publishEditorStatus()

    let requestEffects (effects: AppEffect list) =
        if not effects.IsEmpty then
            effectsRequested.Trigger(effects)

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

    let requestCloseDocument() =
        match state.Model.ActiveDocument with
        | Some document when isDirty() ->
            setPendingAction (PendingAction.CloseDocument document.Id) "Unsaved changes must be confirmed before closing the document."
        | Some document ->
            updateModel (CoreEvent.CloseDocument document.Id)
        | None ->
            ()

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
        | SaveFileRequested ->
            let effect =
                match state.Model.ActiveDocument with
                | Some document when document.Metadata.Path.IsSome ->
                    let path = document.Metadata.Path.Value
                    let contents = String.concat "\n" state.Model.Editing.Buffer
                    AppEffect.writeFile path contents
                | Some document ->
                    let contents = String.concat "\n" state.Model.Editing.Buffer
                    AppEffect.saveFile (Some document.Metadata.Name) contents
                | None ->
                    let contents = String.concat "\n" state.Model.Editing.Buffer
                    AppEffect.saveFile None contents

            requestEffects [ effect ]
        | SaveFileAsRequested ->
            let contents = String.concat "\n" state.Model.Editing.Buffer
            let suggestedName = state.Model.ActiveDocument |> Option.map (fun document -> document.Metadata.Name)
            requestEffects [ AppEffect.saveFile suggestedName contents ]
        | CloseDocumentRequested ->
            requestCloseDocument()
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
                | PendingAction.CloseDocument id -> updateModel (CoreEvent.CloseDocument id)
            | None ->
                ()
        | CancelPendingOperation ->
            if state.Status.PendingAction.IsSome then
                clearPendingAction()
        | FileOpened(path, contents) ->
            clearPendingAction()
            updateModel (LoadDocument(path, contents))
        | FileSaved path ->
            clearPendingAction()
            match state.Model.ActiveDocument with
            | Some document ->
                if document.Metadata.Path <> Some path then
                    updateModel (ApplyDocumentEvent(SetDocumentPath(Some path)))

                updateModel (ApplyDocumentEvent MarkDocumentClean)
            | None -> ()
        | FileOperationFailed message ->
            state <- AppSessionState.withError message state
            publishState()
            publishStatus()

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
        | ReadFile _ -> ()
        | SaveFile _ -> ()
        | WriteFile _ -> ()

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
