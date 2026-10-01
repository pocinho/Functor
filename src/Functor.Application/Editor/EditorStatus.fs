namespace Functor.Application

open Functor.Domain.Core

/// Presentation-ready editor information for status bars and other shell UI.
type EditorStatus =
    { Line: int
      Column: int
      FileName: string
      FileType: string
      IsDirty: bool
      Message: string option
      Error: string option }

module EditorStatus =
    let fromState (state: AppSessionState) =
        let model = state.Model
        let cursor = model.Editing.Cursor
        let document = model.ActiveDocument

        { Line = cursor.Line + 1
          Column = cursor.Column + 1
          FileName = document |> Option.map (fun item -> item.Metadata.Name) |> Option.defaultValue "untitled"
          FileType = document |> Option.bind (fun item -> item.Metadata.Path) |> FileType.fromPath
          IsDirty = model.Editing.IsDirty
          Message = state.Status.Message
          Error = state.Status.Error }
