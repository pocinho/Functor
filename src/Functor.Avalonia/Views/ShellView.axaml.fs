namespace Functor.Avalonia.Views

open Avalonia.Controls
open Avalonia.Markup.Xaml
open Functor.Avalonia
open Functor.Avalonia.Controls
open Functor.Domain.Document

type ShellView() as this =
    inherit UserControl()

    let toolTitle tool = (ToolDescriptor.get tool).Title

    let applyToolPanel
        (host: SidePanelControl)
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
                        let workspaceView, isNewView =
                            match host.PanelContent with
                            | :? WorkspaceDocumentControl as existing -> existing, false
                            | _ -> WorkspaceDocumentControl(), true

                        workspaceView.ApplyWorkspace(input.FileTree, input.Tabs)

                        if isNewView then
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

    let applyAuxiliaryPanel (host: SidePanelControl) (input: ShellViewInput) =
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

    let applyModelCore
        (view: IShellProjectionTarget)
        (sidePanelHost: SidePanelControl)
        (auxiliaryPanelHost: SidePanelControl)
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

    do this.InitializeComponent()

    member this.ApplyModel
        (model: ShellModel)
        (input: ShellViewInput)
        (sidePanelHost: SidePanelControl)
        (auxiliaryPanelHost: SidePanelControl)
        (documentActivated: DocumentId -> unit)
        (fileOpenRequested: string -> unit)
        =
        this.ApplyModelTo(
            this :> IShellProjectionTarget,
            sidePanelHost,
            auxiliaryPanelHost,
            model,
            input,
            documentActivated,
            fileOpenRequested
        )

    static member applyModel
        (view: IShellProjectionTarget)
        (sidePanelHost: SidePanelControl)
        (auxiliaryPanelHost: SidePanelControl)
        (model: ShellModel)
        (input: ShellViewInput)
        (documentActivated: DocumentId -> unit)
        (fileOpenRequested: string -> unit)
        =
        let shellView = ShellView()

        shellView.ApplyModelTo(
            view,
            sidePanelHost,
            auxiliaryPanelHost,
            model,
            input,
            documentActivated,
            fileOpenRequested
        )

    interface IShellProjectionTarget with
        member _.ApplyShellInput(_) = ()

    member private _.ApplyModelTo
        (
            view: IShellProjectionTarget,
            sidePanelHost: SidePanelControl,
            auxiliaryPanelHost: SidePanelControl,
            model: ShellModel,
            input: ShellViewInput,
            documentActivated: DocumentId -> unit,
            fileOpenRequested: string -> unit
        ) =
        applyModelCore view sidePanelHost auxiliaryPanelHost model input documentActivated fileOpenRequested

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
