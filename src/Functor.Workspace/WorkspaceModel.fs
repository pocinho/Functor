namespace Functor.Workspace

open System
open System.IO
open Functor.Domain.Core
open Functor.Domain.Diagnostics
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Navigation
open Functor.Domain.Syntax

type WorkspaceId = Guid

/// State that belongs to one open document and survives tab switches.
type PerDocumentSessionState =
    { Document: DocumentModel
      Editing: EditingModel
      Syntax: SyntaxModel
      Navigation: NavigationModel
      Diagnostics: DiagnosticsModel
      Mode: EditorMode
      View: ViewState }

/// The workspace owns document membership and the active tab. Document state is
/// keyed by identity so switching tabs cannot reset editing or rendering state.
type RecentlyClosedDocument = { Path: string; Name: string }

type WorkspaceModel = { Id: WorkspaceId; RootPath: string option; Settings: WorkspaceSettings; ActiveDocumentId: DocumentId option; Documents: Map<DocumentId, PerDocumentSessionState>; TabOrder: DocumentId list; RecentlyClosedDocuments: RecentlyClosedDocument list }

module WorkspaceModel =

    let canonicalizeRootPath (path: string) =
        if String.IsNullOrWhiteSpace path then
            invalidArg (nameof path) "Workspace root path cannot be empty."

        let fullPath = Path.GetFullPath path
        let root = Path.GetPathRoot fullPath

        if String.IsNullOrEmpty root then
            fullPath
        elif String.Equals(fullPath, root, StringComparison.OrdinalIgnoreCase) then
            root
        else
            fullPath.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)

    let private sessionForDocument document =
        let editing = EditingModel.createFromText document.InitialText (EditingModel.create ())

        { Document = document
          Editing = editing
          Syntax = SyntaxModel.createForDocument document.Id editing.Revision
          Navigation = NavigationModel.create ()
          Diagnostics = DiagnosticsModel.create ()
          Mode = EditorMode.Normal
          View =
              { Viewport = { Width = 0; Height = 0 }
                VerticalOffset = 0
                HorizontalOffset = 0 } }

    let create (rootPath: string option) =
        { Id = Guid.NewGuid(); RootPath = rootPath |> Option.map canonicalizeRootPath; Settings = WorkspaceSettings.defaults; ActiveDocumentId = None; Documents = Map.empty; TabOrder = []; RecentlyClosedDocuments = [] }

    let empty = create None

    let activeDocument workspace =
        workspace.ActiveDocumentId
        |> Option.bind (fun documentId -> workspace.Documents |> Map.tryFind documentId)

    let tryFindDocument documentId workspace =
        workspace.Documents |> Map.tryFind documentId

    let containsDocument documentId workspace =
        workspace.Documents |> Map.containsKey documentId

    let tryFindDocumentByPath path workspace =
        let canonicalPath = DocumentModel.canonicalizePath path

        workspace.TabOrder
        |> List.tryPick (fun documentId ->
            match workspace.Documents |> Map.tryFind documentId with
            | Some document when
                document.Document.Metadata.Path
                |> Option.exists (fun documentPath ->
                    String.Equals(documentPath, canonicalPath, StringComparison.OrdinalIgnoreCase)) ->
                Some document
            | _ -> None)

    let isPathWithinRoot path workspace =
        match workspace.RootPath with
        | None -> true
        | Some root ->
            let canonicalPath = DocumentModel.canonicalizePath path
            let rootWithSeparator = root.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar) + string Path.DirectorySeparatorChar
            canonicalPath = root || canonicalPath.StartsWith(rootWithSeparator, StringComparison.OrdinalIgnoreCase)

    let consumeRecentlyClosed path workspace =
        let canonicalPath = DocumentModel.canonicalizePath path

        { workspace with
            RecentlyClosedDocuments =
                workspace.RecentlyClosedDocuments
                |> List.filter (fun closed -> not (String.Equals(closed.Path, canonicalPath, StringComparison.OrdinalIgnoreCase))) }

    let addDocument (document: DocumentModel) (workspace: WorkspaceModel) =
        let existingDocument =
            document.Metadata.Path
            |> Option.bind (fun path -> tryFindDocumentByPath path workspace)

        match existingDocument with
        | Some existing ->
            { workspace with ActiveDocumentId = Some existing.Document.Id }
        | None when containsDocument document.Id workspace ->
            workspace
        | None ->
            let documents = workspace.Documents |> Map.add document.Id (sessionForDocument document)

            { workspace with
                ActiveDocumentId = Option.orElse (Some document.Id) workspace.ActiveDocumentId
                Documents = documents
                TabOrder = workspace.TabOrder @ [ document.Id ] }
