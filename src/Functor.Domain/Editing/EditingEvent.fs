namespace Functor.Domain.Editing

/// Events that insert or delete text in the buffer.
type TextInputEvent =
    | InsertChar of char
    | InsertString of string
    | Backspace
    | Delete
    | DeleteSelection

/// Events that move or explicitly position the cursor.
type CursorEvent =
    | MoveLeft
    | MoveRight
    | MoveUp
    | MoveDown
    | MoveWordLeft
    | MoveWordRight
    | MoveToLineStart
    | MoveToLineEnd
    | MoveToDocumentStart
    | MoveToDocumentEnd
    | SetCursor of Position

/// Events that manage the active selection.
type SelectionEvent =
    | StartSelection
    | UpdateSelection
    | SelectAll
    | ClearSelection
    | SetSelection of Selection option

/// Events that operate on complete lines.
type LineEvent =
    | InsertNewLine
    | DeleteLine
    | DuplicateLine

/// Events that navigate the edit history.
type HistoryEvent =
    | Undo
    | Redo

/// Events that change editing modes.
type EditingModeEvent =
    | ToggleOverwriteMode
    | SetOverwriteMode of bool

/// Events handled by the editing subdomain.
type EditingEvent =
    | TextInput of TextInputEvent
    | Cursor of CursorEvent
    | Selection of SelectionEvent
    | Line of LineEvent
    | History of HistoryEvent
    | Mode of EditingModeEvent

    static member InsertChar value =
        TextInput(TextInputEvent.InsertChar value)

    static member InsertString value =
        TextInput(TextInputEvent.InsertString value)

    static member Backspace = TextInput TextInputEvent.Backspace
    static member Delete = TextInput TextInputEvent.Delete
    static member DeleteSelection = TextInput TextInputEvent.DeleteSelection
    static member MoveLeft = Cursor CursorEvent.MoveLeft
    static member MoveRight = Cursor CursorEvent.MoveRight
    static member MoveUp = Cursor CursorEvent.MoveUp
    static member MoveDown = Cursor CursorEvent.MoveDown
    static member MoveWordLeft = Cursor CursorEvent.MoveWordLeft
    static member MoveWordRight = Cursor CursorEvent.MoveWordRight
    static member MoveToLineStart = Cursor CursorEvent.MoveToLineStart
    static member MoveToLineEnd = Cursor CursorEvent.MoveToLineEnd
    static member MoveToDocumentStart = Cursor CursorEvent.MoveToDocumentStart
    static member MoveToDocumentEnd = Cursor CursorEvent.MoveToDocumentEnd
    static member SetCursor value = Cursor(CursorEvent.SetCursor value)
    static member StartSelection = Selection SelectionEvent.StartSelection
    static member UpdateSelection = Selection SelectionEvent.UpdateSelection
    static member SelectAll = Selection SelectionEvent.SelectAll
    static member ClearSelection = Selection SelectionEvent.ClearSelection

    static member SetSelection value =
        Selection(SelectionEvent.SetSelection value)

    static member InsertNewLine = Line LineEvent.InsertNewLine
    static member DeleteLine = Line LineEvent.DeleteLine
    static member DuplicateLine = Line LineEvent.DuplicateLine
    static member Undo = History HistoryEvent.Undo
    static member Redo = History HistoryEvent.Redo
    static member ToggleOverwriteMode = Mode EditingModeEvent.ToggleOverwriteMode

    static member SetOverwriteMode value =
        Mode(EditingModeEvent.SetOverwriteMode value)

[<AutoOpen>]
module EditingEventConstructors =
    let InsertChar value = EditingEvent.InsertChar value
    let InsertString value = EditingEvent.InsertString value
    let Backspace = EditingEvent.Backspace
    let Delete = EditingEvent.Delete
    let DeleteSelection = EditingEvent.DeleteSelection
    let MoveLeft = EditingEvent.MoveLeft
    let MoveRight = EditingEvent.MoveRight
    let MoveUp = EditingEvent.MoveUp
    let MoveDown = EditingEvent.MoveDown
    let MoveWordLeft = EditingEvent.MoveWordLeft
    let MoveWordRight = EditingEvent.MoveWordRight
    let MoveToLineStart = EditingEvent.MoveToLineStart
    let MoveToLineEnd = EditingEvent.MoveToLineEnd
    let MoveToDocumentStart = EditingEvent.MoveToDocumentStart
    let MoveToDocumentEnd = EditingEvent.MoveToDocumentEnd
    let SetCursor value = EditingEvent.SetCursor value
    let StartSelection = EditingEvent.StartSelection
    let UpdateSelection = EditingEvent.UpdateSelection
    let SelectAll = EditingEvent.SelectAll
    let ClearSelection = EditingEvent.ClearSelection
    let SetSelection value = EditingEvent.SetSelection value
    let InsertNewLine = EditingEvent.InsertNewLine
    let DeleteLine = EditingEvent.DeleteLine
    let DuplicateLine = EditingEvent.DuplicateLine
    let Undo = EditingEvent.Undo
    let Redo = EditingEvent.Redo
    let ToggleOverwriteMode = EditingEvent.ToggleOverwriteMode
    let SetOverwriteMode value = EditingEvent.SetOverwriteMode value
