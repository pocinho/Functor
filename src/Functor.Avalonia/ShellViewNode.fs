namespace Functor.Avalonia

open Functor.Application
open Functor.Workspace

type ShellViewNode =
    | EditorNode of isVisible: bool
    | WelcomeNode of isVisible: bool
    | TabsNode of tabs: WorkspaceTabProjection list
    | TabToolbarNode of hasActiveDocument: bool * notebookIsOpen: bool * agentIsOpen: bool
    | AuxiliaryNode of notebookIsOpen: bool * agentIsOpen: bool
    | StatusNode of status: EditorStatus
    | ScrollNode of scroll: ShellScrollPresentation

module ShellViewNode =
    let describe (model: ShellModel) (input: ShellViewInput) =
        [ TabsNode(input.Tabs)
          TabToolbarNode(input.HasActiveDocument, input.NotebookIsOpen, input.AgentIsOpen)
          AuxiliaryNode(input.NotebookIsOpen, input.AgentIsOpen)
          EditorNode(input.HasActiveDocument)
          WelcomeNode(not input.HasActiveDocument)
          StatusNode(input.Status)
          ScrollNode(input.Scroll) ]
