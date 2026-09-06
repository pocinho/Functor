namespace Functor.App.Android

open Android.App
open Android.Content.PM
open Avalonia
open Avalonia.Android
open Functor.App

    [<Application>]
type Application(javaReference: nativeint, transfer: Android.Runtime.JniHandleOwnership) = 
    inherit AvaloniaAndroidApplication<App>(javaReference, transfer)

     override _.CustomizeAppBuilder(builder) =
        base.CustomizeAppBuilder(builder)
            .WithInterFont()
