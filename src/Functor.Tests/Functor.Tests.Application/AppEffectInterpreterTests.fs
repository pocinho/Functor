namespace Functor.Tests.Application

open System
open System.Threading
open Functor.Application
open Functor.Domain.Syntax
open Xunit

type private InterpreterFileService(readResult: Result<string, string>, writeResult: Result<unit, string>) =
    let writes = ResizeArray<string * string>()

    member _.Writes = writes

    interface IFileService with
        member _.ReadText _ = async { return readResult }

        member _.WriteText(path, contents) =
            async {
                writes.Add(path, contents)
                return writeResult
            }

type private InterpreterDialogService(openPath: string option, savePath: string option) =
    interface IDialogService with
        member _.OpenFile() = async { return openPath }

        member _.OpenFolder() =
            async { return Some "C:\\work\\folder" }

        member _.SaveFile _ = async { return savePath }

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

type AppEffectInterpreterTests() =
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
                session.DispatchCommand
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
