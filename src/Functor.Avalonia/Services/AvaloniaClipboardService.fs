namespace Functor.Avalonia.Services

open Avalonia.Controls
open Avalonia.Input.Platform
open System.Threading
open Functor.Application

/// Avalonia adapter for the application clipboard contract.
type AvaloniaClipboardService(getTopLevel: unit -> TopLevel option) =
    interface IClipboardService with
        member _.SetText(text: string, cancellationToken: CancellationToken) =
            async {
                cancellationToken.ThrowIfCancellationRequested()

                match getTopLevel() with
                | Some topLevel ->
                    do! topLevel.Clipboard.SetTextAsync(text) |> Async.AwaitTask
                    cancellationToken.ThrowIfCancellationRequested()
                | None ->
                    ()
            }

        member _.GetText(cancellationToken: CancellationToken) =
            async {
                cancellationToken.ThrowIfCancellationRequested()

                match getTopLevel() with
                | Some topLevel ->
                    let! text = topLevel.Clipboard.TryGetTextAsync() |> Async.AwaitTask
                    cancellationToken.ThrowIfCancellationRequested()
                    return
                        if isNull text || System.String.IsNullOrEmpty(text) then
                            None
                        else
                            Some text
                | None ->
                    return None
            }
