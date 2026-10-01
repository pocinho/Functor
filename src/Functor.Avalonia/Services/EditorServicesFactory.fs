namespace Functor.Avalonia.Services

open Avalonia.Controls
open Functor.Application
open Functor.Syntax
open Functor.Platform

module EditorServicesFactory =
    let create (getTopLevel: unit -> TopLevel option) =
        { EditorServices =
            EditorServices.create
                (AvaloniaClipboardService getTopLevel)
                (FileService())
                (AvaloniaDialogService getTopLevel)
                (Some(DefaultTokenizerService() :> ITokenizerService))
          SettingsStore = SettingsStore() :> ISettingsStore }
