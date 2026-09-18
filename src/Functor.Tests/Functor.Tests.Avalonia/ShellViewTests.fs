namespace Functor.Tests.Avalonia

open Avalonia.Controls
open Avalonia.Headless.XUnit
open Functor.Avalonia
open Functor.Avalonia.Controls
open Functor.Avalonia.Views
open Xunit

type ShellViewTests() =
    [<AvaloniaFact>]
    member _.``applying a shell model preserves the retained editor control``() =
        let view = ShellHostView()
        let window = Window(Content = view)
        window.Show()
        let editor = view.Editor
        let input = ShellProjection.fromEditor editor
        let sidePanelHost = SidePanelView()
        let auxiliaryPanelHost = SidePanelView()

        ShellView.applyModel view sidePanelHost auxiliaryPanelHost ShellModel.initial input ignore |> ignore

        Assert.Same(editor, view.Editor)

        window.Close()

    [<AvaloniaFact>]
    member _.``applying auxiliary projection updates and clears stable panel controls``() =
        let view = ShellHostView()
        let window = Window(Content = view)
        window.Show()
        let sidePanelHost = SidePanelView()
        let auxiliaryPanelHost = SidePanelView()
        let input = ShellProjection.fromEditor view.Editor

        let panelInput =
            { input with
                AgentIsOpen = true }

        ShellView.applyModel view sidePanelHost auxiliaryPanelHost ShellModel.initial panelInput ignore |> ignore

        Assert.True(auxiliaryPanelHost.IsOpen)
        Assert.Equal(ShellLayoutState.DefaultSidePanelWidth, auxiliaryPanelHost.PanelWidth)
        Assert.Equal("Agent", auxiliaryPanelHost.Title)

        ShellView.applyModel view sidePanelHost auxiliaryPanelHost ShellModel.initial input ignore |> ignore

        Assert.False(auxiliaryPanelHost.IsOpen)
        Assert.Equal(0.0, auxiliaryPanelHost.PanelWidth)
        Assert.Equal("", auxiliaryPanelHost.Title)
        Assert.Null(auxiliaryPanelHost.PanelContent)

        window.Close()
