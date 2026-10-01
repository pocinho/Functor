namespace Functor.Tests.Application

open System
open System.Diagnostics
open System.Threading
open Functor.Application
open Functor.Domain.Core
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Syntax
open Xunit

module private PerformanceRegression =
    let measure action =
        let stopwatch = Stopwatch.StartNew()
        action ()
        stopwatch.Stop()
        stopwatch.ElapsedMilliseconds

    let assertWithin budget operation =
        let elapsed = measure operation
        Assert.True(elapsed < budget, $"Operation took {elapsed} ms; budget was {budget} ms.")

type PerformanceRegressionTests() =
    [<Fact>]
    member _.``tokenizes a large document within the CI budget``() =
        let lines = [ for line in 1..10000 -> sprintf "let value%d = %d" line line ]
        let service = DefaultTokenizerService() :> ITokenizerService
        let request =
            { DocumentId = Guid.NewGuid()
              Revision = 1L
              Language = "fsharp"
              Scope = FullDocument
              Lines = lines
              InitialState = Initial }

        PerformanceRegression.assertWithin 5000 (fun () ->
            match service.Tokenize(request, CancellationToken.None) |> Async.RunSynchronously with
            | Ok output -> Assert.Equal(lines.Length, output.Tokens.Length)
            | Error message -> Assert.Fail(message))

    [<Fact>]
    member _.``switches across many editor tabs within the CI budget``() =
        let session = EditorSession()
        let documents =
            [ for index in 1..100 do
                  session.DispatchCommand(AppCommand.fileOpened ($"C:\\work\\tab-{index}.fs") "content")
                  yield session.State.Workspace.ActiveDocumentId.Value ]

        PerformanceRegression.assertWithin 1000 (fun () ->
            for documentId in documents do
                session.Dispatch(CoreEvent.SwitchDocument documentId))

    [<Fact>]
    member _.``applies rapid edits within the CI budget``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\rapid.fs" "")

        PerformanceRegression.assertWithin 1000 (fun () ->
            for _ in 1..500 do
                session.DispatchCommand(
                    AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "x"))
                ))

        Assert.Equal(500, session.Model.Editing.Buffer.Head.Length)
