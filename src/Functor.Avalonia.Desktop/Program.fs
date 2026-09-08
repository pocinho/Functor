namespace Functor.Avalonia.Desktop

open System
open Avalonia
open Functor.Avalonia

module Program =

    [<CompiledName "BuildAvaloniaApp">]
    let buildAvaloniaApp () =
        AppBuilder.Configure<AvaloniaApp>().UsePlatformDetect().WithInterFont().LogToTrace(areas = Array.empty)

    [<EntryPoint; STAThread>]
    let main argv =
        buildAvaloniaApp().StartWithClassicDesktopLifetime(argv)
