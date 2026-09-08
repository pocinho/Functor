open System.Runtime.Versioning
open Avalonia
open Avalonia.Browser
open Functor.Avalonia

module Program =
    [<assembly: SupportedOSPlatform("browser")>]
    do ()

    [<CompiledName "BuildAvaloniaApp">]
    let buildAvaloniaApp () = AppBuilder.Configure<AvaloniaApp>()

    [<EntryPoint>]
    let main argv =
        task { do! (buildAvaloniaApp().WithInterFont().StartBrowserAppAsync("out")) }
        |> ignore

        0
