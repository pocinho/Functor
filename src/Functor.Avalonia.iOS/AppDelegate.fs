namespace Functor.Avalonia.iOS

open Foundation
open Avalonia
open Avalonia.iOS

// The UIApplicationDelegate for the application. This class is responsible for launching the
// User Interface of the application, as well as listening (and optionally responding) to
// application events from iOS.
[<Register("AppDelegate")>]
type AppDelegate() =
    inherit AvaloniaAppDelegate<Functor.Avalonia.AvaloniaApp>()

    override _.CustomizeAppBuilder(builder) =
        base.CustomizeAppBuilder(builder).WithInterFont()
