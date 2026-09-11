namespace Functor.Application

type DefaultTokenizerService() =
    interface ITokenizerService with
        member _.Tokenize(request, cancellationToken) =
            async {
                match request.Language.Trim().ToLowerInvariant() with
                | "fsharp"
                | "f#"
                | "fs"
                | "fsx"
                | "fsi" ->
                    return FSharpTokenizer.tokenize request cancellationToken
                | "csharp"
                | "c#"
                | "cs" ->
                    return CSharpTokenizer.tokenize request cancellationToken
                | "json" ->
                    return JsonTokenizer.tokenize request cancellationToken
                | "markdown"
                | "md" ->
                    return MarkdownTokenizer.tokenize request cancellationToken
                | language ->
                    return Error $"Unsupported language: {language}"
            }