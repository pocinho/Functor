namespace Functor.Avalonia.Views

open Avalonia.Controls
open Avalonia.Markup.Xaml
open Functor.Avalonia
open Functor.Application
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
        (commandRequested: AppCommand -> unit)
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
                            match host.CachedWorkspaceContent with
                            | Some(:? WorkspaceDocumentControl as existing) -> existing, false
                            | _ ->
                                let created = WorkspaceDocumentControl()
                                host.CachedWorkspaceContent <- Some(created :> Control)
                                created, true

                        workspaceView.ApplyWorkspace(input.FileTree, input.Tabs)

                        if isNewView then
                            workspaceView.DocumentActivated.Add(documentActivated)
                            workspaceView.FileOpenRequested.Add(fileOpenRequested)

                        workspaceView :> Control
                    elif tool = SearchTool then
                        let searchView, isNewView =
                            match host.CachedSearchContent with
                            | Some(:? SearchPanelControl as existing) -> existing, false
                            | _ ->
                                let created = SearchPanelControl()
                                host.CachedSearchContent <- Some(created :> Control)
                                created, true

                        searchView.ApplySearch(input.Search)

                        if isNewView then
                            searchView.CommandRequested.Add(commandRequested)

                        searchView :> Control
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
        (commandRequested: AppCommand -> unit)
        =
        let nodes = ShellViewNode.describe model input
        view.ApplyShellInput input

        applyToolPanel sidePanelHost model input documentActivated fileOpenRequested commandRequested
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
        (commandRequested: AppCommand -> unit)
        =
        this.ApplyModelTo(
            this :> IShellProjectionTarget,
            sidePanelHost,
            auxiliaryPanelHost,
            model,
            input,
            documentActivated,
            fileOpenRequested,
            commandRequested
        )

    static member applyModel
        (view: IShellProjectionTarget)
        (sidePanelHost: SidePanelControl)
        (auxiliaryPanelHost: SidePanelControl)
        (model: ShellModel)
        (input: ShellViewInput)
        (documentActivated: DocumentId -> unit)
        (fileOpenRequested: string -> unit)
        (commandRequested: AppCommand -> unit)
        =
        let shellView = ShellView()

        shellView.ApplyModelTo(
            view,
            sidePanelHost,
            auxiliaryPanelHost,
            model,
            input,
            documentActivated,
            fileOpenRequested,
            commandRequested
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
            fileOpenRequested: string -> unit,
            commandRequested: AppCommand -> unit
        ) =
        applyModelCore
            view
            sidePanelHost
            auxiliaryPanelHost
            model
            input
            documentActivated
            fileOpenRequested
            commandRequested

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
