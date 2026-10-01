namespace Functor.Tests.Application

open System
open System.Threading
open Functor.Application
open Functor.Domain.Document
open Functor.Domain.Search
open Functor.Domain.Syntax
open Functor.Workspace
open Xunit

type private InterpreterFileService(readResult: Result<string, string>, writeResult: Result<unit, string>) =
    let writes = ResizeArray<string * string>()

    member _.Writes = writes

    interface IFileService with
        member _.ReadText(_, _) = async { return readResult }

        member _.EnumerateFiles(_, _) = async { return Ok [] }

        member _.WriteText(path, contents, _) =
            async {
                writes.Add(path, contents)
                return writeResult
            }

type private WorkspaceSearchFileService() =
    let reads = ResizeArray<string>()

    member _.Reads = reads

    interface IFileService with
        member _.ReadText(path, _) =
            async {
                reads.Add(path)

                if path.EndsWith("other.fs", StringComparison.OrdinalIgnoreCase) then
                    return Ok "disk term"
                else
                    return Ok "disk term"
            }

        member _.EnumerateFiles(_, _) =
            async {
                return
                    Ok
                        [ "C:\work\open.fs"
                          "C:\work\other.fs"
                          "C:\work\project.fsproj"
                          "C:\work\ignored.bin"
                          "C:\outside.fs" ]
            }

        member _.WriteText(_, _, _) = async { return Ok() }

type private CancellationObservingFileService() =
    let mutable enumerationToken: CancellationToken option = None

    member _.EnumerationToken = enumerationToken

    interface IFileService with
        member _.ReadText(_, _) = async { return Ok "" }

        member _.EnumerateFiles(_, cancellationToken) =
            async {
                enumerationToken <- Some cancellationToken
                return Ok []
            }

        member _.WriteText(_, _, _) = async { return Ok() }

type private RecoverableWorkspaceSearchFileService() =
    interface IFileService with
        member _.ReadText(path, _) =
            async {
                if path.EndsWith("missing.fs", StringComparison.OrdinalIgnoreCase) then
                    return Error "access denied"
                else
                    return Ok "term"
            }

        member _.EnumerateFiles(_, _) =
            async { return Ok [ "C:\work\available.fs"; "C:\work\missing.fs" ] }

        member _.WriteText(_, _, _) = async { return Ok() }

type private PartialWorkspaceReplacementFileService() =
    let writes = ResizeArray<string * string>()

    member _.Writes = writes

    interface IFileService with
        member _.ReadText(path, _) =
            async {
                if path.EndsWith("failed.fs", StringComparison.OrdinalIgnoreCase) then
                    return Ok "term"
                else
                    return Ok "term"
            }

        member _.EnumerateFiles(_, _) = async { return Ok [] }

        member _.WriteText(path, contents, _) =
            async {
                writes.Add(path, contents)

                if path.EndsWith("failed.fs", StringComparison.OrdinalIgnoreCase) then
                    return Error "disk full"
                else
                    return Ok()
            }

type private InterpreterDialogService(openPath: string option, savePath: string option) =
    interface IDialogService with
        member _.OpenFile(_) = async { return openPath }

        member _.OpenFolder(_) =
            async { return Some "C:\\work\\folder" }

        member _.SaveFile(_, _) = async { return savePath }

type private InterpreterTokenizerService(tokens: LineTokens list) =
    interface ITokenizerService with
        member _.Tokenize(_, _: CancellationToken) =
            async {
                return
                    Ok
                        { Provider = LocalLexical
                          Layer = Lexical
                          Tokens = tokens
                          Snapshots = []
                          FinalState = Initial }
            }

type private FailingInterpreterTokenizerService(message: string) =
    interface ITokenizerService with
        member _.Tokenize(_, _: CancellationToken) = async { return Error message }

type private CancelingInterpreterTokenizerService() =
    interface ITokenizerService with
        member _.Tokenize(_, _: CancellationToken) =
            async { return raise (OperationCanceledException()) }
    
type private ConcurrentWriteObservingFileService() =
    let mutable activeWrites = 0
    let mutable maximumConcurrentWrites = 0

    member _.MaximumConcurrentWrites = maximumConcurrentWrites

    interface IFileService with
        member _.ReadText(_, _) = async { return Ok "" }

        member _.EnumerateFiles(_, _) = async { return Ok [] }

        member _.WriteText(_, _, _) =
            async {
                let active = Interlocked.Increment(&activeWrites)
                maximumConcurrentWrites <- max maximumConcurrentWrites active
                do! Async.Sleep 50
                Interlocked.Decrement(&activeWrites) |> ignore
                return Ok()
            }

type AppEffectInterpreterTests() =
    [<Fact>]
    member _.``workspace search prefers open buffers and filters unsupported files``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = WorkspaceSearchFileService()
        let workspaceId = Guid.NewGuid()
        let documentId = Guid.NewGuid()
        let options = SearchOptions.create "term"

        let request: WorkspaceSearchRequest =
            { RequestId = Guid.NewGuid()
              WorkspaceId = workspaceId
              RootPath = "C:\work"
              Options = options
              OpenDocuments =
                [ { Id = documentId
                    Path = Some "C:\work\open.fs"
                    Name = "open.fs"
                    Lines = [ "unsaved term" ] } ]
              OpenDocumentPaths = Set.ofList [ "C:\work\open.fs" ]
              OpenDocumentRevisions = Map.ofList [ documentId, 0L ] }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        interpreter.Execute(AppEffect.searchWorkspace request) |> Async.RunSynchronously

        match commands |> Seq.toList with
        | [ WorkspaceSearchCompleted result ] ->
            Assert.Equal(workspaceId, result.WorkspaceId)
            Assert.Equal(3, result.Matches.Length)

            Assert.Equal<string list>(
                [ "C:\work\open.fs"; "C:\work\other.fs"; "C:\work\project.fsproj" ],
                result.Documents |> List.map (fun document -> document.Path.Value)
            )

            Assert.Equal("disk term", result.Sources["C:\work\other.fs"])

            Assert.Equal<string list>(
                [ "C:\work\open.fs"; "C:\work\other.fs"; "C:\work\project.fsproj" ],
                result.Matches |> List.map (fun matchValue -> matchValue.Path.Value)
            )

            Assert.True(
                result.Matches
                |> List.exists (fun matchValue -> matchValue.Path = Some "C:\work\open.fs")
            )

            Assert.True(
                result.Matches
                |> List.exists (fun matchValue -> matchValue.Path = Some "C:\work\other.fs")
            )

            Assert.DoesNotContain(result.Matches, fun matchValue -> matchValue.Path = Some "C:\work\ignored.bin")
            Assert.DoesNotContain(result.Matches, fun matchValue -> matchValue.Path = Some "C:\outside.fs")
            Assert.DoesNotContain("C:\work\open.fs", fileService.Reads)
            Assert.Contains("C:\work\other.fs", fileService.Reads)
        | _ -> Assert.True(false, "Expected one workspace search completion.")

    [<Fact>]
    member _.``workspace search reads all candidate files in one operation``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = WorkspaceSearchFileService()

        let request: WorkspaceSearchRequest =
            { RequestId = Guid.NewGuid()
              WorkspaceId = Guid.NewGuid()
              RootPath = "C:\work"
              Options = SearchOptions.create "term"
              OpenDocuments = []
              OpenDocumentPaths = Set.empty
              OpenDocumentRevisions = Map.empty }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        interpreter.Execute(AppEffect.searchWorkspace request) |> Async.RunSynchronously

        match commands |> Seq.toList with
        | [ WorkspaceSearchCompleted result ] ->
            Assert.Equal(3, result.Matches.Length)
            Assert.Equal(3, fileService.Reads.Count)
        | _ -> Assert.True(false, "Expected one workspace search completion.")

    [<Fact>]
    member _.``workspace search honors cancellation before traversal``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = WorkspaceSearchFileService()

        let request: WorkspaceSearchRequest =
            { RequestId = Guid.NewGuid()
              WorkspaceId = Guid.NewGuid()
              RootPath = "C:\work"
              Options = SearchOptions.create "term"
              OpenDocuments = []
              OpenDocumentPaths = Set.empty
              OpenDocumentRevisions = Map.empty }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        use cancellation = new CancellationTokenSource()
        cancellation.Cancel()

        interpreter.Execute(AppEffect.searchWorkspaceWithCancellation request cancellation.Token)
        |> Async.RunSynchronously

        Assert.Empty(commands)
        Assert.Empty(fileService.Reads)

    [<Fact>]
    member _.``workspace search forwards its cancellation token to file enumeration``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = CancellationObservingFileService()
        let request: WorkspaceSearchRequest =
            { RequestId = Guid.NewGuid()
              WorkspaceId = Guid.NewGuid()
              RootPath = "C:\work"
              Options = SearchOptions.create "term"
              OpenDocuments = []
              OpenDocumentPaths = Set.empty
              OpenDocumentRevisions = Map.empty }

        use cancellation = new CancellationTokenSource()
        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        interpreter.Execute(AppEffect.searchWorkspaceWithCancellation request cancellation.Token)
        |> Async.RunSynchronously

        Assert.True(Some cancellation.Token = fileService.EnumerationToken)

    [<Fact>]
    member _.``workspace search reports read failures as recoverable errors``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = RecoverableWorkspaceSearchFileService()

        let request: WorkspaceSearchRequest =
            { RequestId = Guid.NewGuid()
              WorkspaceId = Guid.NewGuid()
              RootPath = "C:\work"
              Options = SearchOptions.create "term"
              OpenDocuments = []
              OpenDocumentPaths = Set.empty
              OpenDocumentRevisions = Map.empty }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        interpreter.Execute(AppEffect.searchWorkspace request) |> Async.RunSynchronously

        match commands |> Seq.toList with
        | [ WorkspaceSearchCompleted result ] ->
            Assert.Single(result.Errors) |> ignore
            Assert.Contains("missing.fs", result.Errors[0])
            Assert.Contains("access denied", result.Errors[0])
        | _ -> Assert.True(false, "Expected one workspace search completion.")

    [<Fact>]
    member _.``workspace replacement skips stale snapshots``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = InterpreterFileService(Ok "changed", Ok())
        let path = "C:\work\file.fs"

        let request =
            { RequestId = Guid.NewGuid()
              WorkspaceId = Guid.NewGuid()
              StalePaths = []
              Sources = Map.ofList [ path, "original" ]
              Replacements = Map.ofList [ path, "replacement" ]
              MatchCounts = Map.ofList [ path, 1 ]
              AlreadyReplacedMatches = 0
              AlreadyReplacedFiles = 0
              AlreadyReplacedPaths = [] }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        interpreter.Execute(AppEffect.replaceWorkspace request)
        |> Async.RunSynchronously

        match commands |> Seq.toList with
        | [ WorkspaceReplacementCompleted result ] ->
            Assert.Equal<string list>([ path ], result.StalePaths)
            Assert.Empty(result.ReplacedPaths)
            Assert.Empty(fileService.Writes)
        | _ -> Assert.True(false, "Expected one workspace replacement completion.")

    [<Fact>]
    member _.``workspace replacement reports write failures``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = InterpreterFileService(Ok "original", Error "access denied")
        let path = "C:\work\file.fs"

        let request =
            { RequestId = Guid.NewGuid()
              WorkspaceId = Guid.NewGuid()
              StalePaths = []
              Sources = Map.ofList [ path, "original" ]
              Replacements = Map.ofList [ path, "replacement" ]
              MatchCounts = Map.ofList [ path, 1 ]
              AlreadyReplacedMatches = 0
              AlreadyReplacedFiles = 0
              AlreadyReplacedPaths = [] }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        interpreter.Execute(AppEffect.replaceWorkspace request)
        |> Async.RunSynchronously

        match commands |> Seq.toList with
        | [ WorkspaceReplacementCompleted result ] ->
            Assert.Empty(result.ReplacedPaths)
            Assert.Contains("access denied", result.Errors.Head)
        | _ -> Assert.True(false, "Expected one workspace replacement completion.")

    [<Fact>]
    member _.``workspace replacement reports partial success and failure``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = PartialWorkspaceReplacementFileService()
        let successfulPath = "C:\work\successful.fs"
        let failedPath = "C:\\work\\failed.fs"

        let request =
            { RequestId = Guid.NewGuid()
              WorkspaceId = Guid.NewGuid()
              StalePaths = []
              Sources = Map.ofList [ successfulPath, "term"; failedPath, "term" ]
              Replacements = Map.ofList [ successfulPath, "word"; failedPath, "word" ]
              MatchCounts = Map.ofList [ successfulPath, 1; failedPath, 1 ]
              AlreadyReplacedMatches = 0
              AlreadyReplacedFiles = 0
              AlreadyReplacedPaths = [] }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        interpreter.Execute(AppEffect.replaceWorkspace request)
        |> Async.RunSynchronously

        match commands |> Seq.toList with
        | [ WorkspaceReplacementCompleted result ] ->
            Assert.Equal<string list>([ successfulPath ], result.ReplacedPaths)
            Assert.Empty(result.StalePaths)
            Assert.Single(result.Errors) |> ignore
            Assert.Contains(failedPath, result.Errors.Head)
            Assert.Contains("disk full", result.Errors.Head)

            Assert.Equal<(string * string) list>(
                [ failedPath, "word"; successfulPath, "word" ],
                List.ofSeq fileService.Writes
            )
        | _ -> Assert.True(false, "Expected one workspace replacement completion.")

    [<Fact>]
    member _.``no effect does not dispatch an application command``() =
        let commands = ResizeArray<AppCommand>()

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                Unchecked.defaultof<IFileService>,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        interpreter.Execute(AppEffect.noEffect) |> Async.RunSynchronously

        Assert.Empty(commands)

    [<Fact>]
    member _.``canceling save dialog does not write or dispatch``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = InterpreterFileService(Ok "", Ok())
        let dialogService = InterpreterDialogService(None, None)

        let interpreter =
            AppEffectInterpreter(Unchecked.defaultof<IClipboardService>, fileService, dialogService, commands.Add)

        interpreter.Execute(AppEffect.saveFile (Some "file.fs") "contents")
        |> Async.RunSynchronously

        Assert.Empty(commands)
        Assert.Empty(fileService.Writes)

    [<Fact>]
    member _.``executes tokenization and dispatches completion``() =
        let commands = ResizeArray<AppCommand>()
        let service = InterpreterTokenizerService([])

        let request =
            { DocumentId = Guid.NewGuid()
              Revision = 2L
              Language = "fsharp"
              Scope = FullDocument
              Lines = [ "let value" ]
              InitialState = Initial }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                Unchecked.defaultof<IFileService>,
                Unchecked.defaultof<IDialogService>,
                commands.Add,
                tokenizerService = (service :> ITokenizerService)
            )

        interpreter.Execute(AppEffect.tokenize request) |> Async.RunSynchronously

        Assert.Single(commands) |> ignore

        match commands[0] with
        | TokenizationCompleted result ->
            Assert.Equal(request.DocumentId, result.DocumentId)
            Assert.Equal(request.Revision, result.Revision)
            Assert.Empty(result.Tokens)
        | command -> Assert.True(false, $"Expected tokenization completion but received {command}")

    [<Fact>]
    member _.``reports an error when tokenization has no configured service``() =
        let commands = ResizeArray<AppCommand>()

        let request =
            { DocumentId = Guid.NewGuid()
              Revision = 0L
              Language = "fsharp"
              Scope = FullDocument
              Lines = [ "let value" ]
              InitialState = Initial }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                Unchecked.defaultof<IFileService>,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        interpreter.Execute(AppEffect.tokenize request) |> Async.RunSynchronously

        Assert.Single(commands) |> ignore
        Assert.Equal(AppCommand.reportError "No tokenizer service is configured.", commands[0])

    [<Fact>]
    member _.``reports tokenizer failures as application commands``() =
        let commands = ResizeArray<AppCommand>()

        let request =
            { DocumentId = Guid.NewGuid()
              Revision = 0L
              Language = "fsharp"
              Scope = FullDocument
              Lines = [ "let value" ]
              InitialState = Initial }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                Unchecked.defaultof<IFileService>,
                Unchecked.defaultof<IDialogService>,
                commands.Add,
                tokenizerService = (FailingInterpreterTokenizerService "tokenizer failed" :> ITokenizerService)
            )

        interpreter.Execute(AppEffect.tokenize request) |> Async.RunSynchronously

        Assert.Single(commands) |> ignore
        Assert.Equal(AppCommand.reportError "tokenizer failed", commands[0])

    [<Fact>]
    member _.``ignores tokenizer cancellation``() =
        let commands = ResizeArray<AppCommand>()

        let request =
            { DocumentId = Guid.NewGuid()
              Revision = 0L
              Language = "fsharp"
              Scope = FullDocument
              Lines = [ "let value" ]
              InitialState = Initial }

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                Unchecked.defaultof<IFileService>,
                Unchecked.defaultof<IDialogService>,
                commands.Add,
                tokenizerService = (CancelingInterpreterTokenizerService() :> ITokenizerService)
            )

        interpreter.Execute(AppEffect.tokenize request) |> Async.RunSynchronously

        Assert.Empty(commands)

    [<Fact>]
    member _.``reads selected file and dispatches completion``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = InterpreterFileService(Ok "contents", Ok())
        let dialogService = InterpreterDialogService(Some "C:\\work\\file.fs", None)

        let interpreter =
            AppEffectInterpreter(Unchecked.defaultof<IClipboardService>, fileService, dialogService, commands.Add)

        interpreter.Execute(AppEffect.openFile) |> Async.RunSynchronously

        Assert.Single(commands) |> ignore
        Assert.True(AppCommand.fileOpened "C:\\work\\file.fs" "contents" = commands[0])

    [<Fact>]
    member _.``open file effect updates the editor session through the interpreter``() =
        let session = EditorSession()
        let fileService = InterpreterFileService(Ok "one\ntwo", Ok())
        let dialogService = InterpreterDialogService(Some "C:\\work\\file.fs", None)

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                dialogService,
                (fun command -> session.DispatchCommand(command) |> ignore)
            )

        interpreter.Execute(AppEffect.openFile) |> Async.RunSynchronously

        Assert.Equal(Some "C:\\work\\file.fs", session.State.Model.ActiveDocument.Value.Metadata.Path)
        Assert.Equal<string list>([ "one"; "two" ], session.State.Model.Editing.Buffer)

    [<Fact>]
    member _.``opens selected folder and dispatches completion``() =
        let commands = ResizeArray<AppCommand>()

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                Unchecked.defaultof<IFileService>,
                InterpreterDialogService(None, None),
                commands.Add
            )

        interpreter.Execute(AppEffect.openFolder) |> Async.RunSynchronously

        Assert.Single(commands) |> ignore
        Assert.True(AppCommand.folderOpened "C:\\work\\folder" = commands[0])

    [<Fact>]
    member _.``writes selected save file and dispatches completion``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = InterpreterFileService(Ok "", Ok())
        let dialogService = InterpreterDialogService(None, Some "C:\\work\\file.fs")

        let interpreter =
            AppEffectInterpreter(Unchecked.defaultof<IClipboardService>, fileService, dialogService, commands.Add)

        interpreter.Execute(AppEffect.saveFile (Some "file.fs") "contents")
        |> Async.RunSynchronously

        Assert.Single(commands) |> ignore
        Assert.True(AppCommand.fileSaved "C:\\work\\file.fs" = commands[0])
        Assert.True([ ("C:\\work\\file.fs", "contents") ] = List.ofSeq fileService.Writes)

    [<Fact>]
    member _.``serializes concurrent writes to the same path``() =
        let fileService = ConcurrentWriteObservingFileService()
        let interpreter =
            AppEffectInterpreter(Unchecked.defaultof<IClipboardService>, fileService, Unchecked.defaultof<IDialogService>, ignore)

        let first = Async.StartAsTask(interpreter.Execute(AppEffect.writeFile "C:\work\file.fs" "first"))
        let second = Async.StartAsTask(interpreter.Execute(AppEffect.writeFile "C:\work\file.fs" "second"))

        System.Threading.Tasks.Task.WhenAll([| first; second |])
        |> Async.AwaitTask
        |> Async.RunSynchronously
        |> ignore

        Assert.Equal(1, fileService.MaximumConcurrentWrites)

    [<Fact>]
    member _.``reports file failures as application commands``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = InterpreterFileService(Error "read failed", Ok())
        let dialogService = InterpreterDialogService(Some "C:\\work\\file.fs", None)

        let interpreter =
            AppEffectInterpreter(Unchecked.defaultof<IClipboardService>, fileService, dialogService, commands.Add)

        interpreter.Execute(AppEffect.openFile) |> Async.RunSynchronously

        Assert.Single(commands) |> ignore
        Assert.True(AppCommand.fileOperationFailed "read failed" = commands[0])

    [<Fact>]
    member _.``reports write failures as application commands``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = InterpreterFileService(Ok "", Error "write failed")

        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                Unchecked.defaultof<IDialogService>,
                commands.Add
            )

        interpreter.Execute(AppEffect.writeFile "C:\\work\\file.fs" "contents")
        |> Async.RunSynchronously

        Assert.Single(commands) |> ignore
        Assert.True(AppCommand.fileOperationFailed "write failed" = commands[0])
