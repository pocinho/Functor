namespace Functor.Domain.Document

open System

/// Events related to document metadata and lifecycle.
/// These do NOT modify the text buffer — EditingEvent handles that.
/// DocumentEvent only affects DocumentModel (identity, metadata, timestamps).
type DocumentEvent =
    // ────────────────────────────────────────────────
    // Metadata
    // ────────────────────────────────────────────────
    | RenameDocument of newName:string
    | MarkDocumentDirty
    | MarkDocumentClean

    // ────────────────────────────────────────────────
    // File Path
    // ────────────────────────────────────────────────
    | SetDocumentPath of path:string option

    // ────────────────────────────────────────────────
    // Timestamps
    // ────────────────────────────────────────────────
    | UpdateModifiedTimestamp of timestamp:DateTime
