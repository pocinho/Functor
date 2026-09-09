namespace Functor.Avalonia.Services

open Avalonia.Controls
open Avalonia.Input.Platform
open Functor.Application

/// Avalonia adapter for the application clipboard contract.
type AvaloniaClipboardService(getTopLevel: unit -> TopLevel option) =
    interface IClipboardService with
        member _.SetText(text: string) =
            async {
                match getTopLevel() with
                | Some topLevel ->
                    do! topLevel.Clipboard.SetTextAsync(text) |> Async.AwaitTask
                | None ->
                    ()
            }

        member _.GetText() =
            async {
                match getTopLevel() with
                | Some topLevel ->
                    let! text = topLevel.Clipboard.TryGetTextAsync() |> Async.AwaitTask
                    return
                        if isNull text || System.String.IsNullOrEmpty(text) then
                            None
                        else
                            Some text
                | None ->
                    return None
            }
