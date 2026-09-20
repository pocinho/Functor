namespace Functor.Avalonia.Views

open System
open System.Threading.Tasks
open Functor.Workspace

type WorkspaceTreeCoordinator
    (createTree: WorkspaceModel -> WorkspaceFileTreeNode, dispatch: (unit -> unit) -> unit, requestRefresh: unit -> unit)
    =
    let mutable cachedWorkspace: WorkspaceModel option = None
    let mutable cachedWorkspaceTree: WorkspaceFileTreeNode option = None
    let mutable pendingWorkspaceTree: WorkspaceModel option = None
    let mutable generation = 0L

    let acceptLoaded loadGeneration workspace tree =
        if loadGeneration = generation then
            pendingWorkspaceTree <- None
            cachedWorkspace <- Some workspace
            cachedWorkspaceTree <- Some tree
            true
        else
            false

    let startLoad (workspace: WorkspaceModel) =
        generation <- generation + 1L
        let loadGeneration = generation
        pendingWorkspaceTree <- Some workspace

        Task
            .Run(fun () -> createTree workspace)
            .ContinueWith(fun (completed: Task<WorkspaceFileTreeNode>) ->
                if completed.Status = TaskStatus.RanToCompletion then
                    dispatch (fun () ->
                        if acceptLoaded loadGeneration workspace completed.Result then
                            requestRefresh ()))
        |> ignore

        loadGeneration

    member _.GetTree(workspace: WorkspaceModel) =
        let treeKey = ShellHostProjection.workspaceTreeKey workspace

        match cachedWorkspace with
        | Some previous when ShellHostProjection.workspaceTreeKey previous = treeKey -> cachedWorkspaceTree.Value
        | _ when
            pendingWorkspaceTree
            |> Option.exists (fun pending -> ShellHostProjection.workspaceTreeKey pending = treeKey)
            ->
            WorkspaceFileTree.loading workspace
        | _ ->
            if ShellHostProjection.hasDiskWorkspace workspace then
                startLoad workspace |> ignore
                WorkspaceFileTree.loading workspace
            else
                generation <- generation + 1L
                pendingWorkspaceTree <- None
                let tree = createTree workspace
                cachedWorkspace <- Some workspace
                cachedWorkspaceTree <- Some tree
                tree

    member _.Invalidate() =
        generation <- generation + 1L
        pendingWorkspaceTree <- None
        cachedWorkspace <- None
        cachedWorkspaceTree <- None

    member _.Dispose() =
        generation <- generation + 1L
        pendingWorkspaceTree <- None

    member _.StartLoad(workspace: WorkspaceModel) = startLoad workspace

    member _.AcceptLoaded(loadGeneration, workspace, tree) =
        acceptLoaded loadGeneration workspace tree
