namespace Functor.Application

open System.Threading

/// Platform-neutral clipboard operations required by the application layer.
type IClipboardService =
    abstract SetText: text: string * cancellationToken: CancellationToken -> Async<unit>
    abstract GetText: cancellationToken: CancellationToken -> Async<string option>
