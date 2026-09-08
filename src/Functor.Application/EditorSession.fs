namespace Functor.Application

open Functor.Domain.Core

type EditorSession(initialModel: CoreModel) =
    let mutable model = initialModel
    let stateChanged = Event<CoreModel>()

    new() = EditorSession(CoreModel.empty)

    member _.Model = model

    member _.StateChanged = stateChanged.Publish

    member _.Dispatch(event: CoreEvent) =
        model <- CoreLogic.update event model
        stateChanged.Trigger(model)
