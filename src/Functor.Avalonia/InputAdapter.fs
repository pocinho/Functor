namespace Functor.Avalonia

open Avalonia.Input
open Functor.Domain.Editing
open Functor.Input

module InputAdapter =
    let private modifiers (value: Avalonia.Input.KeyModifiers) =
        let mutable result = KeyModifiers.None

        if value.HasFlag(Avalonia.Input.KeyModifiers.Shift) then
            result <- result ||| KeyModifiers.Shift

        if value.HasFlag(Avalonia.Input.KeyModifiers.Control) then
            result <- result ||| KeyModifiers.Control

        if value.HasFlag(Avalonia.Input.KeyModifiers.Alt) then
            result <- result ||| KeyModifiers.Alt

        if value.HasFlag(Avalonia.Input.KeyModifiers.Meta) then
            result <- result ||| KeyModifiers.Meta

        result

    let private key (value: Avalonia.Input.Key) =
        match value with
        | Avalonia.Input.Key.Left -> Some Functor.Input.Key.Left
        | Avalonia.Input.Key.Right -> Some Functor.Input.Key.Right
        | Avalonia.Input.Key.Up -> Some Functor.Input.Key.Up
        | Avalonia.Input.Key.Down -> Some Functor.Input.Key.Down
        | Avalonia.Input.Key.Back -> Some Functor.Input.Key.Backspace
        | Avalonia.Input.Key.Delete -> Some Functor.Input.Key.Delete
        | Avalonia.Input.Key.Enter -> Some Functor.Input.Key.Enter
        | _ -> None

    let editingEvent (keyValue: Avalonia.Input.Key) (keyModifiers: Avalonia.Input.KeyModifiers) =
        match key keyValue with
        | None -> None
        | Some semanticKey ->
            match
                Keymap.resolve
                    { Key = semanticKey
                      Modifiers = modifiers keyModifiers }
            with
            | Some KeyAction.MoveLeft -> Some EditingEvent.MoveLeft
            | Some KeyAction.MoveRight -> Some EditingEvent.MoveRight
            | Some KeyAction.MoveUp -> Some EditingEvent.MoveUp
            | Some KeyAction.MoveDown -> Some EditingEvent.MoveDown
            | Some KeyAction.Backspace -> Some EditingEvent.Backspace
            | Some KeyAction.Delete -> Some EditingEvent.Delete
            | Some KeyAction.InsertNewLine -> Some EditingEvent.InsertNewLine
            | None -> None

    let shellAction (keyValue: Avalonia.Input.Key) (keyModifiers: Avalonia.Input.KeyModifiers) =
        let semanticKey =
            match keyValue with
            | Avalonia.Input.Key.P -> Some(Functor.Input.Key.Character 'p')
            | Avalonia.Input.Key.OemComma -> Some(Functor.Input.Key.Character ',')
            | _ -> None

        semanticKey
        |> Option.bind (fun key ->
            Keymap.resolveShell
                { Key = key
                  Modifiers = modifiers keyModifiers })

    let editorAction (keyValue: Avalonia.Input.Key) (keyModifiers: Avalonia.Input.KeyModifiers) =
        let semanticKey =
            match keyValue with
            | Avalonia.Input.Key.C -> Some(Functor.Input.Key.Character 'c')
            | Avalonia.Input.Key.X -> Some(Functor.Input.Key.Character 'x')
            | Avalonia.Input.Key.V -> Some(Functor.Input.Key.Character 'v')
            | Avalonia.Input.Key.O -> Some(Functor.Input.Key.Character 'o')
            | Avalonia.Input.Key.S -> Some(Functor.Input.Key.Character 's')
            | Avalonia.Input.Key.N -> Some(Functor.Input.Key.Character 'n')
            | Avalonia.Input.Key.W -> Some(Functor.Input.Key.Character 'w')
            | Avalonia.Input.Key.T -> Some(Functor.Input.Key.Character 't')
            | _ -> None

        semanticKey
        |> Option.bind (fun key ->
            Keymap.resolveEditor
                { Key = key
                  Modifiers = modifiers keyModifiers })

    let textInput text =
        if System.String.IsNullOrEmpty(text) then
            None
        else
            Some { Text = text }

    let pointerInput
        (position: Avalonia.Point)
        (button: Avalonia.Input.MouseButton option)
        isPressed
        keyModifiers
        : PointerInput =
        let semanticButton =
            button
            |> Option.bind (function
                | Avalonia.Input.MouseButton.Left -> Some PointerButton.Left
                | Avalonia.Input.MouseButton.Middle -> Some PointerButton.Middle
                | Avalonia.Input.MouseButton.Right -> Some PointerButton.Right
                | _ -> None)

        { Position = { X = position.X; Y = position.Y }
          Button = semanticButton
          IsPressed = isPressed
          Modifiers = modifiers keyModifiers }
