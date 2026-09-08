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
    | DeleteSelection

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
    | SetCursor of Position

    // ────────────────────────────────────────────────
    // Selection
    // ────────────────────────────────────────────────
    | StartSelection
    | UpdateSelection
    | ClearSelection
    | SetSelection of Selection option

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
    | SetOverwriteMode of bool
