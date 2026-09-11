namespace Functor.Application

open Functor.Domain.Document

type PendingAction =
    | NewDocument
    | OpenFile
    | OpenWorkspace of path: string
    | CloseDocument of DocumentId

/// Minimal session status used to report application-level feedback.
type SessionStatus =
    { Message: string option
      Error: string option
      PendingAction: PendingAction option }

module SessionStatus =
    let empty =
        { Message = None
          Error = None
          PendingAction = None }
