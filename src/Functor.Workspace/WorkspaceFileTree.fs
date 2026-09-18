namespace Functor.Workspace

open System
open System.IO
open Functor.Domain.Document

type WorkspaceFileTreeNode =
    { Key: string
      Name: string
      Path: string
      IsDirectory: bool
      DocumentId: DocumentId option
      IsDirty: bool
      Children: WorkspaceFileTreeNode list }

module WorkspaceFileTree =

    let private documentIsDirty documentState =
        documentState.Editing.IsDirty || documentState.Document.Metadata.IsDirty

    let private pathSegments (path: string) =
        path.Split(
            [| Path.DirectorySeparatorChar; Path.AltDirectorySeparatorChar |],
            StringSplitOptions.RemoveEmptyEntries
        )
        |> Array.toList

    let private relativeSegments (workspace: WorkspaceModel) path =
        match workspace.RootPath with
        | Some root when WorkspaceModel.isPathWithinRoot path workspace ->
            Path.GetRelativePath(root, DocumentModel.canonicalizePath path) |> pathSegments
        | Some _
        | None -> [ Path.GetFileName(DocumentModel.canonicalizePath path) ]

    let private nodePath parentPath name =
        if String.IsNullOrEmpty parentPath then
            name
        else
            Path.Combine(parentPath, name)

    let private addOpenDocument
        (workspace: WorkspaceModel)
        (documentState: PerDocumentSessionState)
        (root: WorkspaceFileTreeNode)
        =
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
                        :: (node.Children |> List.filter (fun child -> child.Key <> updated.Key))
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

    let private openDocumentAtPath path (workspace: WorkspaceModel) =
        workspace.TabOrder
        |> List.tryPick (fun documentId ->
            workspace.Documents
            |> Map.tryFind documentId
            |> Option.bind (fun documentState ->
                documentState.Document.Metadata.Path
                |> Option.bind (fun documentPath ->
                    if String.Equals(documentPath, path, StringComparison.OrdinalIgnoreCase) then
                        Some documentState
                    else
                        None)))

    let private fileNode workspace path =
        let canonicalPath = DocumentModel.canonicalizePath path
        let openDocument = openDocumentAtPath canonicalPath workspace

        { Key = "file:" + canonicalPath
          Name = Path.GetFileName(canonicalPath)
          Path = canonicalPath
          IsDirectory = false
          DocumentId = openDocument |> Option.map (fun documentState -> documentState.Document.Id)
          IsDirty = openDocument |> Option.exists documentIsDirty
          Children = [] }

    let rec private directoryNode workspace path =
        let directories =
            try
                Directory.EnumerateDirectories(path) |> Seq.toList
            with _ ->
                []

        let files =
            try
                Directory.EnumerateFiles(path) |> Seq.toList
            with _ ->
                []

        let children =
            (directories |> List.map (directoryNode workspace))
            @ (files |> List.map (fileNode workspace))
            |> List.sortBy (fun child -> child.Name.ToUpperInvariant())

        { Key = "directory:" + DocumentModel.canonicalizePath path
          Name = Path.GetFileName(path)
          Path = DocumentModel.canonicalizePath path
          IsDirectory = true
          DocumentId = None
          IsDirty = false
          Children = children }

    let create (workspace: WorkspaceModel) =
        match workspace.RootPath with
        | Some root when Directory.Exists root -> directoryNode workspace root
        | _ ->
            workspace.TabOrder
            |> List.choose (fun documentId -> workspace.Documents |> Map.tryFind documentId)
            |> List.fold (fun root documentState -> addOpenDocument workspace documentState root) (rootNode workspace)
