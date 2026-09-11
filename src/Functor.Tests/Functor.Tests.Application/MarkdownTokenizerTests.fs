namespace Functor.Tests.Application

open System
open System.Threading
open Functor.Application
open Functor.Domain.Syntax
open Xunit

module private MarkdownTokenizerTest =
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

type MarkdownTokenizerTests() =
    [<Fact>]
    member _.``tokenizes common lexical constructs``() =
        let tokens =
            MarkdownTokenizerTest.tokenize "markdown" [ "# Title"; "Use **bold** `code` [link](url) <!-- note -->" ]
            |> MarkdownTokenizerTest.allTokens

        Assert.Equal<string list>(
            [ "heading"; "text"; "text"; "emphasis"; "code"; "link"; "comment" ],
            tokens |> List.map (fun token -> token.Kind)
        )

        let spans = tokens |> List.map (fun token -> token.Line, token.Column, token.Length)
        Assert.True([ 0, 0, 1; 0, 2, 5; 1, 0, 3; 1, 4, 8; 1, 13, 6; 1, 20, 11; 1, 32, 13 ] = spans)

    [<Fact>]
    member _.``returns HTML comment and fenced code snapshots``() =
        let result =
            MarkdownTokenizerTest.tokenize "markdown" [ "<!-- open"; "comment -->"; "```fsharp"; "let value"; "```" ]

        let output =
            match result with
            | Ok value -> value
            | Error message -> failwith message

        Assert.Equal(MarkdownState(false, None), output.Snapshots.[0].State)
        Assert.Equal(MarkdownState(true, None), output.Snapshots.[1].State)
        Assert.Equal(MarkdownState(false, None), output.Snapshots.[2].State)
        Assert.Equal(MarkdownState(false, Some "```"), output.Snapshots.[3].State)
        Assert.Equal(MarkdownState(false, Some "```"), output.Snapshots.[4].State)
        Assert.Equal(MarkdownState(false, None), output.FinalState)