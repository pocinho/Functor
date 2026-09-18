namespace Functor.Avalonia

open Functor.Application
open Functor.Workspace

type ShellViewNode =
    | EditorNode of isVisible: bool
    | WelcomeNode of isVisible: bool
    | TabsNode of tabs: WorkspaceTabProjection list
    | StatusNode of status: EditorStatus
    | ScrollNode of scroll: ShellScrollPresentation
    | SidePanelNode of isOpen: bool * width: double * panel: ShellPanel option

module ShellViewNode =
    let describe (model: ShellModel) (input: ShellViewInput) =
        [ TabsNode input.Tabs
          SidePanelNode(model.Layout.IsSidePanelOpen, model.Layout.SidePanelWidth, model.Layout.ActivePanel)
          EditorNode input.HasActiveDocument
          WelcomeNode(not input.HasActiveDocument)
          StatusNode input.Status
          ScrollNode input.Scroll ]
