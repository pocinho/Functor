namespace Functor.Tests.Application

open System
open System.Threading
open Functor.Application
open Functor.Domain.Syntax
open Xunit

module private FSharpTokenizerTest =
    let tokenize language lines =
        let service = DefaultTokenizerService() :> ITokenizerService
        let request =
            { DocumentId = Guid.NewGuid()
              Revision = 1L
              Language = language
              Scope = FullDocument
              Lines = lines
              InitialState = Initial }

        service.Tokenize(request, CancellationToken.None) |> Async.RunSynchronously

    let tokenizeFromState state lines =
        let service = DefaultTokenizerService() :> ITokenizerService
        let request =
            { DocumentId = Guid.NewGuid()
              Revision = 1L
              Language = "fsharp"
              Scope = LineRange(1, 1)
              Lines = lines
              InitialState = state }

        service.Tokenize(request, CancellationToken.None) |> Async.RunSynchronously

    let allTokens (result: Result<TokenizationOutput, string>) =
        match result with
        | Ok output -> output.Tokens |> List.collect (fun line -> line.Tokens)
        | Error message -> failwith message

type FSharpTokenizerTests() =
    [<Fact>]
    member _.``tokenizes common lexical constructs``() =
        let tokens = FSharpTokenizerTest.tokenize "fsharp" [ "let value = 42 // note" ] |> FSharpTokenizerTest.allTokens

        Assert.Equal<string list>(
            [ "keyword"; "identifier"; "operator"; "number"; "comment" ],
            tokens |> List.map (fun token -> token.Kind)
        )

        let spans = tokens |> List.map (fun token -> token.Column, token.Length)
        Assert.True([ 0, 3; 4, 5; 10, 1; 12, 2; 15, 7 ] = spans)

    [<Fact>]
    member _.``preserves UTF16 token offsets``() =
        let tokens = FSharpTokenizerTest.tokenize "fs" [ "let π = 1" ] |> FSharpTokenizerTest.allTokens
        let identifier = tokens |> List.find (fun token -> token.Kind = "identifier")

        Assert.Equal(4, identifier.Column)
        Assert.Equal(1, identifier.Length)

    [<Fact>]
    member _.``tracks nested block comments across lines``() =
        let result = FSharpTokenizerTest.tokenize "fsharp" [ "(* outer"; "(* inner *)"; "*) let value" ]

        let lines =
            match result with
            | Ok output -> output.Tokens
            | Error message -> failwith message

        Assert.Equal(3, lines.Length)
        Assert.Equal("comment", lines.[0].Tokens.Head.Kind)
        Assert.Equal("comment", lines.[1].Tokens.Head.Kind)
        Assert.Equal("comment", lines.[2].Tokens.Head.Kind)
        Assert.Equal("keyword", lines.[2].Tokens.[1].Kind)

    [<Fact>]
    member _.``returns block comment lexer snapshots``() =
        let result = FSharpTokenizerTest.tokenize "fsharp" [ "(* outer"; "body"; "*)" ]

        let output =
            match result with
            | Ok value -> value
            | Error message -> failwith message

        Assert.True([ 0; 1; 2 ] = (output.Snapshots |> List.map (fun snapshot -> snapshot.Line)))
        Assert.Equal(FSharpState 0, output.Snapshots.[0].State)
        Assert.Equal(FSharpState 1, output.Snapshots.[1].State)
        Assert.Equal(FSharpState 1, output.Snapshots.[2].State)
        Assert.Equal(FSharpState 0, output.FinalState)

    [<Fact>]
    member _.``rehydrates block comment state for a line range``() =
        let result = FSharpTokenizerTest.tokenizeFromState (FSharpState 1) [ "ignored"; "let value" ]

        let output =
            match result with
            | Ok value -> value
            | Error message -> failwith message

        Assert.Equal("comment", output.Tokens.Head.Tokens.Head.Kind)
        Assert.Equal(FSharpState 1, output.FinalState)