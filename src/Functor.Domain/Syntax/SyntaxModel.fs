namespace Functor.Domain.Syntax

open System

/// Represents a single lexical token produced by the tokenizer.
type Token =
    {
        Kind : string          // e.g., "identifier", "keyword", "string", "comment"
        Line : int
        Column : int
        Length : int
    }

/// Represents the result of tokenizing a single line.
type LineTokens =
    {
        Line : int
        Tokens : Token list
    }

/// Represents the syntax state of a document:
/// - language identity
/// - cached tokens
/// - incremental tokenization metadata
/// - last tokenization timestamp
type SyntaxModel =
    {
        /// Language ID (e.g., "fsharp", "csharp", "json")
        Language : string option

        /// Cached tokens for each line.
        /// Rendering and semantic layers consume this.
        Tokens : LineTokens list

        /// Timestamp of last tokenization.
        LastTokenized : DateTime option

        /// Whether the syntax model is currently out of date.
        /// (e.g., editing occurred since last tokenization)
        IsDirty : bool
    }

module SyntaxModel =

    /// Empty syntax model (no language, no tokens).
    let create () =
        {
            Language = None
            Tokens = []
            LastTokenized = None
            IsDirty = false
        }

    /// Marks the syntax model as needing re-tokenization.
    let markDirty (model: SyntaxModel) =
        { model with IsDirty = true }

    /// Updates the language ID.
    let setLanguage (model: SyntaxModel) (lang: string) =
        { model with Language = Some lang; IsDirty = true }

    /// Updates the token cache and timestamp.
    let setTokens (model: SyntaxModel) (tokens: LineTokens list) =
        {
            model with
                Tokens = tokens
                LastTokenized = Some DateTime.UtcNow
                IsDirty = false
        }
