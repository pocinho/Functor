namespace Functor.Application

open System.Threading

/// Platform-neutral file dialog operations required by the application layer.
type IDialogService =
    abstract OpenFile: cancellationToken: CancellationToken -> Async<string option>
    abstract OpenFolder: cancellationToken: CancellationToken -> Async<string option>
    abstract SaveFile: suggestedName: string option * cancellationToken: CancellationToken -> Async<string option>
