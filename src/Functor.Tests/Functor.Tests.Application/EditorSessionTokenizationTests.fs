namespace Functor.Tests.Application

open System.Threading
open System
open Functor.Application
open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Xunit

type EditorSessionTokenizationTests() =
    [<Fact>]
    member _.``tokenization request snapshots the active document``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "let value\n42")
        requestedEffects.Clear()

        session.DispatchCommand(AppCommand.requestTokenization "fsharp")

        Assert.Single(requestedEffects) |> ignore

        match requestedEffects[0] with
        | [ Tokenize request ] ->
            Assert.Equal(session.Model.ActiveDocument.Value.Id, request.DocumentId)
            Assert.Equal(session.Model.Editing.Revision, request.Revision)
            Assert.Equal("fsharp", request.Language)
            Assert.True([ "let value"; "42" ] = request.Lines)
        | effects ->
            Assert.True(false, $"Expected one tokenization effect but received {effects}")

    [<Fact>]
    member _.``opening supported syntax files requests tokenization with selected language``() =
        let cases =
            [ "C:\\work\\file.fs", "fsharp"; "C:\\work\\file.cs", "csharp"; "C:\\work\\data.json", "json"; "C:\\work\\notes.md", "markdown"; "C:\\work\\notes.markdown", "markdown" ]

        for path, expectedLanguage in cases do
            let session = EditorSession()
            let requestedEffects = ResizeArray<AppEffect list>()
            session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

            session.DispatchCommand(AppCommand.fileOpened path "text")

            Assert.Single(requestedEffects) |> ignore

            match requestedEffects[0] with
            | [ Tokenize request ] ->
                Assert.Equal(expectedLanguage, request.Language)
                Assert.True([ "text" ] = request.Lines)
            | effects ->
                Assert.True(false, $"Expected one tokenization effect for {path} but received {effects}")

    [<Fact>]
    member _.``opening plain text files does not request tokenization``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\notes.txt" "text")

        Assert.Empty(requestedEffects)

    [<Fact>]
    member _.``editing debounces tokenization``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "let value")
        requestedEffects.Clear()

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        Assert.Empty(requestedEffects)
        Thread.Sleep(250)
        Assert.Single(requestedEffects) |> ignore

        match requestedEffects[0] with
        | [ Tokenize request ] -> Assert.Equal(session.Model.Editing.Revision, request.Revision)
        | effects -> Assert.True(false, $"Expected one tokenization effect but received {effects}")

    [<Fact>]
    member _.``editing requests only the dirty line range``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "let value\n42")
        requestedEffects.Clear()

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        Thread.Sleep(250)

        match requestedEffects[0] with
        | [ Tokenize request ] -> Assert.Equal(Line 0, request.Scope)
        | effects ->
            Assert.True(false, $"Expected one tokenization effect but received {effects}")

    [<Fact>]
    member _.``incremental tokenization stops when lexer state stabilizes``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "(* open\nbody\n*)")
        let document = session.Model.ActiveDocument.Value

        session.DispatchCommand(
            AppCommand.tokenizationCompleted
                { DocumentId = document.Id
                  Revision = session.Model.Editing.Revision
                  Scope = FullDocument
                  Provider = LocalLexical
                  Layer = Lexical
                  Tokens = []
                  Snapshots =
                    [ { Line = 0; State = FSharpState 0 }
                      { Line = 1; State = FSharpState 1 }
                      { Line = 2; State = FSharpState 1 } ]
                  FinalState = FSharpState 0 })

        requestedEffects.Clear()
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(SetCursor { Line = 1; Column = 0 })))
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "x")))
        Thread.Sleep(250)

        match requestedEffects[0] with
        | [ Tokenize request ] ->
            Assert.Equal(Line 1, request.Scope)
            Assert.Equal(FSharpState 1, request.InitialState)
        | effects ->
            Assert.True(false, $"Expected one tokenization effect but received {effects}")

        session.DispatchCommand(
            AppCommand.tokenizationCompleted
                { DocumentId = document.Id
                  Revision = session.Model.Editing.Revision
                  Scope = Line 1
                  Provider = LocalLexical
                  Layer = Lexical
                  Tokens = [ { Line = 1; Tokens = [] } ]
                  Snapshots = [ { Line = 1; State = FSharpState 1 } ]
                  FinalState = FSharpState 1 })

        Assert.False(session.Model.Syntax.IsDirty)

    [<Fact>]
    member _.``editing supported additional syntax files debounces selected language tokenization``() =
        let session = EditorSession()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\data.json" "{}")
        requestedEffects.Clear()

        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString " updated")))

        Assert.Empty(requestedEffects)
        Thread.Sleep(250)
        Assert.Single(requestedEffects) |> ignore

        match requestedEffects[0] with
        | [ Tokenize request ] ->
            Assert.Equal("json", request.Language)
            Assert.Equal(session.Model.Editing.Revision, request.Revision)
        | effects -> Assert.True(false, $"Expected one tokenization effect but received {effects}")

    [<Fact>]
    member _.``session rejects tokenization completed for an old revision``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "let value")
        let document = session.Model.ActiveDocument.Value
        session.DispatchCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "x")))
        let staleToken = { Kind = "keyword"; Line = 0; Column = 0; Length = 3 }
        let staleResult: TokenizationResult =
            { DocumentId = document.Id
              Revision = 0L
              Scope = FullDocument
              Provider = LocalLexical
              Layer = Lexical
              Tokens = [ { Line = 0; Tokens = [ staleToken ] } ]
              Snapshots = []
              FinalState = Initial }

        session.DispatchCommand(AppCommand.tokenizationCompleted staleResult)

        Assert.True(session.Model.Syntax.Tokens.IsEmpty)
        Assert.True(session.Model.Syntax.IsDirty)

    [<Fact>]
    member _.``session ignores tokenization completed for a non-open document``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "let value")
        let document = session.Model.ActiveDocument.Value
        let result: TokenizationResult =
            { DocumentId = Guid.NewGuid()
              Revision = session.Model.Editing.Revision
              Scope = FullDocument
              Provider = LocalLexical
              Layer = Lexical
              Tokens = [ { Line = 0; Tokens = [ { Kind = "keyword"; Line = 0; Column = 0; Length = 3 } ] } ]
              Snapshots = []
              FinalState = Initial }

        session.DispatchCommand(AppCommand.tokenizationCompleted result)

        Assert.Equal(Some document.Id, session.Model.Syntax.DocumentId)
        Assert.Empty(session.Model.Syntax.Tokens)

    [<Fact>]
    member _.``session applies tokenization completed for a line range without replacing other cached lines``() =
        let session = EditorSession()
        session.DispatchCommand(AppCommand.fileOpened "C:\\work\\file.fs" "let one\nlet two\nlet three")
        let document = session.Model.ActiveDocument.Value
        let existing =
            [ { Line = 0; Tokens = [ { Kind = "identifier"; Line = 0; Column = 0; Length = 3 } ] }
              { Line = 1; Tokens = [ { Kind = "identifier"; Line = 1; Column = 0; Length = 3 } ] }
              { Line = 2; Tokens = [ { Kind = "identifier"; Line = 2; Column = 0; Length = 3 } ] } ]
        let replacement =
            [ { Line = 1; Tokens = [ { Kind = "keyword"; Line = 1; Column = 0; Length = 3 } ] } ]

        session.DispatchCommand(
            AppCommand.tokenizationCompleted
                { DocumentId = document.Id
                  Revision = session.Model.Editing.Revision
                  Scope = FullDocument
                  Provider = LocalLexical
                  Layer = Lexical
                  Tokens = existing
                  Snapshots = []
                  FinalState = Initial })

        session.DispatchCommand(
            AppCommand.tokenizationCompleted
                { DocumentId = document.Id
                  Revision = session.Model.Editing.Revision
                  Scope = LineRange(1, 1)
                  Provider = LocalLexical
                  Layer = Lexical
                  Tokens = replacement
                  Snapshots = []
                  FinalState = Initial })

        Assert.True([ existing.[0]; replacement.[0]; existing.[2] ] = session.Model.Syntax.Tokens)
