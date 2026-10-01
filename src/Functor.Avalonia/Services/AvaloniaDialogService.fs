namespace Functor.Avalonia.Services

open Avalonia.Controls
open Avalonia.Platform.Storage
open System.Threading
open Functor.Application

/// Avalonia adapter for the application file-dialog contract.
type AvaloniaDialogService(getTopLevel: unit -> TopLevel option) =
    interface IDialogService with
        member _.OpenFile(cancellationToken: CancellationToken) =
            async {
                cancellationToken.ThrowIfCancellationRequested()

                match getTopLevel() with
                | Some topLevel ->
                    let options = FilePickerOpenOptions()
                    options.Title <- "Open file"
                    options.AllowMultiple <- false

                    let! files = topLevel.StorageProvider.OpenFilePickerAsync(options) |> Async.AwaitTask
                    cancellationToken.ThrowIfCancellationRequested()

                    return
                        files
                        |> Seq.tryHead
                        |> Option.map (fun file -> file.Path.LocalPath)
                | None ->
                    return None
            }

        member _.OpenFolder(cancellationToken: CancellationToken) =
            async {
                cancellationToken.ThrowIfCancellationRequested()

                match getTopLevel() with
                | Some topLevel ->
                    let options = FolderPickerOpenOptions()
                    options.Title <- "Open folder"

                    let! folders = topLevel.StorageProvider.OpenFolderPickerAsync(options) |> Async.AwaitTask
                    cancellationToken.ThrowIfCancellationRequested()

                    return
                        folders
                        |> Seq.tryHead
                        |> Option.map (fun folder -> folder.Path.LocalPath)
                | None ->
                    return None
            }

        member _.SaveFile(suggestedName: string option, cancellationToken: CancellationToken) =
            async {
                cancellationToken.ThrowIfCancellationRequested()

                match getTopLevel() with
                | Some topLevel ->
                    let options = FilePickerSaveOptions()
                    options.Title <- "Save file"
                    options.SuggestedFileName <- suggestedName |> Option.defaultValue "untitled"

                    let! file = topLevel.StorageProvider.SaveFilePickerAsync(options) |> Async.AwaitTask
                    cancellationToken.ThrowIfCancellationRequested()
                    return file |> Option.ofObj |> Option.map (fun item -> item.Path.LocalPath)
                | None ->
                    return None
            }
