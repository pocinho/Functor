namespace Functor.Avalonia.Views

open Avalonia.Controls
open Functor.Avalonia
open Functor.Avalonia.Controls

module ShellView =
    let private panelTitle panel =
        match panel with
        | Notebook -> "Notebook"
        | Agent -> "Agent"
        | PluginPanel name -> name

    let private applySidePanel (host: SidePanelView) node =
        match node with
        | SidePanelNode(isOpen, width, Some panel) when isOpen ->
            host.IsOpen <- true
            host.PanelWidth <- width
            let name = panelTitle panel
            host.Title <- name
            host.PanelContent <- TextBlock(Text = name)
        | SidePanelNode _ ->
            host.IsOpen <- false
            host.PanelWidth <- 0.0
            host.Title <- ""
            host.PanelContent <- null
        | _ -> ()

    let applyModel
        (view: IShellProjectionTarget)
        (sidePanelHost: SidePanelView)
        (model: ShellModel)
        (input: ShellViewInput)
        =
        let nodes = ShellViewNode.describe model input
        view.ApplyShellInput input

        nodes |> List.iter (applySidePanel sidePanelHost)

        nodes
