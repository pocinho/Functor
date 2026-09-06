namespace Functor.App.Android

open Android.App
open Android.Content.PM
open Avalonia
open Avalonia.Android
open Functor.App

[<Activity(
    Label = "Functor.App.Android",
    Theme = "@style/MyTheme.NoActionBar",
    Icon = "@drawable/icon",
    MainLauncher = true,
    ConfigurationChanges = (ConfigChanges.Orientation ||| ConfigChanges.ScreenSize ||| ConfigChanges.UiMode))>]
type MainActivity() =
    inherit AvaloniaMainActivity()
