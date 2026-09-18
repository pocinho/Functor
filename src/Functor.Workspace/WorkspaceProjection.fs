namespace Functor.Workspace

open Functor.Domain.Document

type WorkspaceTabProjection =
    { DocumentId: DocumentId
      Index: int
      Name: string
      Path: string option
      IsActive: bool
      IsDirty: bool
      AgentIsOpen: bool }

type ActiveDocumentProjection =
    { DocumentId: DocumentId
      Name: string
      Path: string option
      IsDirty: bool
      Session: PerDocumentSessionState }

type WorkspaceProjection =
    { WorkspaceId: WorkspaceId
      RootPath: string option
      Tabs: WorkspaceTabProjection list
      FileTree: WorkspaceFileTreeNode
      ActiveDocument: ActiveDocumentProjection option }

module WorkspaceProjection =

    let private documentIsDirty documentState =
        documentState.Editing.IsDirty || documentState.Document.Metadata.IsDirty

    let tabs (workspace: WorkspaceModel) =
        workspace.TabOrder
        |> List.mapi (fun index documentId ->
            match workspace.Documents |> Map.tryFind documentId with
            | Some documentState ->
                Some
                    { DocumentId = documentId
                      Index = index
                      Name = documentState.Document.Metadata.Name
                      Path = documentState.Document.Metadata.Path
                      IsActive = workspace.ActiveDocumentId = Some documentId
                      IsDirty = documentIsDirty documentState
                      AgentIsOpen = documentState.Auxiliary.Agent.IsOpen }
            | None -> None)
        |> List.choose id

    let activeDocument (workspace: WorkspaceModel) =
        WorkspaceModel.activeDocument workspace
        |> Option.map (fun documentState ->
            { DocumentId = documentState.Document.Id
              Name = documentState.Document.Metadata.Name
              Path = documentState.Document.Metadata.Path
              IsDirty = documentIsDirty documentState
              Session = documentState })

    let create (workspace: WorkspaceModel) =
        { WorkspaceId = workspace.Id
          RootPath = workspace.RootPath
          Tabs = tabs workspace
          FileTree = WorkspaceFileTree.create workspace
          ActiveDocument = activeDocument workspace }
