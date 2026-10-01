namespace Functor.Application

open System.Threading

/// Platform-neutral text file operations required by the application layer.
type IFileService =
    abstract ReadText: path: string * cancellationToken: CancellationToken -> Async<Result<string, string>>
    abstract EnumerateFiles: rootPath: string * cancellationToken: CancellationToken -> Async<Result<string list, string>>
    abstract WriteText: path: string * contents: string * cancellationToken: CancellationToken -> Async<Result<unit, string>>
