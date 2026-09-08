namespace Functor.Domain.Core

open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Domain.Navigation
open Functor.Domain.Diagnostics

/// Represents the current editor mode.
type EditorMode =
    | Normal
    | Insert
    | Visual

/// Represents the visible area of the editor.
type Viewport = { Width: int; Height: int }

/// Represents the complete state of the editor.
type CoreModel =
    { ActiveDocument: DocumentModel option
      OpenDocuments: DocumentModel list

      Editing: EditingModel
      Syntax: SyntaxModel
      Navigation: NavigationModel
      Diagnostics: DiagnosticsModel

      Mode: EditorMode

      Viewport: Viewport

      VerticalOffset: int
      HorizontalOffset: int }

module CoreModel =

    let create () =
        { ActiveDocument = None
          OpenDocuments = []

          Editing = EditingModel.create ()
          Syntax = SyntaxModel.create ()
          Navigation = NavigationModel.create ()
          Diagnostics = DiagnosticsModel.create ()

          Mode = EditorMode.Normal

          Viewport = { Width = 0; Height = 0 }

          VerticalOffset = 0
          HorizontalOffset = 0 }

    let empty = create ()
