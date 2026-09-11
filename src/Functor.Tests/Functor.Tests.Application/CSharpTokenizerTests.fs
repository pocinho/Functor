namespace Functor.Tests.Application

open System
open System.Threading
open Functor.Application
open Functor.Domain.Syntax
open Xunit

module private CSharpTokenizerTest =
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

    let allTokens (result: Result<TokenizationOutput, string>) =
        match result with
        | Ok output -> output.Tokens |> List.collect (fun line -> line.Tokens)
        | Error message -> failwith message

type CSharpTokenizerTests() =
    [<Fact>]
    member _.``tokenizes common lexical constructs``() =
        let tokens =
            CSharpTokenizerTest.tokenize "csharp" [ "public class Greeter { string Name = \"Ada\"; // note" ]
            |> CSharpTokenizerTest.allTokens

        Assert.Equal<string list>(
            [ "keyword"; "keyword"; "type"; "punctuation"; "keyword"; "identifier"; "operator"; "string"; "punctuation"; "comment" ],
            tokens |> List.map (fun token -> token.Kind)
        )

        let spans = tokens |> List.map (fun token -> token.Column, token.Length)
        Assert.True([ 0, 6; 7, 5; 13, 7; 21, 1; 23, 6; 30, 4; 35, 1; 37, 5; 42, 1; 44, 7 ] = spans)

    [<Fact>]
    member _.``returns verbatim string lexer snapshots``() =
        let result = CSharpTokenizerTest.tokenize "csharp" [ "var text = @\"one"; "two\";" ]

        let output =
            match result with
            | Ok value -> value
            | Error message -> failwith message

        Assert.Equal(CSharpState(false, false, false), output.Snapshots.[0].State)
        Assert.Equal(CSharpState(false, true, false), output.Snapshots.[1].State)
        Assert.Equal(CSharpState(false, false, false), output.FinalState)