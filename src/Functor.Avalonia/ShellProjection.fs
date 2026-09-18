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
      HasActiveDocument: bool
      ActiveDocumentId: DocumentId option
      NotebookIsOpen: bool
      AgentIsOpen: bool
      Status: EditorStatus
      Scroll: ShellScrollPresentation }

module ShellProjection =
    let fromState (state: AppSessionState) (editor: EditorControl) =
        { Tabs = WorkspaceProjection.tabs state.Workspace
          HasActiveDocument = state.Model.ActiveDocument.IsSome
          ActiveDocumentId = state.Workspace.ActiveDocumentId
          NotebookIsOpen =
            state.Workspace
            |> WorkspaceModel.activeDocument
            |> Option.map (fun document -> document.Auxiliary.Notebook.IsOpen)
            |> Option.defaultValue false
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

    let fromEditor (editor: EditorControl) = fromState editor.SessionState editor
