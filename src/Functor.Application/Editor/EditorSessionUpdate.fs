namespace Functor.Application

open Functor.Domain.Core
open Functor.Domain.Document

module internal EditorSessionUpdate =

    let syncActiveDocumentToWorkspace (currentState: AppSessionState) =
        match currentState.Model.ActiveDocument with
        | Some document when Functor.Workspace.WorkspaceModel.containsDocument document.Id currentState.Workspace ->
            let documentState: Functor.Workspace.PerDocumentSessionState =
                { Document = document
                  Editing = currentState.Model.Editing
                  Syntax = currentState.Model.Syntax
                  Navigation = currentState.Model.Navigation
                  Diagnostics = currentState.Model.Diagnostics
                  Mode = currentState.Model.Mode
                  View = currentState.Model.View
                  Auxiliary = currentState.Workspace.Documents[document.Id].Auxiliary }

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
            { CoreModel.empty with
                OpenDocuments = openDocuments |> List.map (fun value -> value.Document) }

    let apply (event: CoreEvent) (currentState: AppSessionState) =
        let state = syncActiveDocumentToWorkspace currentState

        let state =
            { state with
                Model = CoreLogic.update event state.Model }

        let state =
            match event with
            | NewDocument _
            | LoadDocument _ ->
                match state.Model.ActiveDocument with
                | Some document ->
                    { state with
                        Workspace =
                            Functor.Workspace.WorkspaceLogic.update
                                (Functor.Workspace.AddDocument document)
                                state.Workspace }
                | None -> state
            | CloseDocument documentId ->
                { state with
                    Workspace =
                        Functor.Workspace.WorkspaceLogic.update
                            (Functor.Workspace.RemoveDocument documentId)
                            state.Workspace }
            | SwitchDocument documentId ->
                { state with
                    Workspace =
                        Functor.Workspace.WorkspaceLogic.update
                            (Functor.Workspace.ActivateDocument documentId)
                            state.Workspace }
            | _ -> state

        match event with
        | CloseDocument _
        | SwitchDocument _ ->
            { state with
                Model = modelFromWorkspace state.Workspace }
        | _ -> syncActiveDocumentToWorkspace state
