namespace Functor.Tests.Avalonia

open Functor.Application
open Functor.Avalonia
open Xunit

type ShellModelTests() =
    [<Fact>]
    member _.``shell updates preserve immutable previous state``() =
        let initial = ShellModel.initial
        let updated, _ = ShellUpdate.update (SetSidePanelOpen true) initial

        Assert.False(initial.Layout.IsSidePanelOpen)
        Assert.True(updated.Layout.IsSidePanelOpen)

    [<Fact>]
    member _.``side panel width is clamped to layout bounds``() =
        let tooSmall, _ = ShellUpdate.update (SetSidePanelWidth 1.0) ShellModel.initial
        let tooLarge, _ = ShellUpdate.update (SetSidePanelWidth 1000.0) ShellModel.initial

        Assert.Equal(ShellLayoutState.MinimumSidePanelWidth, tooSmall.Layout.SidePanelWidth)
        Assert.Equal(ShellLayoutState.MaximumSidePanelWidth, tooLarge.Layout.SidePanelWidth)

    [<Fact>]
    member _.``selecting a panel opens the side panel``() =
        let updated, _ = ShellUpdate.update (SelectPanel(Some Agent)) ShellModel.initial

        Assert.Equal(Some Agent, updated.Layout.ActivePanel)
        Assert.True(updated.Layout.IsSidePanelOpen)

    [<Fact>]
    member _.``clearing the panel selection closes the side panel``() =
        let model, _ = ShellUpdate.update (SelectPanel(Some Notebook)) ShellModel.initial
        let updated, _ = ShellUpdate.update (SelectPanel None) model

        Assert.Equal(None, updated.Layout.ActivePanel)
        Assert.False(updated.Layout.IsSidePanelOpen)

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
