namespace Functor.Avalonia

open Functor.Application
open Functor.Workspace

type ShellViewNode =
    | EditorNode of isVisible: bool
    | WelcomeNode of isVisible: bool
    | TabsNode of tabs: WorkspaceTabProjection list
    | AuxiliaryNode of agentIsOpen: bool
    | ToolRailNode of activeTool: ToolKind option * isPanelOpen: bool
    | ToolPanelNode of tool: ToolKind option * isOpen: bool * width: double
    | StatusNode of status: EditorStatus
    | ScrollNode of scroll: ShellScrollPresentation

module ShellViewNode =
    let describe (model: ShellModel) (input: ShellViewInput) =
        [ TabsNode input.Tabs; AuxiliaryNode input.AgentIsOpen; ToolRailNode(model.Layout.ActiveTool, model.Layout.IsToolPanelOpen); ToolPanelNode(model.Layout.ActiveTool, model.Layout.IsToolPanelOpen, model.Layout.SidePanelWidth); EditorNode input.HasActiveDocument; WelcomeNode(not input.HasActiveDocument); StatusNode input.Status; ScrollNode input.Scroll ]
