namespace Functor.Avalonia.Views

open Avalonia.Controls
open Functor.Avalonia
open Functor.Avalonia.Controls

module ShellView =
    let private applyAuxiliary (host: SidePanelView) (model: ShellModel) (input: ShellViewInput) =
        if input.NotebookIsOpen || input.AgentIsOpen then
            let content = StackPanel(Spacing = 8.0)

            if input.NotebookIsOpen then
                content.Children.Add(TextBlock(Text = "Notebook")) |> ignore

            if input.AgentIsOpen then
                content.Children.Add(TextBlock(Text = "Agent")) |> ignore

            host.IsOpen <- true
            host.PanelWidth <- model.Layout.SidePanelWidth
            host.Title <- "Workspace"
            host.PanelContent <- content
        else
            host.IsOpen <- false
            host.PanelWidth <- 0.0
            host.Title <- ""
            host.PanelContent <- null

    let applyModel
        (view: IShellProjectionTarget)
        (sidePanelHost: SidePanelView)
        (model: ShellModel)
        (input: ShellViewInput)
        =
        let nodes = ShellViewNode.describe model input
        view.ApplyShellInput input

        applyAuxiliary sidePanelHost model input

        nodes
