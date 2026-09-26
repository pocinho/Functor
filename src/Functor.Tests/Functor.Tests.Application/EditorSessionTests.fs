namespace Functor.Tests.Application

open System
open Functor.Application
open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Domain.Search
open Xunit


type EditorSessionTests() =
    [<Fact>]
    member _.``search remains empty when no document is active``() =
        let session = EditorSession()

        session.DispatchCommand(AppCommand.searchQueryChanged "needle")

        Assert.Equal(Some "needle", session.State.Model.Navigation.Search.Query)
        Assert.Empty(session.State.Model.Navigation.Search.Matches)

    [<Fact>]
    member _.``session exposes unified application state``() =
        let session = EditorSession()

        Assert.Equal(CoreModel.empty, session.State.Model)
        Assert.Equal(SessionStatus.empty, session.State.Status)
        Assert.Equal(session.State.Model, session.Model)
        Assert.Equal(session.State.Status, session.Status)

    [<Fact>]
    member _.``agent state remains isolated when switching tabs``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.newDocument)
        session.DispatchCommand(AppCommand.newDocument)
        let firstId = session.State.Workspace.TabOrder.Head
        let secondId = session.State.Workspace.TabOrder.Tail.Head

        session.DispatchCommand(AppCommand.setAgentOpen secondId true)
        session.Dispatch(CoreEvent.SwitchDocument firstId)

        Assert.False(session.State.Workspace.Documents[firstId].Auxiliary.Agent.IsOpen)
        Assert.True(session.State.Workspace.Documents[secondId].Auxiliary.Agent.IsOpen)

        session.Dispatch(CoreEvent.SwitchDocument secondId)
        Assert.True(session.State.Workspace.Documents[secondId].Auxiliary.Agent.IsOpen)

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
    member _.``search command uses the unsaved active buffer``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "disk")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " unsaved")))

        session.DispatchCommand(AppCommand.searchQueryChanged "unsaved")

        let search = session.State.Model.Navigation.Search
        Assert.Equal(Some "unsaved", search.Query)
        Assert.Single(search.Matches) |> ignore
        Assert.Equal(1, search.Matches[0].Column)
        Assert.Equal(session.State.Model.Editing.Revision, search.Revision.Value)

    [<Fact>]
    member _.``replace current search match uses the exact range``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "term 🚧 term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        session.DispatchCommand(AppCommand.replaceCurrentSearch "word")

        Assert.Equal<string list>([ "word 🚧 term" ], session.State.Model.Editing.Buffer)
        Assert.Single(session.State.Model.Navigation.Search.Matches) |> ignore
        Assert.Equal(8, session.State.Model.Navigation.Search.Matches[0].Column)

    [<Fact>]
    member _.``replace current search searches before replacing when needed``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "term")

        session.DispatchCommand(
            AppCommand.replaceCurrentSearchWithOptions
                { Query = "term"
                  CaseSensitive = false }
                "word"
        )

        Assert.Equal<string list>([ "word" ], session.State.Model.Editing.Buffer)

    [<Fact>]
    member _.``replace all search matches applies descending edits and refreshes search``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "term term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        session.DispatchCommand(AppCommand.replaceAllSearch "🚧")

        Assert.Equal<string list>([ "🚧 🚧" ], session.State.Model.Editing.Buffer)
        Assert.Empty(session.State.Model.Navigation.Search.Matches)
        Assert.Equal(session.State.Model.Editing.Revision, session.State.Model.Navigation.Search.Revision.Value)

    [<Fact>]
    member _.``replace all search matches can delete matches``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "term term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        session.DispatchCommand(AppCommand.replaceAllSearch "")

        Assert.Equal<string list>([ " " ], session.State.Model.Editing.Buffer)
        Assert.Empty(session.State.Model.Navigation.Search.Matches)

    [<Fact>]
    member _.``replace all with zero matches leaves the document unchanged``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "unchanged")
        session.DispatchCommand(AppCommand.searchQueryChanged "missing")
        let revision = session.State.Model.Editing.Revision

        session.DispatchCommand(AppCommand.replaceAllSearch "replacement")

        Assert.Equal<string list>([ "unchanged" ], session.State.Model.Editing.Buffer)
        Assert.Equal(revision, session.State.Model.Editing.Revision)

    [<Fact>]
    member _.``replace all keeps the document dirty and uses normal undo granularity``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "term term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        session.DispatchCommand(AppCommand.replaceAllSearch "word")

        Assert.Equal<string list>([ "word word" ], session.State.Model.Editing.Buffer)
        Assert.True(session.State.Model.Editing.IsDirty)
        Assert.Equal(2, session.State.Model.Editing.UndoStack.Length)

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent Undo))
        Assert.Equal<string list>([ "term word" ], session.State.Model.Editing.Buffer)

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent Redo))
        Assert.Equal<string list>([ "word word" ], session.State.Model.Editing.Buffer)

    [<Fact>]
    member _.``search defaults to every open in-memory tab without a workspace``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\first.fs" "needle in first")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\second.fs" "needle in second")
        session.DispatchCommand(AppCommand.searchQueryChanged "needle")

        let matches = session.State.Model.Navigation.Search.Matches

        Assert.Equal(2, matches.Length)
        Assert.Equal(Some "C:\work\first.fs", matches[0].Path)
        Assert.Equal(Some "C:\work\second.fs", matches[1].Path)

    [<Fact>]
    member _.``search remains global when switching documents``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\first.fs" "needle in first")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\second.fs" "needle in second")
        session.DispatchCommand(AppCommand.searchQueryChanged "needle")

        let matchesBeforeSwitch = session.State.Model.Navigation.Search.Matches
        session.Dispatch(CoreEvent.SwitchDocument session.State.Workspace.TabOrder.Head)

        let searchAfterSwitch = session.State.Model.Navigation.Search
        Assert.Equal(Some "needle", searchAfterSwitch.Query)
        Assert.Equal<SearchMatch list>(matchesBeforeSwitch, searchAfterSwitch.Matches)

    [<Fact>]
    member _.``workspace search completion is rejected after an open document edit``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.folderOpened "C:\work")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        let request =
            requestedEffects
            |> Seq.collect id
            |> Seq.choose (function
                | AppEffect.SearchWorkspace(request, _) -> Some request
                | _ -> None)
            |> Seq.last

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " changed")))

        let staleMatches =
            request.OpenDocuments
            |> List.collect (SearchEngine.findInDocument request.Options)

        session.DispatchCommand(
            AppCommand.workspaceSearchCompleted
                { RequestId = request.RequestId
                  WorkspaceId = request.WorkspaceId
                  Options = request.Options
                  Documents = request.OpenDocuments
                  Sources = Map.empty
                  Matches = staleMatches
                  Errors = []
                  OpenDocumentRevisions = request.OpenDocumentRevisions }
        )

        Assert.Empty(session.State.Model.Navigation.Search.Matches)
        Assert.True(session.State.Model.Navigation.Search.IsDirty)

    [<Fact>]
    member _.``editing cancels the in-flight workspace search before the next query``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.folderOpened "C:\work")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        let firstRequest, firstCancellation =
            requestedEffects
            |> Seq.collect id
            |> Seq.choose (function
                | AppEffect.SearchWorkspace(request, cancellationToken) -> Some(request, cancellationToken)
                | _ -> None)
            |> Seq.last

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " changed")))
        Assert.True(firstCancellation.IsCancellationRequested)

        session.DispatchCommand(AppCommand.searchQueryChanged "changed")

        let secondRequest, secondCancellation =
            requestedEffects
            |> Seq.collect id
            |> Seq.choose (function
                | AppEffect.SearchWorkspace(request, cancellationToken) -> Some(request, cancellationToken)
                | _ -> None)
            |> Seq.last

        Assert.NotEqual(firstRequest.RequestId, secondRequest.RequestId)
        Assert.False(secondCancellation.IsCancellationRequested)

    [<Fact>]
    member _.``clearing search cancels the in-flight workspace search``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.folderOpened "C:\work")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        let cancellationTokenSource =
            requestedEffects
            |> Seq.collect id
            |> Seq.choose (function
                | AppEffect.SearchWorkspace(_, cancellationToken) -> Some cancellationToken
                | _ -> None)
            |> Seq.last

        session.DispatchCommand(AppCommand.clearSearch)

        Assert.True(cancellationTokenSource.IsCancellationRequested)
        Assert.Equal(None, session.State.Model.Navigation.Search.Query)
        Assert.Empty(session.State.Model.Navigation.Search.Matches)

    [<Fact>]
    member _.``activating a match after an emoji preserves its UTF-16 range``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\emoji.fs" "😀 term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        let result = session.State.Model.Navigation.Search.Matches |> List.exactlyOne
        session.DispatchCommand(AppCommand.activateSearchResult result)

        Assert.Equal(
            Some
                { Start = { Line = 0; Column = 3 }
                  End = { Line = 0; Column = 7 } },
            session.State.Model.Editing.Selection
        )

    [<Fact>]
    member _.``workspace search completion is rejected after workspace replacement``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.folderOpened "C:\work\old")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\old\file.fs" "term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        let request =
            requestedEffects
            |> Seq.collect id
            |> Seq.choose (function
                | AppEffect.SearchWorkspace(request, _) -> Some request
                | _ -> None)
            |> Seq.last

        session.DispatchCommand(AppCommand.folderOpened "C:\work\new")

        let staleMatch =
            request.OpenDocuments
            |> List.collect (SearchEngine.findInDocument request.Options)

        session.DispatchCommand(
            AppCommand.workspaceSearchCompleted
                { RequestId = request.RequestId
                  WorkspaceId = request.WorkspaceId
                  Options = request.Options
                  Documents = request.OpenDocuments
                  Sources = Map.empty
                  Matches = staleMatch
                  Errors = []
                  OpenDocumentRevisions = request.OpenDocumentRevisions }
        )

        Assert.Empty(session.State.Model.Navigation.Search.Matches)
        Assert.Null(session.State.Model.Navigation.Search.Query)

    [<Fact>]
    member _.``workspace search completion is rejected after refresh``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.folderOpened "C:\work")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        let firstRequest =
            requestedEffects
            |> Seq.collect id
            |> Seq.choose (function
                | AppEffect.SearchWorkspace(request, _) -> Some request
                | _ -> None)
            |> Seq.last

        let matchesBeforeRefresh = session.State.Model.Navigation.Search.Matches
        session.DispatchCommand(AppCommand.refreshSearch)

        let secondRequest =
            requestedEffects
            |> Seq.collect id
            |> Seq.choose (function
                | AppEffect.SearchWorkspace(request, _) -> Some request
                | _ -> None)
            |> Seq.last

        Assert.NotEqual(firstRequest.RequestId, secondRequest.RequestId)

        let staleMatches =
            firstRequest.OpenDocuments
            |> List.collect (SearchEngine.findInDocument firstRequest.Options)

        session.DispatchCommand(
            AppCommand.workspaceSearchCompleted
                { RequestId = firstRequest.RequestId
                  WorkspaceId = firstRequest.WorkspaceId
                  Options = firstRequest.Options
                  Documents = firstRequest.OpenDocuments
                  Sources = Map.empty
                  Matches = staleMatches
                  Errors = []
                  OpenDocumentRevisions = firstRequest.OpenDocumentRevisions }
        )

        Assert.Equal<SearchMatch list>(matchesBeforeRefresh, session.State.Model.Navigation.Search.Matches)

    [<Fact>]
    member _.``workspace replace-all requests a safe effect for unopened matches``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.folderOpened "C:\work")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\open.fs" "open")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        let request =
            requestedEffects
            |> Seq.collect id
            |> Seq.choose (function
                | AppEffect.SearchWorkspace(request, _) -> Some request
                | _ -> None)
            |> Seq.last

        let unopenedId = Guid.NewGuid()

        let unopenedDocument: SearchDocument =
            { Id = unopenedId
              Path = Some "C:\work\unopened.fs"
              Name = "unopened.fs"
              Lines = [ "term" ] }

        let matchValue: SearchMatch =
            { DocumentId = unopenedId
              Path = unopenedDocument.Path
              Name = unopenedDocument.Name
              Line = 0
              Column = 0
              Length = 4
              Range =
                { Start = { Line = 0; Column = 0 }
                  End = { Line = 0; Column = 4 } }
              Preview = "term" }

        session.DispatchCommand(
            AppCommand.workspaceSearchCompleted
                { RequestId = request.RequestId
                  WorkspaceId = request.WorkspaceId
                  Options = request.Options
                  Documents = request.OpenDocuments @ [ unopenedDocument ]
                  Sources = Map.ofList [ "C:\work\unopened.fs", "term" ]
                  Matches = [ matchValue ]
                  Errors = []
                  OpenDocumentRevisions = request.OpenDocumentRevisions }
        )

        requestedEffects.Clear()
        session.DispatchCommand(AppCommand.replaceAllSearch "word")

        Assert.Contains(
            requestedEffects |> Seq.collect id,
            function
            | AppEffect.ReplaceWorkspace(replacement, _) -> replacement.Replacements["C:\work\unopened.fs"] = "word"
            | _ -> false
        )

    [<Fact>]
    member _.``workspace replace-all reports stale open documents``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.folderOpened "C:\work")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\open.fs" "term")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        let request =
            requestedEffects
            |> Seq.collect id
            |> Seq.choose (function
                | AppEffect.SearchWorkspace(request, _) -> Some request
                | _ -> None)
            |> Seq.last

        let staleMatches =
            request.OpenDocuments
            |> List.collect (SearchEngine.findInDocument request.Options)

        session.DispatchCommand(
            AppCommand.workspaceSearchCompleted
                { RequestId = request.RequestId
                  WorkspaceId = request.WorkspaceId
                  Options = request.Options
                  Documents = request.OpenDocuments
                  Sources = Map.empty
                  Matches = staleMatches
                  Errors = []
                  OpenDocumentRevisions = request.OpenDocumentRevisions }
        )

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " changed")))
        session.DispatchCommand(AppCommand.replaceAllSearch "word")

        Assert.Equal<string list>([ " changedterm" ], session.State.Model.Editing.Buffer)
        Assert.Equal(Some "Skipped 1 stale file(s).", session.State.Status.Message)

    [<Fact>]
    member _.``next and previous search results select and reveal the active match``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\file.fs" "term here\nterm again")
        session.DispatchCommand(AppCommand.searchQueryChanged "term")

        session.DispatchCommand(AppCommand.nextSearchResult)

        Assert.Equal(Some 1, session.State.Model.Navigation.Search.Index)

        Assert.Equal(
            Some
                { Start = { Line = 1; Column = 0 }
                  End = { Line = 1; Column = 4 } },
            session.State.Model.Editing.Selection
        )

        Assert.Equal(1, session.State.Model.View.VerticalOffset)

        session.DispatchCommand(AppCommand.previousSearchResult)

        Assert.Equal(Some 0, session.State.Model.Navigation.Search.Index)
        Assert.Equal(0, session.State.Model.Editing.Selection.Value.Start.Line)
        Assert.Equal(0, session.State.Model.View.VerticalOffset)

    [<Fact>]
    member _.``activating an open workspace result selects its exact range``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\result.fs" "first\nneedle here")
        session.DispatchCommand(AppCommand.searchQueryChanged "needle")

        let result = session.State.Model.Navigation.Search.Matches[0]
        session.DispatchCommand(AppCommand.activateSearchResult result)

        Assert.Equal(Some result.DocumentId, session.State.Workspace.ActiveDocumentId)
        Assert.Equal(Some 0, session.State.Model.Navigation.Search.Index)
        Assert.Equal(Some result.Range, session.State.Model.Editing.Selection)
        Assert.Equal(result.Line, session.State.Model.View.VerticalOffset)

    [<Fact>]
    member _.``activating an unopened workspace result opens and selects its range``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.folderOpened "C:\work")

        let result: SearchMatch =
            { DocumentId = Guid.NewGuid()
              Path = Some "C:\work\unopened.fs"
              Name = "unopened.fs"
              Line = 1
              Column = 2
              Length = 3
              Range =
                { Start = { Line = 1; Column = 2 }
                  End = { Line = 1; Column = 5 } }
              Preview = "needle" }

        session.DispatchCommand(AppCommand.activateSearchResult result)

        Assert.Contains(
            requestedEffects |> Seq.collect id,
            fun effect -> effect = AppEffect.ReadFile "C:\work\unopened.fs"
        )

        session.DispatchCommand(AppCommand.fileOpened "C:\work\unopened.fs" "x\n  needle")

        Assert.Equal(Some result.Range, session.State.Model.Editing.Selection)
        Assert.Equal(1, session.State.Model.View.VerticalOffset)

    [<Fact>]
    member _.``activating an already open workspace result does not reread the file``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\work\open.fs" "needle")
        let documentId = session.State.Model.ActiveDocument.Value.Id
        requestedEffects.Clear()

        let result: SearchMatch =
            { DocumentId = documentId
              Path = Some "C:\work\open.fs"
              Name = "open.fs"
              Line = 0
              Column = 0
              Length = 6
              Range =
                { Start = { Line = 0; Column = 0 }
                  End = { Line = 0; Column = 6 } }
              Preview = "needle" }

        session.DispatchCommand(AppCommand.activateSearchResult result)

        Assert.DoesNotContain(
            requestedEffects |> Seq.collect id,
            fun effect ->
                match effect with
                | ReadFile _ -> true
                | _ -> false
        )

        Assert.Equal(Some documentId, session.State.Workspace.ActiveDocumentId)

    [<Fact>]
    member _.``activating a result preserves an already open dirty document``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\dirty.fs" "original")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " unsaved")))
        let documentId = session.State.Model.ActiveDocument.Value.Id

        let result: SearchMatch =
            { DocumentId = documentId
              Path = Some "C:\work\dirty.fs"
              Name = "dirty.fs"
              Line = 0
              Column = 0
              Length = 8
              Range =
                { Start = { Line = 0; Column = 0 }
                  End = { Line = 0; Column = 8 } }
              Preview = "original" }

        session.DispatchCommand(AppCommand.activateSearchResult result)

        Assert.Equal(Some documentId, session.State.Workspace.ActiveDocumentId)
        Assert.True(session.State.Model.Editing.IsDirty)
        Assert.Equal(" unsavedoriginal", String.concat "\n" session.State.Model.Editing.Buffer)

    [<Fact>]
    member _.``activating a result after switching tabs returns to the matching document``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\work\first.fs" "needle")
        session.DispatchCommand(AppCommand.searchQueryChanged "needle")
        let result = session.State.Model.Navigation.Search.Matches[0]
        session.DispatchCommand(AppCommand.fileOpened "C:\work\second.fs" "other")

        session.DispatchCommand(AppCommand.activateSearchResult result)

        Assert.Equal(Some result.DocumentId, session.State.Workspace.ActiveDocumentId)
        Assert.Equal(Some result.Range, session.State.Model.Editing.Selection)

    [<Fact>]
    member _.``opening a new workspace clears existing search results``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.folderOpened "C:\work\first")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\first\file.fs" "needle")
        session.DispatchCommand(AppCommand.searchQueryChanged "needle")
        Assert.NotEmpty(session.State.Model.Navigation.Search.Matches)

        session.DispatchCommand(AppCommand.folderOpened "C:\work\second")

        Assert.Empty(session.State.Model.Navigation.Search.Matches)
        Assert.Empty(session.State.Model.Navigation.Search.Results)
        Assert.Equal(None, session.State.Model.Navigation.Search.Query)

    [<Fact>]
    member _.``search options persist before a workspace document is opened``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.folderOpened "C:\work")
        session.DispatchCommand(AppCommand.searchQueryChanged "needle")

        session.DispatchCommand(
            AppCommand.searchOptionsChanged
                { Query = "needle"
                  CaseSensitive = true }
        )

        let search = session.State.Model.Navigation.Search
        Assert.Equal(Some "needle", search.Query)
        Assert.Equal("needle", search.Options.Query)
        Assert.True(search.Options.CaseSensitive)

    [<Fact>]
    member _.``workspace search request includes open in-memory documents``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.folderOpened "C:\work")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\open.fs" "unsaved")
        session.DispatchCommand(AppCommand.fileOpened "C:\work\second.fs" "second unsaved")
        session.DispatchCommand(AppCommand.searchQueryChanged "unsaved")
        requestedEffects.Clear()
        session.DispatchCommand(AppCommand.workspaceSearch)

        let searchEffects =
            requestedEffects
            |> Seq.collect id
            |> Seq.choose (function
                | SearchWorkspace(request, _) -> Some request
                | _ -> None)
            |> Seq.toList

        match searchEffects with
        | [ request ] ->
            Assert.Equal(session.State.Workspace.Id, request.WorkspaceId)
            Assert.Equal("C:\work", request.RootPath)
            Assert.Equal(2, request.OpenDocuments.Length)
            Assert.True([ "unsaved" ] = request.OpenDocuments[0].Lines)
            Assert.True([ "second unsaved" ] = request.OpenDocuments[1].Lines)
            Assert.True(request.OpenDocumentPaths.Contains "C:\work\open.fs")
            Assert.True(request.OpenDocumentPaths.Contains "C:\work\second.fs")
        | _ -> Assert.True(false, "Expected one workspace search effect.")

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
    member _.``opening a known closed path requests a read effect``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.openDocument "C:\\work\\file.fs")

        Assert.True([ AppEffect.readFile "C:\\work\\file.fs" ] = requestedEffects[0])

    [<Fact>]
    member _.``opening an unsupported file reports status without reading it``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.openDocument "C:\\work\\image.png")

        Assert.Empty(requestedEffects)

        Assert.Equal(
            Some "Unsupported or binary file cannot be opened: C:\\work\\image.png",
            session.State.Status.Message
        )

    [<Fact>]
    member _.``opening an unknown workspace text extension requests a read``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.openDocument "C:\work\script.py")

        Assert.Equal<AppEffect list>([ AppEffect.readFile "C:\work\script.py" ], requestedEffects[0])

    [<Fact>]
    member _.``opening an already open path activates the existing document``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\first.fs" "first")
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\second.fs" "second")
        let firstId = session.State.Workspace.TabOrder.Head

        session.DispatchCommand(AppCommand.openDocument "C:\\work\\first.fs")

        Assert.Equal(Some firstId, session.State.Workspace.ActiveDocumentId)
        Assert.Equal(2, session.State.Workspace.TabOrder.Length)

    [<Fact>]
    member _.``opening a path while dirty waits for discard confirmation``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\first.fs" "first")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))
        requestedEffects.Clear()

        session.DispatchCommand(AppCommand.openDocument "C:\\work\\second.fs")

        Assert.Equal(Some(PendingAction.OpenDocument "C:\\work\\second.fs"), session.Status.PendingAction)
        Assert.Empty(requestedEffects)

        session.DispatchCommand(AppCommand.confirmDiscardChanges)

        Assert.True([ AppEffect.readFile "C:\\work\\second.fs" ] = requestedEffects[0])

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

        Assert.True(
            [ AppEffect.writeFileForDocument
                  document.Id
                  session.Model.Editing.Revision
                  "C:\\work\\file.fs"
                  " updatedtext" ] =
                requestedEffects[0]
        )

    [<Fact>]
    member _.``save command does nothing when there is no active document``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.saveFile)

        Assert.Empty(requestedEffects)

    [<Fact>]
    member _.``save as command does nothing when there is no active document``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.saveFileAs)

        Assert.Empty(requestedEffects)

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
    member _.``completed earlier save preserves its saved snapshot while newer edits remain dirty``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "1")))
        let firstRevision = session.Model.Editing.Revision
        session.DispatchCommand(AppCommand.saveFile)

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "2")))
        session.DispatchCommand(AppCommand.saveFile)
        let documentId = session.Model.ActiveDocument.Value.Id

        session.DispatchCommand(AppCommand.fileSavedForDocument documentId firstRevision "C:\\work\\file.fs")

        Assert.True([ "1" + "text" ] = session.Model.Editing.SavedBuffer)
        Assert.True(session.Model.Editing.IsDirty)

    [<Fact>]
    member _.``file-saved command marks the active document clean``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))
        session.DispatchCommand(AppCommand.saveFile)

        session.DispatchCommand(AppCommand.fileSaved "C:\\work\\file.fs")

        Assert.False(session.State.Model.ActiveDocument.Value.Metadata.IsDirty)
        Assert.False(session.State.Model.Editing.IsDirty)

    [<Fact>]
    member _.``file-saved command assigns a path when saving an untitled document``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.newDocument)
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))
        session.DispatchCommand(AppCommand.saveFile)

        session.DispatchCommand(AppCommand.fileSaved "C:\\work\\saved.fs")

        Assert.Equal(Some "C:\\work\\saved.fs", session.State.Model.ActiveDocument.Value.Metadata.Path)
        Assert.False(session.State.Model.ActiveDocument.Value.Metadata.IsDirty)
        Assert.False(session.State.Model.Editing.IsDirty)

    [<Fact>]
    member _.``unmatched file-saved completion does not clear dirty edits``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        session.DispatchCommand(AppCommand.fileSaved "C:\\work\\other.fs")

        Assert.True(session.State.Model.Editing.IsDirty)

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
    member _.``creating a new document preserves the dirty document in another tab``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        session.DispatchCommand(AppCommand.newDocument)

        Assert.Equal(None, session.State.Model.ActiveDocument.Value.Metadata.Path)
        Assert.False(session.State.Model.Editing.IsDirty)
        Assert.Equal(2, session.State.Workspace.Documents.Count)

        Assert.True(
            session.State.Workspace.Documents.Values
            |> Seq.exists (fun document -> document.Editing.IsDirty)
        )

        Assert.Equal(None, session.State.Status.PendingAction)

    [<Fact>]
    member _.``confirming a pending close removes the dirty document``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        let document = session.Model.ActiveDocument.Value
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        session.DispatchCommand(AppCommand.closeDocument)

        Assert.Equal(Some(PendingAction.CloseDocument document.Id), session.State.Status.PendingAction)
        session.DispatchCommand(AppCommand.confirmDiscardChanges)

        Assert.True(session.State.Model.ActiveDocument.IsNone)

        Assert.True(
            session.State.Workspace.RecentlyClosedDocuments
            |> List.exists (fun closed -> closed.Path = "C:\\work\\file.fs")
        )

        Assert.Equal(None, session.State.Status.PendingAction)

    [<Fact>]
    member _.``save completion from a closed document does not affect a reopened document``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        let closedDocument = session.Model.ActiveDocument.Value
        session.DispatchCommand(AppCommand.saveFile)
        session.DispatchCommand(AppCommand.closeDocument)

        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "new text")
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        session.DispatchCommand(AppCommand.fileSavedForDocument closedDocument.Id 0L "C:\\work\\file.fs")

        Assert.True(session.State.Model.Editing.IsDirty)
        Assert.Equal(Some "C:\\work\\file.fs", session.State.Model.ActiveDocument.Value.Metadata.Path)

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

    [<Fact>]
    member _.``unrequested document save completion does not clear dirty edits``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        let document = session.Model.ActiveDocument.Value
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))
        let revision = session.Model.Editing.Revision

        session.DispatchCommand(AppCommand.fileSavedForDocument document.Id revision "C:\\work\\file.fs")

        Assert.True(session.State.Model.Editing.IsDirty)
