namespace Functor.Avalonia

open Functor.Application
open Functor.Avalonia.Controls
open Functor.Domain.Document
open Functor.Workspace

type ShellScrollPresentation =
    { VerticalMaximum: float
      VerticalViewport: float
      VerticalOffset: float
      HorizontalMaximum: float
      HorizontalViewport: float
      HorizontalOffset: float }

type ShellViewInput =
    { Tabs: WorkspaceTabProjection list
      FileTree: WorkspaceFileTreeNode
      HasActiveDocument: bool
      ActiveDocumentId: DocumentId option
      AgentIsOpen: bool
      Status: EditorStatus
      Scroll: ShellScrollPresentation }

module ShellProjection =
    let fromStateWithWorkspace
        (state: AppSessionState)
        (editor: EditorControl)
        (tabs: WorkspaceTabProjection list)
        (fileTree: WorkspaceFileTreeNode)
        =
        { Tabs = tabs
          FileTree = fileTree
          HasActiveDocument = state.Model.ActiveDocument.IsSome
          ActiveDocumentId = state.Workspace.ActiveDocumentId
          AgentIsOpen =
            state.Workspace
            |> WorkspaceModel.activeDocument
            |> Option.map (fun document -> document.Auxiliary.Agent.IsOpen)
            |> Option.defaultValue false
          Status = editor.EditorStatus
          Scroll =
            { VerticalMaximum = float editor.VerticalScrollMaximum
              VerticalViewport = float editor.VerticalScrollViewport
              VerticalOffset = float editor.VerticalOffset
              HorizontalMaximum = float editor.HorizontalScrollMaximum
              HorizontalViewport = editor.HorizontalScrollViewport
              HorizontalOffset = float editor.HorizontalOffset } }

    let fromState (state: AppSessionState) (editor: EditorControl) =
        fromStateWithWorkspace
            state
            editor
            (WorkspaceProjection.tabs state.Workspace)
            (WorkspaceFileTree.create state.Workspace)

    let fromEditor (editor: EditorControl) = fromState editor.SessionState editor

    let fromEditorWithWorkspace
        (editor: EditorControl)
        (tabs: WorkspaceTabProjection list)
        (fileTree: WorkspaceFileTreeNode)
        =
        fromStateWithWorkspace editor.SessionState editor tabs fileTree
