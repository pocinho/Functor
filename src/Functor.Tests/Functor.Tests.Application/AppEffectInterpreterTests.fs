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

        member _.OpenFolder() = async { return Some "C:\\work\\folder" }

        member _.SaveFile _ = async { return savePath }

type private InterpreterTokenizerService(tokens: LineTokens list) =
    interface ITokenizerService with
        member _.Tokenize(_, _: CancellationToken) =
            async {
                return Ok { Provider = LocalLexical; Layer = Lexical; Tokens = tokens; Snapshots = []; FinalState = Initial }
            }

type AppEffectInterpreterTests() =
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
        | command ->
            Assert.True(false, $"Expected tokenization completion but received {command}")

    [<Fact>]
    member _.``reads selected file and dispatches completion``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = InterpreterFileService(Ok "contents", Ok())
        let dialogService = InterpreterDialogService(Some "C:\\work\\file.fs", None)
        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                dialogService,
                commands.Add
            )

        interpreter.Execute(AppEffect.openFile) |> Async.RunSynchronously

        Assert.Single(commands) |> ignore
        Assert.True(AppCommand.fileOpened "C:\\work\\file.fs" "contents" = commands[0])

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
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                dialogService,
                commands.Add
            )

        interpreter.Execute(AppEffect.saveFile (Some "file.fs") "contents") |> Async.RunSynchronously

        Assert.Single(commands) |> ignore
        Assert.True(AppCommand.fileSaved "C:\\work\\file.fs" = commands[0])
        Assert.True([ ("C:\\work\\file.fs", "contents") ] = List.ofSeq fileService.Writes)

    [<Fact>]
    member _.``reports file failures as application commands``() =
        let commands = ResizeArray<AppCommand>()
        let fileService = InterpreterFileService(Error "read failed", Ok())
        let dialogService = InterpreterDialogService(Some "C:\\work\\file.fs", None)
        let interpreter =
            AppEffectInterpreter(
                Unchecked.defaultof<IClipboardService>,
                fileService,
                dialogService,
                commands.Add
            )

        interpreter.Execute(AppEffect.openFile) |> Async.RunSynchronously

        Assert.Single(commands) |> ignore
        Assert.True(AppCommand.fileOperationFailed "read failed" = commands[0])
