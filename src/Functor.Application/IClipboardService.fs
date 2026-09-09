namespace Functor.Application

/// Platform-neutral clipboard operations required by the application layer.
type IClipboardService =
    abstract SetText: text: string -> Async<unit>
    abstract GetText: unit -> Async<string option>
