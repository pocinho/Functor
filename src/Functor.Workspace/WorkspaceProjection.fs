namespace Functor.Workspace

open System
open System.IO
open Functor.Domain.Document

type WorkspaceTabProjection =
    { DocumentId: DocumentId
      Index: int
      Name: string
      Path: string option
      IsActive: bool
      IsDirty: bool }

type WorkspaceFileTreeNode =
    { Key: string
      Name: string
      Path: string
      IsDirectory: bool
      DocumentId: DocumentId option
      IsDirty: bool
      Children: WorkspaceFileTreeNode list }

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

    let private pathSegments (path: string) =
        path.Split([| Path.DirectorySeparatorChar; Path.AltDirectorySeparatorChar |], StringSplitOptions.RemoveEmptyEntries)
        |> Array.toList

    let private relativeSegments (workspace: WorkspaceModel) path =
        match workspace.RootPath with
        | Some root when WorkspaceModel.isPathWithinRoot path workspace ->
            Path.GetRelativePath(root, DocumentModel.canonicalizePath path) |> pathSegments
        | _ ->
            pathSegments (DocumentModel.canonicalizePath path)

    let private nodePath parentPath name =
        if String.IsNullOrEmpty parentPath then name else Path.Combine(parentPath, name)

    let private addTreeDocument (workspace: WorkspaceModel) (documentState: PerDocumentSessionState) (root: WorkspaceFileTreeNode) =
        let document = documentState.Document

        let segments, path =
            match document.Metadata.Path with
            | Some documentPath -> relativeSegments workspace documentPath, DocumentModel.canonicalizePath documentPath
            | None -> [ document.Metadata.Name ], "untitled:" + document.Id.ToString("N")

        let rec insert parentPath remaining node =
            match remaining with
            | [] -> node
            | [ name ] ->
                let child =
                    { Key =
                          if document.Metadata.Path.IsSome then
                              "file:" + path
                          else
                              "document:" + document.Id.ToString("N")
                      Name = name
                      Path = path
                      IsDirectory = false
                      DocumentId = Some document.Id
                      IsDirty = documentIsDirty documentState
                      Children = [] }

                { node with
                    Children =
                        child
                        :: (node.Children |> List.filter (fun existing -> existing.Key <> child.Key))
                        |> List.sortBy (fun existing -> existing.Name.ToUpperInvariant()) }
            | name :: rest ->
                let childPath = nodePath parentPath name
                let existing =
                    node.Children
                    |> List.tryFind (fun child -> child.IsDirectory && child.Name = name)

                let directory =
                    existing
                    |> Option.defaultValue
                        { Key = "directory:" + childPath
                          Name = name
                          Path = childPath
                          IsDirectory = true
                          DocumentId = None
                          IsDirty = false
                          Children = [] }

                let updated = insert childPath rest directory

                { node with
                    Children =
                        updated
                        :: (node.Children
                            |> List.filter (fun child -> child.Key <> updated.Key))
                        |> List.sortBy (fun child -> child.Name.ToUpperInvariant()) }

        insert root.Path segments root

    let private rootNode (workspace: WorkspaceModel) =
        let path = workspace.RootPath |> Option.defaultValue ""
        let name =
            workspace.RootPath
            |> Option.map Path.GetFileName
            |> Option.filter (String.IsNullOrEmpty >> not)
            |> Option.defaultValue "Workspace"

        { Key = "workspace:" + workspace.Id.ToString("N")
          Name = name
          Path = path
          IsDirectory = true
          DocumentId = None
          IsDirty = false
          Children = [] }

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
                      IsDirty = documentIsDirty documentState }
            | None -> None)
        |> List.choose id

    let fileTree (workspace: WorkspaceModel) =
        workspace.TabOrder
        |> List.choose (fun documentId -> workspace.Documents |> Map.tryFind documentId)
        |> List.fold (fun root documentState -> addTreeDocument workspace documentState root) (rootNode workspace)

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
          FileTree = fileTree workspace
          ActiveDocument = activeDocument workspace }
