namespace Functor.Tests.Application

open System
open System.Threading
open Functor.Application
open Functor.Domain.Syntax
open Xunit

module private JsonTokenizerTest =
    let tokenizeWithScope scope language lines =
        let service = DefaultTokenizerService() :> ITokenizerService
        let request =
            { DocumentId = Guid.NewGuid()
              Revision = 1L
              Language = language
              Scope = scope
              Lines = lines
              InitialState = Initial }

        service.Tokenize(request, CancellationToken.None) |> Async.RunSynchronously

    let tokenize language lines =
        tokenizeWithScope FullDocument language lines

    let allTokens (result: Result<TokenizationOutput, string>) =
        match result with
        | Ok output -> output.Tokens |> List.collect (fun line -> line.Tokens)
        | Error message -> failwith message

type JsonTokenizerTests() =
    [<Fact>]
    member _.``tokenizes common lexical constructs``() =
        let tokens =
            JsonTokenizerTest.tokenize "json" [ "{ \"name\": \"Ada\", \"age\": 42, \"ok\": true, \"missing\": null }" ]
            |> JsonTokenizerTest.allTokens

        Assert.Equal<string list>(
            [ "punctuation"; "property"; "punctuation"; "string"; "punctuation"; "property"; "punctuation"; "number"; "punctuation"; "property"; "punctuation"; "keyword"; "punctuation"; "property"; "punctuation"; "keyword"; "punctuation" ],
            tokens |> List.map (fun token -> token.Kind)
        )

        let spans = tokens |> List.map (fun token -> token.Column, token.Length)
        Assert.True([ 0, 1; 2, 6; 8, 1; 10, 5; 15, 1; 17, 5; 22, 1; 24, 2; 26, 1; 28, 4; 32, 1; 34, 4; 38, 1; 40, 9; 49, 1; 51, 4; 56, 1 ] = spans)

    [<Fact>]
    member _.``tokenizes only requested line ranges with absolute line numbers``() =
        let result =
            JsonTokenizerTest.tokenizeWithScope (LineRange(1, 2)) "json" [ "{"; "\"name\": \"Ada\","; "\"age\": 42"; "}" ]

        let lines =
            match result with
            | Ok output -> output.Tokens
            | Error message -> failwith message

        Assert.Equal<int list>([ 1; 2 ], lines |> List.map (fun line -> line.Line))
        Assert.True(lines |> List.forall (fun line -> line.Tokens |> List.forall (fun token -> token.Line = line.Line)))