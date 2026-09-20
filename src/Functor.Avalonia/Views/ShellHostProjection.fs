namespace Functor.Avalonia.Views

open System
open System.IO
open Functor.Domain.Document
open Functor.Workspace

module ShellHostProjection =

    let hasDiskWorkspace (workspace: WorkspaceModel) =
        match workspace.RootPath with
        | Some root -> Directory.Exists root
        | None -> false

    let workspaceTreeKey (workspace: WorkspaceModel) =
        let documents =
            workspace.Documents
            |> Map.toList
            |> List.choose (fun (_, documentState) ->
                let isDirty =
                    documentState.Editing.IsDirty || documentState.Document.Metadata.IsDirty

                if isDirty then
                    Some(
                        documentState.Document.Metadata.Path
                        |> Option.map DocumentModel.canonicalizePath,
                        documentState.Document.Metadata.Name,
                        isDirty
                    )
                else
                    None)

        match workspace.RootPath with
        | Some root -> Some(DocumentModel.canonicalizePath root), documents, None
        | None -> None, documents, Some workspace.TabOrder
