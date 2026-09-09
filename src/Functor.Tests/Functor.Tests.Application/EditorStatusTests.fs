namespace Functor.Tests.Application

open Functor.Application
open Functor.Domain.Core
open Functor.Domain.Editing
open Xunit


type EditorStatusTests() =
    [<Fact>]
    member _.``status projection uses one-based cursor coordinates``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\sample.fs" "text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "!")))

        let status = session.EditorStatus

        Assert.Equal(1, status.Line)
        Assert.Equal(2, status.Column)
        Assert.Equal("sample.fs", status.FileName)
        Assert.Equal("F#", status.FileType)
        Assert.True(status.IsDirty)

    [<Fact>]
    member _.``status projection maps common file types``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\data.json" "{}")

        Assert.Equal("JSON", session.EditorStatus.FileType)

    [<Fact>]
    member _.``status projection exposes errors over messages``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.setStatus "Working")
        session.DispatchCommand(AppCommand.reportError "Failed")

        let status = session.EditorStatus

        Assert.Equal(Some "Working", status.Message)
        Assert.Equal(Some "Failed", status.Error)
