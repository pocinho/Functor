namespace Functor.Avalonia.Views

open Avalonia.Controls
open Functor.Application
open Functor.Avalonia
open Functor.Platform

type SettingsCoordinator(
    settingsView: SettingsView,
    settingsTabButton: Button,
    settingsDocument: Grid,
    hasActiveDocument: unit -> bool,
    updateEmptyState: bool -> unit,
    setActive: bool -> unit,
    focusEditor: unit -> unit
) =
    let mutable isOpen = false
    let mutable isActive = false
    let mutable currentSettings = AppSettings.defaults
    let mutable applySettings: AppSettings -> unit = ignore
    let mutable saveSettings: AppSettings -> Result<unit, string> = fun _ -> Ok()

    member _.CurrentSettings = currentSettings
    member _.IsActive = isActive

    member _.Configure(settings, applyCallback, saveCallback) =
        currentSettings <- settings
        applySettings <- applyCallback
        saveSettings <- saveCallback
        settingsView.Configure(settings, ThemeCatalog.load ())

    member _.Open() =
        isOpen <- true
        isActive <- true
        setActive true
        settingsTabButton.IsVisible <- true
        updateEmptyState (hasActiveDocument ())

    member _.Close() =
        isOpen <- false
        isActive <- false
        setActive false
        settingsDocument.IsVisible <- false
        settingsTabButton.IsVisible <- false
        updateEmptyState (hasActiveDocument ())
        focusEditor ()

    member _.CloseForDocumentNavigation() =
        if isActive then
            isActive <- false
            setActive false
            settingsDocument.IsVisible <- false
            updateEmptyState (hasActiveDocument ())

    member _.TryApply(save: bool) =
        match settingsView.Draft with
        | None -> settingsView.SetError("Settings draft is not initialized.")
        | Some draft ->
            match SettingsDraft.tryCreateSettings draft with
            | Error error -> settingsView.SetError(error)
            | Ok settings ->
                let result =
                    if save then
                        saveSettings settings
                    else
                        Ok(applySettings settings)

                match result with
                | Ok() -> settingsView.SetError("")
                | Error error -> settingsView.SetError(error)

    member _.TryExportTheme() =
        match settingsView.Draft with
        | None -> settingsView.SetError("Theme draft is not initialized.")
        | Some draft ->
            match SettingsDraft.tryCreateSettings draft with
            | Error error -> settingsView.SetError(error)
            | Ok settings ->
                match ThemeCatalog.export settingsView.ThemeName settings with
                | Ok() ->
                    settingsView.UpdateThemes(ThemeCatalog.load ())
                    settingsView.SetError("")
                | Error error -> settingsView.SetError(error)