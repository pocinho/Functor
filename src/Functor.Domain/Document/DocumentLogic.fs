namespace Functor.Domain.Document

open System

/// Pure document metadata logic.
/// Applies a DocumentEvent to a DocumentModel and returns a new DocumentModel.
/// This module intentionally does NOT modify the text buffer — EditingModel handles that.
module DocumentLogic =

    let private rename (doc: DocumentModel) (newName: string) =
        { doc with
            Metadata =
                { doc.Metadata with
                    Name = newName
                    ModifiedAt = Some DateTime.UtcNow } }

    let private markDirty (doc: DocumentModel) =
        { doc with
            Metadata =
                { doc.Metadata with
                    IsDirty = true
                    ModifiedAt = Some DateTime.UtcNow } }

    let private markClean (doc: DocumentModel) =
        { doc with
            Metadata =
                { doc.Metadata with
                    IsDirty = false
                    ModifiedAt = Some DateTime.UtcNow } }

    let private setPath (doc: DocumentModel) (path: string option) =
        let newName =
            match path with
            | Some p -> System.IO.Path.GetFileName(p)
            | None -> doc.Metadata.Name

        { doc with
            Metadata =
                { doc.Metadata with
                    Path = path
                    Name = newName
                    ModifiedAt = Some DateTime.UtcNow } }

    let private updateTimestamp (doc: DocumentModel) (ts: DateTime) =
        { doc with
            Metadata =
                { doc.Metadata with
                    ModifiedAt = Some ts } }

    /// Main update function.
    /// Routes DocumentEvents to the appropriate pure metadata operations.
    let update (evt: DocumentEvent) (doc: DocumentModel) : DocumentModel =
        match evt with
        | RenameDocument newName ->
            rename doc newName

        | MarkDocumentDirty ->
            markDirty doc

        | MarkDocumentClean ->
            markClean doc

        | SetDocumentPath path ->
            setPath doc path

        | UpdateModifiedTimestamp ts ->
            updateTimestamp doc ts
