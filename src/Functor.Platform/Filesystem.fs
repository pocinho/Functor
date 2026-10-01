namespace Functor.Platform

open System
open System.IO
open System.Text
open System.Threading
open Functor.Application

type FileService() =
    let utf8 = UTF8Encoding(false, true)

    interface IFileService with
        member _.ReadText(path: string, cancellationToken: CancellationToken) =
            async {
                try
                    let! text = File.ReadAllTextAsync(path, utf8, cancellationToken) |> Async.AwaitTask
                    return Ok text
                with ex ->
                    return Error ex.Message
            }

        member _.EnumerateFiles(rootPath: string, cancellationToken: CancellationToken) =
            async {
                try
                    let paths = ResizeArray<string>()

                    for path in Directory.EnumerateFiles(rootPath, "*", SearchOption.AllDirectories) do
                        cancellationToken.ThrowIfCancellationRequested()
                        paths.Add(path)

                    return List.ofSeq paths |> Ok
                with ex ->
                    return Error ex.Message
            }

        member _.WriteText(path: string, contents: string, cancellationToken: CancellationToken) =
            async {
                try
                    cancellationToken.ThrowIfCancellationRequested()
                    let directory = Path.GetDirectoryName path

                    if not (String.IsNullOrWhiteSpace directory) then
                        Directory.CreateDirectory(directory) |> ignore

                    do! File.WriteAllTextAsync(path, contents, utf8, cancellationToken) |> Async.AwaitTask
                    return Ok()
                with ex ->
                    return Error ex.Message
            }
