namespace Functor.Workspace

open System
open Functor.Domain.Document

module private PathIdentity =
    let key path =
        let canonicalPath = DocumentModel.canonicalizePath path
        if OperatingSystem.IsWindows() then canonicalPath.ToUpperInvariant() else canonicalPath

module WorkspaceQuery =
    let unsavedDocuments (workspace: WorkspaceModel) =
        workspace.Documents
        |> Map.toList
        |> List.map snd
        |> List.filter (fun document -> document.Editing.IsDirty || document.Document.Metadata.IsDirty)

    let unsavedDocumentIds workspace =
        unsavedDocuments workspace
        |> List.map (fun document -> document.Document.Id)

    let hasUnsavedDocuments workspace =
        unsavedDocuments workspace |> List.isEmpty |> not

    let canSafelyReplaceWorkspace workspace =
        not (hasUnsavedDocuments workspace)

    let documentsByPath workspace =
        workspace.Documents
        |> Map.toList
        |> List.choose (fun (_, document) ->
            document.Document.Metadata.Path
            |> Option.map (fun path -> PathIdentity.key path, document))
        |> Map.ofList

    let documentNames workspace =
        workspace.Documents
        |> Map.toList
        |> List.map (fun (_, document) -> document.Document.Metadata.Name)
