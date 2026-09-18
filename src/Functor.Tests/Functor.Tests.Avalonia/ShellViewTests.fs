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

        ShellView.applyModel view sidePanelHost ShellModel.initial input |> ignore

        Assert.Same(editor, view.Editor)

        window.Close()

    [<AvaloniaFact>]
    member _.``applying panel nodes updates and clears stable panel controls``() =
        let view = ShellHostView()
        let window = Window(Content = view)
        window.Show()
        let sidePanelHost = SidePanelView()
        let input = ShellProjection.fromEditor view.Editor

        let panelModel, _ =
            ShellUpdate.update (SelectPanel(Some(PluginPanel "Extensions"))) ShellModel.initial

        ShellView.applyModel view sidePanelHost panelModel input |> ignore

        Assert.True(sidePanelHost.IsOpen)
        Assert.Equal(ShellLayoutState.DefaultSidePanelWidth, sidePanelHost.PanelWidth)
        Assert.Equal("Extensions", sidePanelHost.Title)
        Assert.Equal("Extensions", (sidePanelHost.PanelContent :?> TextBlock).Text)

        ShellView.applyModel view sidePanelHost ShellModel.initial input |> ignore

        Assert.False(sidePanelHost.IsOpen)
        Assert.Equal(0.0, sidePanelHost.PanelWidth)
        Assert.Equal("", sidePanelHost.Title)
        Assert.Null(sidePanelHost.PanelContent)

        window.Close()
