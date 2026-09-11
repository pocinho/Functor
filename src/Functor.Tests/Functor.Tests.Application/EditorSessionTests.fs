namespace Functor.Tests.Application

open Functor.Application
open Functor.Domain.Core
open Functor.Domain.Editing
open Xunit


type EditorSessionTests() =
    [<Fact>]
    member _.``session exposes unified application state``() =
        let session = EditorSession()

        Assert.Equal(CoreModel.empty, session.State.Model)
        Assert.Equal(SessionStatus.empty, session.State.Status)
        Assert.Equal(session.State.Model, session.Model)
        Assert.Equal(session.State.Status, session.Status)

    [<Fact>]
    member _.``core command updates state and publishes the new state``() =
        let session = EditorSession()
        let publishedStates = ResizeArray<AppSessionState>()
        session.StateChanged.Add(fun state -> publishedStates.Add(state) |> ignore)

        session.DispatchCommand(AppCommand.toCoreEvent (ResizeViewport(800, 600)))

        Assert.Equal({ Width = 800; Height = 600 }, session.State.Model.View.Viewport)
        Assert.Single(publishedStates) |> ignore
        Assert.Equal(session.State, publishedStates[0])

    [<Fact>]
    member _.``status command updates state and publishes status``() =
        let session = EditorSession()
        let publishedStatuses = ResizeArray<SessionStatus>()
        session.StatusChanged.Add(fun status -> publishedStatuses.Add(status) |> ignore)

        session.DispatchCommand(AppCommand.setStatus "Ready")

        Assert.Equal(Some "Ready", session.State.Status.Message)
        Assert.Equal(None, session.State.Status.Error)
        Assert.Single(publishedStatuses) |> ignore
        Assert.Equal(session.State.Status, publishedStatuses[0])

    [<Fact>]
    member _.``error effect updates the session status``() =
        let session = EditorSession()

        session.DispatchEffect(AppEffect.notifyError "Unable to save")

        Assert.Equal(Some "Unable to save", session.State.Status.Error)
        Assert.Equal(None, session.State.Status.Message)

    [<Fact>]
    member _.``clearing status preserves an existing error``() =
        let session = EditorSession()

        session.DispatchCommand(AppCommand.reportError "Failed")
        session.DispatchCommand(AppCommand.setStatus "Retrying")
        session.DispatchCommand(AppCommand.clearStatus)

        Assert.Equal(None, session.State.Status.Message)
        Assert.Equal(Some "Failed", session.State.Status.Error)

    [<Fact>]
    member _.``open command requests an open-file effect``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.openFile)

        Assert.Single(requestedEffects) |> ignore
        Assert.True([ AppEffect.openFile ] = requestedEffects[0])

    [<Fact>]
    member _.``file-opened command loads clean document contents``() =
        let session = EditorSession()

        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "one\ntwo")

        Assert.Equal(Some "C:\\work\\file.fs", session.State.Model.ActiveDocument.Value.Metadata.Path)
        Assert.True([ "one"; "two" ] = session.State.Model.Editing.Buffer)
        Assert.False(session.State.Model.ActiveDocument.Value.Metadata.IsDirty)
        Assert.False(session.State.Model.Editing.IsDirty)

    [<Fact>]
    member _.``reopen closed tab requests the newest closed file``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        session.DispatchCommand(AppCommand.closeDocument)
        requestedEffects.Clear()

        session.DispatchCommand(AppCommand.reopenClosedTab)

        Assert.True([ AppEffect.readFile "C:\\work\\file.fs" ] = requestedEffects[0])

    [<Fact>]
    member _.``reopening a closed tab consumes its history entry``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "old")
        session.DispatchCommand(AppCommand.closeDocument)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "restored")

        Assert.Empty(session.State.Workspace.RecentlyClosedDocuments)
        Assert.True([ "restored" ] = session.State.Model.Editing.Buffer)

    [<Fact>]
    member _.``clear recent documents removes closed file history``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        session.DispatchCommand(AppCommand.closeDocument)

        session.DispatchCommand(AppCommand.clearRecentDocuments)

        Assert.Empty(session.State.Workspace.RecentlyClosedDocuments)

    [<Fact>]
    member _.``save command writes active document contents``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        requestedEffects.Clear()
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        session.DispatchCommand(AppCommand.saveFile)

        Assert.Single(requestedEffects) |> ignore
        let document = session.Model.ActiveDocument.Value
        Assert.True([ AppEffect.writeFileForDocument document.Id session.Model.Editing.Revision "C:\\work\\file.fs" " updatedtext" ] = requestedEffects[0])

    [<Fact>]
    member _.``stale save completion does not clear newer edits``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " first")))
        session.DispatchCommand(AppCommand.saveFile)
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " second")))

        session.DispatchCommand(AppCommand.fileSaved "C:\\work\\file.fs")

        Assert.True(session.State.Model.Editing.IsDirty)

    [<Fact>]
    member _.``file-saved command marks the active document clean``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        session.DispatchCommand(AppCommand.fileSaved "C:\\work\\file.fs")

        Assert.False(session.State.Model.ActiveDocument.Value.Metadata.IsDirty)
        Assert.False(session.State.Model.Editing.IsDirty)

    [<Fact>]
    member _.``file-saved command assigns a path when saving an untitled document``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\untitled.fs" "text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        session.DispatchCommand(AppCommand.fileSaved "C:\\work\\saved.fs")

        Assert.Equal(Some "C:\\work\\saved.fs", session.State.Model.ActiveDocument.Value.Metadata.Path)
        Assert.False(session.State.Model.ActiveDocument.Value.Metadata.IsDirty)
        Assert.False(session.State.Model.Editing.IsDirty)

    [<Fact>]
    member _.``editing becomes clean again when undo restores the saved snapshot``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "!")))

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent Undo))

        Assert.False(session.State.Model.Editing.IsDirty)
        Assert.True([ "text" ] = session.State.Model.Editing.Buffer)

    [<Fact>]
    member _.``opening a file with dirty edits creates a pending confirmation``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        requestedEffects.Clear()
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        session.DispatchCommand(AppCommand.openFile)

        Assert.Equal(Some PendingAction.OpenFile, session.State.Status.PendingAction)
        Assert.Empty(requestedEffects)

    [<Fact>]
    member _.``confirming a pending open requests the file dialog``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        requestedEffects.Clear()
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))
        session.DispatchCommand(AppCommand.openFile)

        session.DispatchCommand(AppCommand.confirmDiscardChanges)

        Assert.Equal(None, session.State.Status.PendingAction)
        Assert.Single(requestedEffects) |> ignore
        Assert.True([ AppEffect.openFile ] = requestedEffects[0])

    [<Fact>]
    member _.``opening a folder replaces a clean session workspace``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")

        session.DispatchCommand(AppCommand.folderOpened "C:\\work\\folder")

        Assert.Equal(Some "C:\\work\\folder", session.State.Workspace.RootPath)
        Assert.True(session.State.Workspace.Documents.IsEmpty)
        Assert.True(session.State.Model.ActiveDocument.IsNone)

    [<Fact>]
    member _.``opening a folder with dirty edits requires confirmation``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        session.DispatchCommand(AppCommand.folderOpened "C:\\work\\folder")

        Assert.Equal(Some(PendingAction.OpenWorkspace "C:\\work\\folder"), session.State.Status.PendingAction)
        session.DispatchCommand(AppCommand.confirmDiscardChanges)
        Assert.Equal(Some "C:\\work\\folder", session.State.Workspace.RootPath)
        Assert.True(session.State.Model.ActiveDocument.IsNone)

    [<Fact>]
    member _.``canceling a pending new document leaves the current document unchanged``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        session.DispatchCommand(AppCommand.newDocument)
        session.DispatchCommand(AppCommand.cancelPendingOperation)

        Assert.Equal(Some "C:\\work\\file.fs", session.State.Model.ActiveDocument.Value.Metadata.Path)
        Assert.True(session.State.Model.Editing.IsDirty)
        Assert.Equal(None, session.State.Status.PendingAction)

    [<Fact>]
    member _.``switching tabs restores each document editing state``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\first.fs" "first")
        let first = session.Model.ActiveDocument.Value
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\second.fs" "second")
        let second = session.Model.ActiveDocument.Value
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " changed")))

        session.DispatchCommand(AppCommand.toCoreEvent (SwitchDocument first.Id))
        Assert.True([ " updatedfirst" ] = session.Model.Editing.Buffer)
        Assert.True(session.Model.Editing.IsDirty)

        session.DispatchCommand(AppCommand.toCoreEvent (SwitchDocument second.Id))
        Assert.True([ " changedsecond" ] = session.Model.Editing.Buffer)
        Assert.True(session.Model.Editing.IsDirty)

    [<Fact>]
    member _.``document save completion updates an inactive tab``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\first.fs" "first")
        let first = session.Model.ActiveDocument.Value
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))
        session.DispatchCommand(AppCommand.saveFile)
        let revision = session.Model.Editing.Revision
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\second.fs" "second")

        session.DispatchCommand(AppCommand.fileSavedForDocument first.Id revision "C:\\work\\first.fs")

        let savedFirst = session.State.Workspace.Documents[first.Id]
        Assert.False(savedFirst.Editing.IsDirty)
        Assert.False(savedFirst.Document.Metadata.IsDirty)
