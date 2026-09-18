namespace Functor.Avalonia.Views

open Avalonia.Controls
open Functor.Avalonia
open Functor.Avalonia.Controls
open Functor.Domain.Document

module ShellView =
    let private toolTitle tool = (ToolDescriptor.get tool).Title

    let private applyToolPanel
        (host: SidePanelView)
        (model: ShellModel)
        (input: ShellViewInput)
        (documentActivated: DocumentId -> unit)
        (fileOpenRequested: string -> unit)
        =
        match model.Layout.ActiveTool, model.Layout.IsToolPanelOpen with
        | Some tool, true ->
            host.IsOpen <- true
            host.PanelWidth <- model.Layout.SidePanelWidth
            host.Title <- toolTitle tool

            host.PanelContent <-
                match ToolPanelState.forTool tool with
                | ToolPanelUnavailable message
                | ToolPanelLoading message
                | ToolPanelEmpty message -> TextBlock(Text = message) :> Control
                | ToolPanelReady ->
                    if tool = WorkspaceTool then
                        let workspaceView = WorkspaceDocumentView()
                        workspaceView.ApplyWorkspace(input.FileTree, input.Tabs)
                        workspaceView.DocumentActivated.Add(documentActivated)
                        workspaceView.FileOpenRequested.Add(fileOpenRequested)
                        workspaceView :> Control
                    else
                        TextBlock(Text = toolTitle tool) :> Control
        | _ ->
            host.IsOpen <- false
            host.PanelWidth <- 0.0
            host.Title <- ""
            host.PanelContent <- null

    let private applyAuxiliaryPanel (host: SidePanelView) (input: ShellViewInput) =
        if input.AgentIsOpen then
            host.IsOpen <- true
            host.PanelWidth <- ShellLayoutState.DefaultSidePanelWidth
            host.Title <- "Agent"

            let content = StackPanel(Spacing = 8.0)

            content.Children.Add(TextBlock(Text = "Agent")) |> ignore

            host.PanelContent <- content
        else
            host.IsOpen <- false
            host.PanelWidth <- 0.0
            host.Title <- ""
            host.PanelContent <- null

    let applyModel
        (view: IShellProjectionTarget)
        (sidePanelHost: SidePanelView)
        (auxiliaryPanelHost: SidePanelView)
        (model: ShellModel)
        (input: ShellViewInput)
        (documentActivated: DocumentId -> unit)
        (fileOpenRequested: string -> unit)
        =
        let nodes = ShellViewNode.describe model input
        view.ApplyShellInput input

        applyToolPanel sidePanelHost model input documentActivated fileOpenRequested
        applyAuxiliaryPanel auxiliaryPanelHost input

        nodes
