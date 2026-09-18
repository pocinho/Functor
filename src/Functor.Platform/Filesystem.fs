namespace Functor.Platform

open System
open System.IO
open System.Text
open Functor.Application

type FileService() =
    let utf8 = UTF8Encoding(false, true)

    interface IFileService with
        member _.ReadText(path: string) =
            async {
                try
                    let! text = File.ReadAllTextAsync(path, utf8) |> Async.AwaitTask
                    return Ok text
                with ex ->
                    return Error ex.Message
            }

        member _.WriteText(path: string, contents: string) =
            async {
                try
                    let directory = Path.GetDirectoryName path

                    if not (String.IsNullOrWhiteSpace directory) then
                        Directory.CreateDirectory(directory) |> ignore

                    do! File.WriteAllTextAsync(path, contents, utf8) |> Async.AwaitTask
                    return Ok()
                with ex ->
                    return Error ex.Message
            }
