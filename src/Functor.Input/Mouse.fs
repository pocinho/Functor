namespace Functor.Input

type PointerButton =
    | Left
    | Middle
    | Right

type PointerPosition = { X: float; Y: float }

type PointerInput =
    { Position: PointerPosition
      Button: PointerButton option
      IsPressed: bool
      Modifiers: KeyModifiers }
