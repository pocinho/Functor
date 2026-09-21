namespace Functor.Domain.Document

open System

/// Events related to document metadata and lifecycle.
/// These do NOT modify the text buffer — EditingEvent handles that.
/// DocumentEvent only affects DocumentModel (identity, metadata, timestamps).
type DocumentMetadataEvent =
    // ────────────────────────────────────────────────
    // Metadata
    // ────────────────────────────────────────────────
    | RenameDocument of newName: string
    | MarkDocumentDirty
    | MarkDocumentClean

type DocumentPathEvent = SetDocumentPath of path: string option

type DocumentTimestampEvent = UpdateModifiedTimestamp of timestamp: DateTime

type DocumentEvent =
    | Metadata of DocumentMetadataEvent
    | Path of DocumentPathEvent
    | Timestamp of DocumentTimestampEvent

    static member RenameDocument name =
        Metadata(DocumentMetadataEvent.RenameDocument name)

    static member MarkDocumentDirty = Metadata DocumentMetadataEvent.MarkDocumentDirty
    static member MarkDocumentClean = Metadata DocumentMetadataEvent.MarkDocumentClean

    static member SetDocumentPath path =
        Path(DocumentPathEvent.SetDocumentPath path)

    static member UpdateModifiedTimestamp timestamp =
        Timestamp(DocumentTimestampEvent.UpdateModifiedTimestamp timestamp)

[<AutoOpen>]
module DocumentEventConstructors =
    let RenameDocument name = DocumentEvent.RenameDocument name
    let MarkDocumentDirty = DocumentEvent.MarkDocumentDirty
    let MarkDocumentClean = DocumentEvent.MarkDocumentClean
    let SetDocumentPath path = DocumentEvent.SetDocumentPath path

    let UpdateModifiedTimestamp timestamp =
        DocumentEvent.UpdateModifiedTimestamp timestamp
