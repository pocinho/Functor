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
              ActiveDocumentId = None
              NotebookIsOpen = false
              AgentIsOpen = false
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

        let nodes = ShellViewNode.describe ShellModel.initial input

        Assert.Contains(EditorNode false, nodes)
        Assert.Contains(WelcomeNode true, nodes)
        Assert.Contains(AuxiliaryNode(false, false), nodes)
