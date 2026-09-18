namespace Functor.Tests.Avalonia

open Functor.Application
open Functor.Avalonia
open Xunit

type ShellModelTests() =
    [<Fact>]
    member _.``side panel width is clamped to layout bounds``() =
        let tooSmall, _ = ShellUpdate.update (SetSidePanelWidth 1.0) ShellModel.initial
        let tooLarge, _ = ShellUpdate.update (SetSidePanelWidth 1000.0) ShellModel.initial

        Assert.Equal(ShellLayoutState.MinimumSidePanelWidth, tooSmall.Layout.SidePanelWidth)
        Assert.Equal(ShellLayoutState.MaximumSidePanelWidth, tooLarge.Layout.SidePanelWidth)

    [<Fact>]
    member _.``tool selection opens, switches, and toggles the shared panel``() =
        let workspace, _ = ShellUpdate.update (ToggleTool WorkspaceTool) ShellModel.initial
        let search, _ = ShellUpdate.update (ToggleTool SearchTool) workspace
        let closed, _ = ShellUpdate.update (ToggleTool SearchTool) search

        Assert.Equal(Some WorkspaceTool, workspace.Layout.ActiveTool)
        Assert.True(workspace.Layout.IsToolPanelOpen)
        Assert.Equal(Some SearchTool, search.Layout.ActiveTool)
        Assert.True(search.Layout.IsToolPanelOpen)
        Assert.Equal(Some SearchTool, closed.Layout.ActiveTool)
        Assert.False(closed.Layout.IsToolPanelOpen)

    [<Fact>]
    member _.``tool descriptors provide stable metadata for every tool``() =
        let descriptors = ToolDescriptor.all
        let ids = descriptors |> List.map (fun descriptor -> descriptor.Id)

        Assert.Equal(2, descriptors.Length)
        Assert.True(([ "workspace"; "search" ] = ids))
        Assert.Equal("Workspace", (ToolDescriptor.get WorkspaceTool).Title)

    [<Fact>]
    member _.``search has an explicit empty panel state``() =
        Assert.Equal(ToolPanelEmpty "Search is not available yet.", ToolPanelState.forTool SearchTool)

    [<Fact>]
    member _.``tool switching preserves the shared panel width``() =
        let resized, _ = ShellUpdate.update (SetSidePanelWidth 480.0) ShellModel.initial
        let selected, _ = ShellUpdate.update (ToggleTool SearchTool) resized

        Assert.Equal(480.0, selected.Layout.SidePanelWidth)

    [<Fact>]
    member _.``animation state is controlled by messages``() =
        let animating, _ = ShellUpdate.update BeginPanelAnimation ShellModel.initial
        let settled, _ = ShellUpdate.update EndPanelAnimation animating

        Assert.True(animating.Layout.IsPanelAnimating)
        Assert.False(settled.Layout.IsPanelAnimating)

    [<Fact>]
    member _.``host commands become effects without changing layout``() =
        let updated, effects = ShellUpdate.update OpenFileRequested ShellModel.initial
        let expected: ShellEffect list = [ DispatchAppCommand AppCommand.openFile ]

        Assert.Equal(ShellModel.initial, updated)
        Assert.True((expected = effects), "effects mismatch")
