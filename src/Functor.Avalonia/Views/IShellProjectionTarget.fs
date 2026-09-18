namespace Functor.Avalonia.Views

open Functor.Avalonia

type IShellProjectionTarget =
    abstract member ApplyShellInput: ShellViewInput -> unit
