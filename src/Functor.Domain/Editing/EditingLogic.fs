namespace Functor.Domain.Editing

open System
open System.Globalization

/// Pure editing logic.
/// Applies an EditingEvent to an EditingModel and returns a new EditingModel.
module EditingLogic =

    let selectedText (model: EditingModel) =
        let normalized =
            model.Selection
            |> Option.map (fun selection ->
                if
                    compare (selection.Start.Line, selection.Start.Column) (selection.End.Line, selection.End.Column)
                    <= 0
                then
                    selection
                else
                    { Start = selection.End
                      End = selection.Start })

        match normalized with
        | None -> None
        | Some selection when selection.Start = selection.End -> None
        | Some selection ->
            let lines =
                model.Buffer
                |> List.skip selection.Start.Line
                |> List.take (selection.End.Line - selection.Start.Line + 1)

            let selected =
                lines
                |> List.mapi (fun index line ->
                    if index = 0 then
                        line.Substring(selection.Start.Column)
                    elif index = lines.Length - 1 then
                        line.Substring(0, selection.End.Column)
                    else
                        line)
                |> String.concat "\n"

            Some selected

    let private normalizeNewlines (text: string) =
        text.Replace("\r\n", "\n").Replace('\r', '\n')

    let private normalizedSelection (model: EditingModel) =
        model.Selection
        |> Option.map (fun selection ->
            if
                compare (selection.Start.Line, selection.Start.Column) (selection.End.Line, selection.End.Column)
                <= 0
            then
                selection
            else
                { Start = selection.End
                  End = selection.Start })
        |> Option.filter (fun selection -> selection.Start <> selection.End)

    // ────────────────────────────────────────────────
    // Helpers
    // ────────────────────────────────────────────────

    let private graphemeBoundaries (text: string) =
        let starts =
            if String.IsNullOrEmpty text then
                [| 0 |]
            else
                StringInfo.ParseCombiningCharacters text

        Array.append starts [| text.Length |]
        |> Array.distinct
        |> Array.filter (fun boundary ->
            boundary = 0
            || boundary = text.Length
            || not (Char.IsLowSurrogate text.[boundary] && Char.IsHighSurrogate text.[boundary - 1]))

    let private normalizeColumn (text: string) column =
        let column = max 0 (min text.Length column)

        graphemeBoundaries text
        |> Array.rev
        |> Array.tryFind (fun start -> start <= column)
        |> Option.defaultValue 0

    let private previousGraphemeBoundary (text: string) column =
        let column = normalizeColumn text column

        graphemeBoundaries text
        |> Array.rev
        |> Array.tryFind (fun start -> start < column)
        |> Option.defaultValue 0

    let private nextGraphemeBoundary (text: string) column =
        let column = normalizeColumn text column

        graphemeBoundaries text
        |> Array.tryFind (fun start -> start > column)
        |> Option.defaultValue text.Length

    let private graphemeCount (text: string) =
        max 0 ((graphemeBoundaries text).Length - 1)

    let private clampCursor (model: EditingModel) (cursor: Position) =
        let line = cursor.Line |> max 0 |> min (model.Buffer.Length - 1)

        let column = normalizeColumn model.Buffer.[line] cursor.Column

        { Line = line; Column = column }

    let private pushUndo (model: EditingModel) =
        { model with
            UndoStack = model.Buffer :: model.UndoStack
            RedoStack = [] }

    let private changeMetadata (before: string list) (after: string list) =
        let firstChanged =
            [ 0 .. max before.Length after.Length - 1 ]
            |> List.tryFind (fun line ->
                (before |> List.tryItem line) <> (after |> List.tryItem line))

        firstChanged
        |> Option.map (fun startLine ->
            let lineDelta = after.Length - before.Length
            let lastChangedNewLine =
                [ startLine .. after.Length - 1 ]
                |> List.rev
                |> List.tryFind (fun newLine ->
                    let oldLine = newLine - lineDelta
                    oldLine < 0
                    || oldLine >= before.Length
                    || before.[oldLine] <> after.[newLine])
                |> Option.defaultValue startLine

            { StartLine = startLine
              EndLine = lastChangedNewLine
              OldEndLine = lastChangedNewLine - lineDelta
              LineDelta = lineDelta })

    // ────────────────────────────────────────────────
    // Text Input
    // ────────────────────────────────────────────────

    let private insertChar (model: EditingModel) ch =
        let model = pushUndo model
        let line = model.Cursor.Line
        let col = model.Cursor.Column
        let current = model.Buffer.[line]

        let updatedLine = current.Insert(col, string ch)

        let updatedBuffer =
            model.Buffer |> List.mapi (fun i l -> if i = line then updatedLine else l)

        { model with
            Buffer = updatedBuffer
            Cursor = { model.Cursor with Column = col + 1 }
            PreferredColumn = None
            IsDirty = true }

    let private replaceSelection (model: EditingModel) (selection: Selection) (text: string) =
        let startText = model.Buffer.[selection.Start.Line]
        let endText = model.Buffer.[selection.End.Line]
        let prefix = startText.Substring(0, selection.Start.Column)
        let suffix = endText.Substring(selection.End.Column)
        let pieces = (normalizeNewlines text).Split('\n') |> Array.toList

        let replacement =
            match pieces with
            | [] -> [ prefix + suffix ]
            | [ piece ] -> [ prefix + piece + suffix ]
            | first :: rest ->
                let last = List.last rest
                let middle = rest |> List.take (rest.Length - 1)
                [ prefix + first ] @ middle @ [ last + suffix ]

        let buffer =
            (model.Buffer |> List.take selection.Start.Line)
            @ replacement
            @ (model.Buffer |> List.skip (selection.End.Line + 1))

        { model with
            Buffer = buffer
            Cursor =
                { Line = selection.Start.Line + replacement.Length - 1
                  Column = (List.last replacement).Length - suffix.Length }
            PreferredColumn = None
            Selection = None
            IsDirty = true }

    let private deleteSelection (model: EditingModel) =
        normalizedSelection model
        |> Option.map (fun selection -> replaceSelection (pushUndo model) selection "")
        |> Option.defaultValue model

    let private insertString (model: EditingModel) (text: string) =
        let text = normalizeNewlines text

        match normalizedSelection model with
        | Some selection -> replaceSelection (pushUndo model) selection text
        | None when text.Contains('\n') ->
            replaceSelection
                (pushUndo model)
                { Start = model.Cursor
                  End = model.Cursor }
                text
        | None ->
            if
                model.OverwriteMode
                && model.Cursor.Column < model.Buffer.[model.Cursor.Line].Length
            then
                let line = model.Buffer.[model.Cursor.Line]
                let mutable endColumn = model.Cursor.Column

                for _ in 1 .. graphemeCount text do
                    endColumn <- nextGraphemeBoundary line endColumn

                let count = endColumn - model.Cursor.Column

                let updated =
                    line.Remove(model.Cursor.Column, count).Insert(model.Cursor.Column, text)

                let buffer =
                    model.Buffer
                    |> List.mapi (fun index value -> if index = model.Cursor.Line then updated else value)

                { model with
                    Buffer = buffer
                    Cursor =
                        { model.Cursor with
                            Column = model.Cursor.Column + text.Length }
                    PreferredColumn = None
                    Selection = None
                    IsDirty = true }
            else
                text |> Seq.fold insertChar model

    let private backspace (model: EditingModel) =
        let line = model.Cursor.Line
        let col = model.Cursor.Column

        if col > 0 then
            let model = pushUndo model
            let current = model.Buffer.[line]
            let previousBoundary = previousGraphemeBoundary current col

            let updatedLine = current.Remove(previousBoundary, col - previousBoundary)

            let updatedBuffer =
                model.Buffer |> List.mapi (fun i l -> if i = line then updatedLine else l)

            { model with
                Buffer = updatedBuffer
                Cursor = { model.Cursor with Column = previousBoundary }
                PreferredColumn = None
                IsDirty = true }
        elif line = 0 then
            model
        else
            let model = pushUndo model
            let previous = model.Buffer.[line - 1]
            let current = model.Buffer.[line]

            let updatedBuffer =
                (model.Buffer |> List.take (line - 1))
                @ [ previous + current ]
                @ (model.Buffer |> List.skip (line + 1))

            { model with
                Buffer = updatedBuffer
                Cursor =
                    { Line = line - 1
                      Column = previous.Length }
                PreferredColumn = None
                IsDirty = true }

    let private delete (model: EditingModel) =
        let line = model.Cursor.Line
        let col = model.Cursor.Column
        let current = model.Buffer.[line]

        if normalizedSelection model |> Option.isSome then
            deleteSelection model
        elif col >= current.Length then
            model
        else
            let model = pushUndo model
            let next = nextGraphemeBoundary current col
            let updatedLine = current.Remove(col, next - col)

            let updatedBuffer =
                model.Buffer |> List.mapi (fun i l -> if i = line then updatedLine else l)

            { model with
                Buffer = updatedBuffer
                PreferredColumn = None
                IsDirty = true }

    // ────────────────────────────────────────────────
    // Cursor Movement
    // ────────────────────────────────────────────────

    let private moveCursor (model: EditingModel) newCursor =
        { model with
            Cursor = clampCursor model newCursor
            PreferredColumn = None }

    let private moveVertically (model: EditingModel) delta =
        let preferred = model.PreferredColumn |> Option.defaultValue model.Cursor.Column
        let line = max 0 (min (model.Buffer.Length - 1) (model.Cursor.Line + delta))
        let column = min preferred model.Buffer.[line].Length |> normalizeColumn model.Buffer.[line]

        { model with
            Cursor =
                { Line = line
                  Column = column }
            PreferredColumn = Some preferred }

    // ────────────────────────────────────────────────
    // Selection
    // ────────────────────────────────────────────────

    let private startSelection (model: EditingModel) =
        { model with
            Selection =
                Some
                    { Start = model.Cursor
                      End = model.Cursor } }

    let private updateSelection (model: EditingModel) =
        match model.Selection with
        | None -> model
        | Some sel ->
            { model with
                Selection = Some { sel with End = model.Cursor } }

    let private clearSelection (model: EditingModel) = { model with Selection = None }

    let private setSelection (model: EditingModel) selection =
        let normalized =
            selection
            |> Option.map (fun selection ->
                { Start = clampCursor model selection.Start
                  End = clampCursor model selection.End })

        { model with Selection = normalized }

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
            (model.Buffer |> List.take line)
            @ [ before; after ]
            @ (model.Buffer |> List.skip (line + 1))

        { model with
            Buffer = updatedBuffer
            Cursor = { Line = line + 1; Column = 0 }
            PreferredColumn = None
            IsDirty = true }

    let private deleteLine (model: EditingModel) =
        if model.Buffer.Length = 1 then
            model
        else
            let model = pushUndo model
            let line = model.Cursor.Line

            let updatedBuffer =
                model.Buffer
                |> List.mapi (fun i l -> i, l)
                |> List.filter (fun (i, _) -> i <> line)
                |> List.map snd

            let newLine = line |> min (updatedBuffer.Length - 1)

            { model with
                Buffer = updatedBuffer
                Cursor = { Line = newLine; Column = 0 }
                PreferredColumn = None
                IsDirty = true }

    let private duplicateLine (model: EditingModel) =
        let model = pushUndo model
        let line = model.Cursor.Line
        let text = model.Buffer.[line]

        let updatedBuffer =
            (model.Buffer |> List.take (line + 1))
            @ [ text ]
            @ (model.Buffer |> List.skip (line + 1))

        { model with
            Buffer = updatedBuffer
            PreferredColumn = None
            IsDirty = true }

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

    let private updateRaw (evt: EditingEvent) (model: EditingModel) : EditingModel =
        match evt with
        | InsertChar ch -> insertChar model ch
        | InsertString str -> insertString model str
        | Backspace ->
            match normalizedSelection model with
            | Some _ -> deleteSelection model
            | None -> backspace model
        | Delete -> delete model
        | DeleteSelection -> deleteSelection model

        | MoveLeft ->
            let text = model.Buffer.[model.Cursor.Line]
            moveCursor model { model.Cursor with Column = previousGraphemeBoundary text model.Cursor.Column }
        | MoveRight ->
            let text = model.Buffer.[model.Cursor.Line]
            moveCursor model { model.Cursor with Column = nextGraphemeBoundary text model.Cursor.Column }
        | MoveUp -> moveVertically model -1
        | MoveDown -> moveVertically model 1
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
        | SetCursor cursor -> moveCursor model cursor

        | StartSelection -> startSelection model
        | UpdateSelection -> updateSelection model
        | ClearSelection -> clearSelection model
        | SetSelection selection -> setSelection model selection

        | InsertNewLine -> insertNewLine model
        | DeleteLine -> deleteLine model
        | DuplicateLine -> duplicateLine model

        | Undo -> undo model
        | Redo -> redo model

        | ToggleOverwriteMode ->
            { model with
                OverwriteMode = not model.OverwriteMode }

        | SetOverwriteMode enabled -> { model with OverwriteMode = enabled }

    let update (evt: EditingEvent) (model: EditingModel) : EditingModel =
        let updated = updateRaw evt model
        let bufferChanged = updated.Buffer <> model.Buffer
        let lastChange =
            if bufferChanged then
                changeMetadata model.Buffer updated.Buffer
            else
                None

        { updated with
            Revision = if bufferChanged then model.Revision + 1L else model.Revision
            IsDirty = updated.Buffer <> updated.SavedBuffer
            LastChange = lastChange }
