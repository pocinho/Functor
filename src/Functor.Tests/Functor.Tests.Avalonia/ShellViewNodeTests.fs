namespace Functor.Tests.Avalonia

open Functor.Application
open Functor.Avalonia
open Xunit

type ShellViewNodeTests() =
    [<Fact>]
    member _.``view description reflects layout and projected editor visibility``() =
        let input =
            { Tabs = []
              HasActiveDocument = false
              Status =
                { Line = 1
                  Column = 1
                  FileName = "untitled"
                  FileType = "Plain Text"
                  IsDirty = false
                  Message = None
                  Error = None }
              Scroll =
                { VerticalMaximum = 0.0
                  VerticalViewport = 1.0
                  VerticalOffset = 0.0
                  HorizontalMaximum = 0.0
                  HorizontalViewport = 1.0
                  HorizontalOffset = 0.0 } }

        let model, _ = ShellUpdate.update (SelectPanel(Some Agent)) ShellModel.initial
        let nodes = ShellViewNode.describe model input

        Assert.Contains(EditorNode false, nodes)
        Assert.Contains(WelcomeNode true, nodes)
        Assert.Contains(SidePanelNode(true, ShellLayoutState.DefaultSidePanelWidth, Some Agent), nodes)
