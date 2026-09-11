namespace Functor.Workspace

open Functor.Domain.Document

module WorkspaceLogic =

    let private rememberClosedDocument document (workspace: WorkspaceModel) =
        match document.Metadata.Path with
        | None -> workspace
        | Some path ->
            let closed =
                { Path = DocumentModel.canonicalizePath path
                  Name = document.Metadata.Name }

            { workspace with
                RecentlyClosedDocuments =
                    closed
                    :: (workspace.RecentlyClosedDocuments
                        |> List.filter (fun previous ->
                            not (System.String.Equals(previous.Path, closed.Path, System.StringComparison.OrdinalIgnoreCase)))
                        |> List.truncate 9) }

    let private updateDocument documentId update workspace =
        match WorkspaceModel.tryFindDocument documentId workspace with
        | Some documentState ->
            { workspace with
                Documents = workspace.Documents |> Map.add documentId (update documentState) }
        | None -> workspace

    let private updateDocumentAtRevision documentId revision update workspace =
        updateDocument documentId (fun documentState ->
            if documentState.Editing.Revision = revision then
                update documentState
            else
                documentState) workspace

    let private activateFallback removedId (workspace: WorkspaceModel) =
        if workspace.ActiveDocumentId <> Some removedId then
            workspace
        else
            { workspace with
                ActiveDocumentId =
                    workspace.TabOrder |> List.tryHead }

    let update event workspace =
        match event with
        | AddDocument document -> WorkspaceModel.addDocument document workspace
        | ActivateDocument documentId when WorkspaceModel.containsDocument documentId workspace ->
            { workspace with ActiveDocumentId = Some documentId }
        | ActivateDocument _ -> workspace
        | RemoveDocument documentId when WorkspaceModel.containsDocument documentId workspace ->
            let closedDocument = WorkspaceModel.tryFindDocument documentId workspace |> Option.map (fun state -> state.Document)
            let updated =
                { workspace with
                    Documents = workspace.Documents |> Map.remove documentId
                    TabOrder = workspace.TabOrder |> List.filter ((<>) documentId) }
                |> activateFallback documentId

            closedDocument |> Option.map (fun document -> rememberClosedDocument document updated) |> Option.defaultValue updated
        | RemoveDocument _ -> workspace
        | SetRootPath rootPath ->
            { workspace with
                RootPath = rootPath |> Option.map WorkspaceModel.canonicalizeRootPath }
        | ReplaceWorkspace rootPath ->
            WorkspaceModel.create rootPath
        | ClearDocuments ->
            { workspace with
                ActiveDocumentId = None
                Documents = Map.empty
                TabOrder = [] }
        | ClearRecentlyClosed ->
            { workspace with RecentlyClosedDocuments = [] }
        | ConsumeRecentlyClosed path ->
            WorkspaceModel.consumeRecentlyClosed path workspace
        | ReplaceDocumentState documentState ->
            updateDocument documentState.Document.Id (fun _ -> documentState) workspace
        | ApplyEditingEvent(documentId, event) ->
            updateDocument documentId (fun documentState ->
                let editing = Functor.Domain.Editing.EditingLogic.update event documentState.Editing
                let document =
                    if editing.IsDirty then
                        DocumentModel.markDirty documentState.Document
                    else
                        documentState.Document

                { documentState with
                    Document = document
                    Editing = editing
                    Syntax =
                        if editing.Revision <> documentState.Editing.Revision then
                            match editing.LastChange with
                            | Some change ->
                                Functor.Domain.Syntax.SyntaxModel.markDirtyRange
                                    editing.Revision
                                    change.StartLine
                                    System.Int32.MaxValue
                                    documentState.Syntax
                            | None -> documentState.Syntax
                        else
                            documentState.Syntax }) workspace
        | ApplySyntaxEvent(documentId, revision, event) ->
            updateDocumentAtRevision documentId revision (fun documentState ->
                { documentState with
                    Syntax = Functor.Domain.Syntax.SyntaxLogic.update event documentState.Syntax }) workspace
        | ApplyNavigationEvent(documentId, revision, event) ->
            updateDocumentAtRevision documentId revision (fun documentState ->
                { documentState with
                    Navigation = Functor.Domain.Navigation.NavigationLogic.update event documentState.Navigation }) workspace
        | ApplyDiagnosticsEvent(documentId, revision, event) ->
            updateDocumentAtRevision documentId revision (fun documentState ->
                { documentState with
                    Diagnostics = Functor.Domain.Diagnostics.DiagnosticsLogic.update event documentState.Diagnostics }) workspace
