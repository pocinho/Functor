namespace Functor.Application

/// Platform-neutral text file operations required by the application layer.
type IFileService =
    abstract ReadText: path: string -> Async<Result<string, string>>
    abstract WriteText: path: string * contents: string -> Async<Result<unit, string>>
