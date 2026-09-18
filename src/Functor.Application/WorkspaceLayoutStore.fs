namespace Functor.Application

open System.IO
open Functor.Workspace

type WorkspaceLayoutStore(fileService: IFileService) =
    member _.PathForWorkspace(rootPath: string option) =
        rootPath
        |> Option.map (fun root -> Path.Combine(root, ".functor", "layout.json"))

    member this.Load(rootPath: string option) =
        async {
            match this.PathForWorkspace(rootPath) with
            | None -> return Error "Cannot load workspace layout without a workspace root."
            | Some path ->
                let! result = fileService.ReadText path
                return result |> Result.bind WorkspaceLayout.ofJson
        }

    member this.Save(rootPath: string option, layout: WorkspaceLayout) =
        async {
            match this.PathForWorkspace(rootPath) with
            | None -> return Error "Cannot save workspace layout without a workspace root."
            | Some path ->
                return! fileService.WriteText(path, WorkspaceLayout.document layout |> WorkspaceLayout.toJson)
        }
