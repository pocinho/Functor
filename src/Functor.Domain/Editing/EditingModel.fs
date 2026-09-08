namespace Functor.Domain.Editing

/// Represents a position in the text buffer.
type Position = { Line: int; Column: int }

/// Represents a text selection (start and end cursor).
type Selection = { Start: Position; End: Position }

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
          UndoStack = []
          RedoStack = []
          IsDirty = false
          OverwriteMode = false }
