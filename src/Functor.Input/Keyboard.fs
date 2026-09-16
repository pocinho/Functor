namespace Functor.Input

type Key =
    | Character of char
    | Left
    | Right
    | Up
    | Down
    | Backspace
    | Delete
    | Enter
    | Escape

[<System.Flags>]
type KeyModifiers =
    | None = 0
    | Shift = 1
    | Control = 2
    | Alt = 4
    | Meta = 8

type KeyboardInput = { Key: Key; Modifiers: KeyModifiers }
