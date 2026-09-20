namespace Functor.Tests.Avalonia

open Functor.Avalonia.Views
open Functor.Workspace
open Xunit

module WorkspaceTreeCoordinatorTests =

    [<Fact>]
    let ``stale workspace tree results are rejected after invalidation`` () =
        let workspace = WorkspaceModel.empty
        let tree = WorkspaceFileTree.loading workspace

        let coordinator = WorkspaceTreeCoordinator(WorkspaceFileTree.create, ignore, ignore)

        let generation = coordinator.StartLoad workspace
        coordinator.Invalidate()

        Assert.False(coordinator.AcceptLoaded(generation, workspace, tree))
