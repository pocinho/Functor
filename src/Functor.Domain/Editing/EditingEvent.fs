namespace Functor.Domain.Editing

/// Events related to text editing:
/// - character insertion
/// - deletion
/// - cursor movement
/// - selection manipulation
/// - line operations
type EditingEvent =
    // ────────────────────────────────────────────────
    // Text Input
    // ────────────────────────────────────────────────
    | InsertChar of char
    | InsertString of string
    | Backspace
    | Delete

    // ────────────────────────────────────────────────
    // Cursor Movement
    // ────────────────────────────────────────────────
    | MoveLeft
    | MoveRight
    | MoveUp
    | MoveDown
    | MoveToLineStart
    | MoveToLineEnd
    | MoveToDocumentStart
    | MoveToDocumentEnd

    // ────────────────────────────────────────────────
    // Selection
    // ────────────────────────────────────────────────
    | StartSelection
    | UpdateSelection
    | ClearSelection

    // ────────────────────────────────────────────────
    // Line Operations
    // ────────────────────────────────────────────────
    | InsertNewLine
    | DeleteLine
    | DuplicateLine

    // ────────────────────────────────────────────────
    // Undo / Redo
    // ────────────────────────────────────────────────
    | Undo
    | Redo

    // ────────────────────────────────────────────────
    // Editing Modes
    // ────────────────────────────────────────────────
    | ToggleOverwriteMode
