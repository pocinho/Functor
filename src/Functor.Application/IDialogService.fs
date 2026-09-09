namespace Functor.Application

/// Platform-neutral file dialog operations required by the application layer.
type IDialogService =
    abstract OpenFile: unit -> Async<string option>
    abstract SaveFile: suggestedName: string option -> Async<string option>
