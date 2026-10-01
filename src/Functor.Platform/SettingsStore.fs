namespace Functor.Platform

open Functor.Application

type SettingsStore() =
    interface ISettingsStore with
        member _.Load() =
            match Settings.tryReadText Settings.themeFilePath with
            | Ok text -> AppSettingsLoader.loadText text
            | Error error -> Error error

        member _.Save(settings) =
            Settings.tryWriteText Settings.themeFilePath (AppSettingsLoader.toJson settings)
