namespace Functor.Domain.Document

open System

/// Unique identifier for a document.
/// This is a real Guid, not an alias, to keep the architecture simple.
type DocumentId = Guid

/// Metadata about a document (path, name, timestamps, dirty flag).
type DocumentMetadata =
    {
        /// Full file path (None for unsaved/untitled documents).
        Path : string option

        /// File name (derived from Path or user-assigned).
        Name : string

        /// Whether the document has unsaved changes.
        IsDirty : bool

        /// Timestamp of creation.
        CreatedAt : DateTime

        /// Timestamp of last modification.
        ModifiedAt : DateTime option
    }

/// Represents the complete state of a document:
/// - identity
/// - metadata
/// - initial text buffer (EditingModel will manage the buffer itself)
type DocumentModel =
    {
        Id : DocumentId
        Metadata : DocumentMetadata
        InitialText : string
    }

module DocumentModel =

    /// Creates a new unsaved document with a given name.
    let createUntitled (name: string) =
        {
            Id = Guid.NewGuid()
            Metadata =
                {
                    Path = None
                    Name = name
                    IsDirty = false
                    CreatedAt = DateTime.UtcNow
                    ModifiedAt = None
                }
            InitialText = ""
        }

    /// Creates a new document from a file path and initial text.
    let createFromFile (path: string) (text: string) =
        {
            Id = Guid.NewGuid()
            Metadata =
                {
                    Path = Some path
                    Name = System.IO.Path.GetFileName(path)
                    IsDirty = false
                    CreatedAt = DateTime.UtcNow
                    ModifiedAt = None
                }
            InitialText = text
        }

    /// Marks the document as dirty (modified).
    let markDirty (doc: DocumentModel) =
        { doc with
            Metadata =
                { doc.Metadata with
                    IsDirty = true
                    ModifiedAt = Some DateTime.UtcNow } }

    /// Marks the document as clean (saved).
    let markClean (doc: DocumentModel) =
        { doc with
            Metadata =
                { doc.Metadata with
                    IsDirty = false
                    ModifiedAt = Some DateTime.UtcNow } }

    /// Renames the document (does not change path).
    let rename (doc: DocumentModel) (newName: string) =
        { doc with
            Metadata =
                { doc.Metadata with
                    Name = newName
                    ModifiedAt = Some DateTime.UtcNow } }
