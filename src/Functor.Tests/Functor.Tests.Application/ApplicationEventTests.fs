namespace Functor.Tests.Application

open System
open Functor.Application
open Xunit

type ApplicationEventTests() =
    [<Fact>]
    member _.``rejected commands publish sanitized failure metadata``() =
        let session = EditorSession()
        let events = ResizeArray<ApplicationEvent>()
        use subscription = session.ApplicationEvents.Subscribe(events.Add)

        session.DispatchCommandWithResult(AppCommand.openDocument "C:\\work\\image.png") |> ignore

        let event = events |> Seq.exactlyOne

        Assert.Equal("document.open", event.CommandName)
        Assert.Equal(None, event.DocumentId)
        Assert.Equal(session.State.Workspace.Id, event.WorkspaceId)
        Assert.True(event.CorrelationId <> Guid.Empty)
        Assert.True(event.Duration >= TimeSpan.Zero)

        match event.Outcome with
        | ApplicationEventOutcome.CommandRejected category -> Assert.Equal("unsupported-document-path", category)
        | outcome -> failwithf "Expected rejected outcome, got %A" outcome

    [<Fact>]
    member _.``applied commands publish current document identity``() =
        let session = EditorSession()
        let events = ResizeArray<ApplicationEvent>()
        use subscription = session.ApplicationEvents.Subscribe(events.Add)

        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\sample.fs" "secret document contents")

        let event = events |> Seq.exactlyOne
        let activeDocumentId = session.State.Model.ActiveDocument |> Option.map (fun document -> document.Id)

        Assert.Equal("file.opened", event.CommandName)
        Assert.Equal(activeDocumentId, event.DocumentId)
        Assert.Equal(ApplicationEventOutcome.CommandApplied, event.Outcome)