namespace Functor.Avalonia.ViewModels

open Functor.Application

type ShellViewModel() =
    inherit ViewModelBase()

    let mutable isCommandPaletteOpen = false
    let mutable appSettings = AppSettings.defaults

    member this.IsCommandPaletteOpen
        with get () = isCommandPaletteOpen
        and set value = this.SetProperty(&isCommandPaletteOpen, value) |> ignore

    member this.AppSettings
        with get () = appSettings
        and set value = this.SetProperty(&appSettings, value) |> ignore