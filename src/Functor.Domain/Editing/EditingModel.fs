namespace Functor.Domain.Editing

/// Represents a position in the text buffer.
type Position = { Line: int; Column: int }

/// Represents a text selection (start and end cursor).
type Selection = { Start: Position; End: Position }

/// Describes the line span affected by the latest buffer mutation.
type EditingChange = { StartLine: int; EndLine: int; OldEndLine: int; LineDelta: int }

/// Represents the complete editing state:
/// - cursor
/// - selection
/// - text buffer
/// - undo/redo stacks
/// - dirty flag
type EditingModel =
    {
        Cursor: Position
        PreferredColumn: int option
        Selection: Selection option

        /// The text buffer, represented as a list of lines.
        Buffer: string list

        /// The buffer as it existed at the last successful load or save.
        SavedBuffer: string list

        /// Monotonically increasing version of the in-memory buffer.
        Revision: int64

        /// Metadata for the latest buffer mutation, if the last event changed text.
        LastChange: EditingChange option

        /// Undo stack: each entry is a snapshot of the buffer.
        UndoStack: string list list

        /// Redo stack: each entry is a snapshot of the buffer.
        RedoStack: string list list

        /// Whether the buffer has unsaved changes.
        IsDirty: bool

        /// Whether inserted characters replace existing characters.
        OverwriteMode: bool
    }

module EditingModel =

    /// Creates a fresh editing model with a single empty line.
    let create () =
        { Cursor = { Line = 0; Column = 0 }
          PreferredColumn = None
          Selection = None
          Buffer = [ "" ]
          SavedBuffer = [ "" ]
          Revision = 0L
          LastChange = None
          UndoStack = []
          RedoStack = []
          IsDirty = false
          OverwriteMode = false }

    let createFromText (text: string) (model: EditingModel) =
        let lines = text.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n') |> Array.toList
        let buffer = if lines.IsEmpty then [ "" ] else lines

        { model with
            Cursor = { Line = 0; Column = 0 }
            PreferredColumn = None
            Selection = None
            Buffer = buffer
            SavedBuffer = buffer
            Revision = 0L
            LastChange = None
            UndoStack = []
            RedoStack = []
            IsDirty = false }

    let withText = createFromText
