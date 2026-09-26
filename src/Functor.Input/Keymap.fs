namespace Functor.Input

type KeyAction =
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
    | Backspace
    | Delete
    | InsertNewLine
    | SelectAll
    | Undo
    | Redo

type Keymap =
    { Bindings: Map<KeyboardInput, KeyAction> }

type ShellAction =
    | OpenCommandPalette
    | OpenSettings
  | OpenSearch
  | NextSearchResult
  | PreviousSearchResult

type EditorAction =
    | Copy
    | Cut
    | Paste
    | OpenFile
    | SaveFileAs
    | SaveFile
    | NewDocument
    | CloseDocument
    | ReopenClosedTab

module Keymap =
    let defaultKeymap =
        { Bindings =
            Map.ofList
                [ ({ Key = Key.Left
                     Modifiers = KeyModifiers.None },
                   KeyAction.MoveLeft)
                  ({ Key = Key.Left
                     Modifiers = KeyModifiers.Shift },
                   KeyAction.MoveLeft)
                  ({ Key = Key.Right
                     Modifiers = KeyModifiers.None },
                   KeyAction.MoveRight)
                  ({ Key = Key.Right
                     Modifiers = KeyModifiers.Shift },
                   KeyAction.MoveRight)
                  ({ Key = Key.Up
                     Modifiers = KeyModifiers.None },
                   KeyAction.MoveUp)
                  ({ Key = Key.Up
                     Modifiers = KeyModifiers.Shift },
                   KeyAction.MoveUp)
                  ({ Key = Key.Down
                     Modifiers = KeyModifiers.None },
                   KeyAction.MoveDown)
                  ({ Key = Key.Down
                     Modifiers = KeyModifiers.Shift },
                   KeyAction.MoveDown)
                  ({ Key = Key.Left
                     Modifiers = KeyModifiers.Control },
                   KeyAction.MoveWordLeft)
                  ({ Key = Key.Left
                     Modifiers = (KeyModifiers.Control ||| KeyModifiers.Shift) },
                   KeyAction.MoveWordLeft)
                  ({ Key = Key.Right
                     Modifiers = KeyModifiers.Control },
                   KeyAction.MoveWordRight)
                  ({ Key = Key.Right
                     Modifiers = (KeyModifiers.Control ||| KeyModifiers.Shift) },
                   KeyAction.MoveWordRight)
                  ({ Key = Key.Home
                     Modifiers = KeyModifiers.None },
                   KeyAction.MoveToLineStart)
                  ({ Key = Key.Home
                     Modifiers = KeyModifiers.Shift },
                   KeyAction.MoveToLineStart)
                  ({ Key = Key.End
                     Modifiers = KeyModifiers.None },
                   KeyAction.MoveToLineEnd)
                  ({ Key = Key.End
                     Modifiers = KeyModifiers.Shift },
                   KeyAction.MoveToLineEnd)
                  ({ Key = Key.Home
                     Modifiers = KeyModifiers.Control },
                   KeyAction.MoveToDocumentStart)
                  ({ Key = Key.Home
                     Modifiers = (KeyModifiers.Control ||| KeyModifiers.Shift) },
                   KeyAction.MoveToDocumentStart)
                  ({ Key = Key.End
                     Modifiers = KeyModifiers.Control },
                   KeyAction.MoveToDocumentEnd)
                  ({ Key = Key.End
                     Modifiers = (KeyModifiers.Control ||| KeyModifiers.Shift) },
                   KeyAction.MoveToDocumentEnd)
                  ({ Key = Key.Backspace
                     Modifiers = KeyModifiers.None },
                   KeyAction.Backspace)
                  ({ Key = Key.Delete
                     Modifiers = KeyModifiers.None },
                   KeyAction.Delete)
                  ({ Key = Key.Enter
                     Modifiers = KeyModifiers.None },
                   KeyAction.InsertNewLine)
                  ({ Key = Key.Character 'a'
                     Modifiers = KeyModifiers.Control },
                   KeyAction.SelectAll)
                  ({ Key = Key.Character 'z'
                     Modifiers = KeyModifiers.Control },
                   KeyAction.Undo)
                  ({ Key = Key.Character 'y'
                     Modifiers = KeyModifiers.Control },
                   KeyAction.Redo) ] }

    let create bindings = { Bindings = Map.ofList bindings }

    let bind input action keymap =
        { keymap with
            Bindings = keymap.Bindings.Add(input, action) }

    let resolveWith keymap input = Map.tryFind input keymap.Bindings

    let resolve input = resolveWith defaultKeymap input

    let resolveShell (input: KeyboardInput) =
        let commandModifier =
            input.Modifiers = KeyModifiers.Control || input.Modifiers = KeyModifiers.Meta

        let paletteModifier =
            input.Modifiers = (KeyModifiers.Control ||| KeyModifiers.Shift)
            || input.Modifiers = (KeyModifiers.Meta ||| KeyModifiers.Shift)

        if input.Key = Key.Character 'p' && paletteModifier then
            Some ShellAction.OpenCommandPalette
        elif input.Key = Key.Character ',' && commandModifier then
            Some ShellAction.OpenSettings
        elif input.Key = Key.Character 'f' && commandModifier then
          Some ShellAction.OpenSearch
        elif input.Key = Key.F3 && input.Modifiers = KeyModifiers.Shift then
          Some ShellAction.PreviousSearchResult
        elif input.Key = Key.F3 && input.Modifiers = KeyModifiers.None then
          Some ShellAction.NextSearchResult
        else
            None

    let resolveEditor (input: KeyboardInput) =
        let commandModifier =
            input.Modifiers = KeyModifiers.Control || input.Modifiers = KeyModifiers.Meta

        let shiftedCommandModifier =
            input.Modifiers = (KeyModifiers.Control ||| KeyModifiers.Shift)
            || input.Modifiers = (KeyModifiers.Meta ||| KeyModifiers.Shift)

        match input.Key with
        | Key.Character 'c' when commandModifier -> Some EditorAction.Copy
        | Key.Character 'x' when commandModifier -> Some EditorAction.Cut
        | Key.Character 'v' when commandModifier -> Some EditorAction.Paste
        | Key.Insert when input.Modifiers = KeyModifiers.Control -> Some EditorAction.Copy
        | Key.Insert when input.Modifiers = KeyModifiers.Shift -> Some EditorAction.Paste
        | Key.Delete when input.Modifiers = KeyModifiers.Shift -> Some EditorAction.Cut
        | Key.Character 'o' when commandModifier -> Some EditorAction.OpenFile
        | Key.Character 's' when shiftedCommandModifier -> Some EditorAction.SaveFileAs
        | Key.Character 's' when commandModifier -> Some EditorAction.SaveFile
        | Key.Character 'n' when commandModifier -> Some EditorAction.NewDocument
        | Key.Character 'w' when commandModifier -> Some EditorAction.CloseDocument
        | Key.Character 't' when shiftedCommandModifier -> Some EditorAction.ReopenClosedTab
        | _ -> None
