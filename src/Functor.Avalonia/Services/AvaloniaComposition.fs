namespace Functor.Avalonia.Services

open Avalonia.Controls
open Functor.Application

type AvaloniaComposition =
    { EditorServices: EditorServices
      SettingsStore: ISettingsStore }