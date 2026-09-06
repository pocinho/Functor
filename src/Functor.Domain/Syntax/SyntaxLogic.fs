namespace Functor.Domain.Syntax

open System

/// Pure syntax logic:
/// Applies a SyntaxEvent to a SyntaxModel and returns a new SyntaxModel.
module SyntaxLogic =

    // ────────────────────────────────────────────────
    // Language
    // ────────────────────────────────────────────────

    let private setLanguage (model: SyntaxModel) (lang: string) =
        { model with
            Language = Some lang
            IsDirty = true }

    // ────────────────────────────────────────────────
    // Tokenization Requests
    // ────────────────────────────────────────────────

    /// Mark entire syntax model as needing re-tokenization.
    let private tokenizeFull (model: SyntaxModel) =
        { model with IsDirty = true }

    /// Mark a single line as needing re-tokenization.
    /// (Actual tokenization will be done by the tokenizer module.)
    let private tokenizeLine (model: SyntaxModel) (line: int) =
        { model with IsDirty = true }

    /// Mark a range of lines as needing re-tokenization.
    let private tokenizeRange (model: SyntaxModel) (startLine: int) (endLine: int) =
        { model with IsDirty = true }

    // ────────────────────────────────────────────────
    // Syntax Invalidation
    // ────────────────────────────────────────────────

    let private markSyntaxDirty (model: SyntaxModel) =
        { model with IsDirty = true }

    // ────────────────────────────────────────────────
    // Metadata
    // ────────────────────────────────────────────────

    let private updateTokenizationTimestamp (model: SyntaxModel) (ts: DateTime) =
        { model with LastTokenized = Some ts }

    // ────────────────────────────────────────────────
    // Token Cache Updates
    // ────────────────────────────────────────────────

    let private setTokens (model: SyntaxModel) (tokens: LineTokens list) =
        {
            model with
                Tokens = tokens
                LastTokenized = Some DateTime.UtcNow
                IsDirty = false
        }

    // ────────────────────────────────────────────────
    // Main update function
    // ────────────────────────────────────────────────

    let update (evt: SyntaxEvent) (model: SyntaxModel) : SyntaxModel =
        match evt with
        | SetLanguage lang ->
            setLanguage model lang

        | TokenizeFull ->
            tokenizeFull model

        | TokenizeLine line ->
            tokenizeLine model line

        | TokenizeRange (startLine, endLine) ->
            tokenizeRange model startLine endLine

        | MarkSyntaxDirty ->
            markSyntaxDirty model

        | UpdateTokenizationTimestamp ts ->
            updateTokenizationTimestamp model ts

        | SetTokens tokens ->
            setTokens model tokens
