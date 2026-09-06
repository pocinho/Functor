namespace Functor.Domain.Editing

/// Pure editing logic.
/// Applies an EditingEvent to an EditingModel and returns a new EditingModel.
module EditingLogic =

    // ────────────────────────────────────────────────
    // Helpers
    // ────────────────────────────────────────────────

    let private clampCursor (model: EditingModel) (cursor: Position) =
        let line =
            cursor.Line
            |> max 0
            |> min (model.Buffer.Length - 1)

        let column =
            cursor.Column
            |> max 0
            |> min (model.Buffer.[line].Length)

        { Line = line; Column = column }

    let private pushUndo (model: EditingModel) =
        { model with
            UndoStack = model.Buffer :: model.UndoStack
            RedoStack = [] }

    // ────────────────────────────────────────────────
    // Text Input
    // ────────────────────────────────────────────────

    let private insertChar (model: EditingModel) ch =
        let model = pushUndo model
        let line = model.Cursor.Line
        let col = model.Cursor.Column
        let current = model.Buffer.[line]

        let updatedLine =
            current.Insert(col, string ch)

        let updatedBuffer =
            model.Buffer
            |> List.mapi (fun i l -> if i = line then updatedLine else l)

        { model with
            Buffer = updatedBuffer
            Cursor = { model.Cursor with Column = col + 1 }
            IsDirty = true }

    let private insertString (model: EditingModel) str =
        str |> Seq.fold insertChar model

    let private backspace (model: EditingModel) =
        if model.Cursor.Column = 0 then model
        else
            let model = pushUndo model
            let line = model.Cursor.Line
            let col = model.Cursor.Column
            let current = model.Buffer.[line]

            let updatedLine =
                current.Remove(col - 1, 1)

            let updatedBuffer =
                model.Buffer
                |> List.mapi (fun i l -> if i = line then updatedLine else l)

            { model with
                Buffer = updatedBuffer
                Cursor = { model.Cursor with Column = col - 1 }
                IsDirty = true }

    let private delete (model: EditingModel) =
        let line = model.Cursor.Line
        let col = model.Cursor.Column
        let current = model.Buffer.[line]

        if col >= current.Length then model
        else
            let model = pushUndo model
            let updatedLine = current.Remove(col, 1)

            let updatedBuffer =
                model.Buffer
                |> List.mapi (fun i l -> if i = line then updatedLine else l)

            { model with
                Buffer = updatedBuffer
                IsDirty = true }

    // ────────────────────────────────────────────────
    // Cursor Movement
    // ────────────────────────────────────────────────

    let private moveCursor (model: EditingModel) newCursor =
        { model with Cursor = clampCursor model newCursor }

    // ────────────────────────────────────────────────
    // Selection
    // ────────────────────────────────────────────────

    let private startSelection (model: EditingModel) =
        { model with Selection = Some { Start = model.Cursor; End = model.Cursor } }

    let private updateSelection (model: EditingModel) =
        match model.Selection with
        | None -> model
        | Some sel ->
            { model with Selection = Some { sel with End = model.Cursor } }

    let private clearSelection (model: EditingModel) =
        { model with Selection = None }

    // ────────────────────────────────────────────────
    // Line Operations
    // ────────────────────────────────────────────────

    let private insertNewLine (model: EditingModel) =
        let model = pushUndo model
        let line = model.Cursor.Line
        let col = model.Cursor.Column
        let current = model.Buffer.[line]

        let before = current.Substring(0, col)
        let after = current.Substring(col)

        let updatedBuffer =
            model.Buffer
            |> List.mapi (fun i l ->
                if i = line then before
                elif i = line + 1 then after
                else l)

        let updatedBuffer =
            (model.Buffer |> List.take line)
            @ [before; after]
            @ (model.Buffer |> List.skip (line + 1))

        { model with
            Buffer = updatedBuffer
            Cursor = { Line = line + 1; Column = 0 }
            IsDirty = true }

    let private deleteLine (model: EditingModel) =
        if model.Buffer.Length = 1 then model
        else
            let model = pushUndo model
            let line = model.Cursor.Line

            let updatedBuffer =
                model.Buffer
                |> List.mapi (fun i l -> i, l)
                |> List.filter (fun (i, _) -> i <> line)
                |> List.map snd

            let newLine =
                line |> min (updatedBuffer.Length - 1)

            { model with
                Buffer = updatedBuffer
                Cursor = { Line = newLine; Column = 0 }
                IsDirty = true }

    let private duplicateLine (model: EditingModel) =
        let model = pushUndo model
        let line = model.Cursor.Line
        let text = model.Buffer.[line]

        let updatedBuffer =
            (model.Buffer |> List.take (line + 1))
            @ [text]
            @ (model.Buffer |> List.skip (line + 1))

        { model with
            Buffer = updatedBuffer
            IsDirty = true }

    // ────────────────────────────────────────────────
    // Undo / Redo
    // ────────────────────────────────────────────────

    let private undo (model: EditingModel) =
        match model.UndoStack with
        | [] -> model
        | prev :: rest ->
            { model with
                Buffer = prev
                UndoStack = rest
                RedoStack = model.Buffer :: model.RedoStack
                IsDirty = true }

    let private redo (model: EditingModel) =
        match model.RedoStack with
        | [] -> model
        | next :: rest ->
            { model with
                Buffer = next
                RedoStack = rest
                UndoStack = model.Buffer :: model.UndoStack
                IsDirty = true }

    // ────────────────────────────────────────────────
    // Main Update Function
    // ────────────────────────────────────────────────

    let update (evt: EditingEvent) (model: EditingModel) : EditingModel =
        match evt with
        | InsertChar ch -> insertChar model ch
        | InsertString str -> insertString model str
        | Backspace -> backspace model
        | Delete -> delete model

        | MoveLeft -> moveCursor model { model.Cursor with Column = model.Cursor.Column - 1 }
        | MoveRight -> moveCursor model { model.Cursor with Column = model.Cursor.Column + 1 }
        | MoveUp -> moveCursor model { model.Cursor with Line = model.Cursor.Line - 1 }
        | MoveDown -> moveCursor model { model.Cursor with Line = model.Cursor.Line + 1 }
        | MoveToLineStart -> moveCursor model { model.Cursor with Column = 0 }
        | MoveToLineEnd ->
            let line = model.Cursor.Line
            let col = model.Buffer.[line].Length
            moveCursor model { Line = line; Column = col }
        | MoveToDocumentStart -> moveCursor model { Line = 0; Column = 0 }
        | MoveToDocumentEnd ->
            let last = model.Buffer.Length - 1
            let col = model.Buffer.[last].Length
            moveCursor model { Line = last; Column = col }

        | StartSelection -> startSelection model
        | UpdateSelection -> updateSelection model
        | ClearSelection -> clearSelection model

        | InsertNewLine -> insertNewLine model
        | DeleteLine -> deleteLine model
        | DuplicateLine -> duplicateLine model

        | Undo -> undo model
        | Redo -> redo model

        | ToggleOverwriteMode ->
            // Overwrite mode not yet implemented — placeholder
            model
