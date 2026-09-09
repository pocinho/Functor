namespace Functor.Application

open Functor.Domain.Core

/// Combined application state exposed by the session coordinator.
type AppSessionState =
    { Model: CoreModel
      Status: SessionStatus }

module AppSessionState =
    let empty model =
        { Model = model
          Status = SessionStatus.empty }

    let withStatus status state =
        { state with Status = status }

    let withMessage message state =
        { state with Status = { state.Status with Message = Some message } }

    let withError error state =
        { state with Status = { state.Status with Error = Some error } }
