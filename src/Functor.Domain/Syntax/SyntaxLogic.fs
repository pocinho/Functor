namespace Functor.Domain.Syntax

/// Pure syntax logic:
/// Applies a SyntaxEvent to a SyntaxModel and returns a new SyntaxModel.
module SyntaxLogic =

    // ────────────────────────────────────────────────
    // Language
    // ────────────────────────────────────────────────

    let private setLanguage (model: SyntaxModel) (lang: string) =
        SyntaxModel.setLanguage model lang

    // ────────────────────────────────────────────────
    // Tokenization Requests
    // ────────────────────────────────────────────────

    /// Mark entire syntax model as needing re-tokenization.
    let private tokenizeFull (model: SyntaxModel) =
        SyntaxModel.markDirty model.DocumentRevision model

    /// Mark a single line as needing re-tokenization.
    /// (Actual tokenization will be done by the tokenizer module.)
    let private tokenizeLine (model: SyntaxModel) (line: int) =
        SyntaxModel.markDirtyRange model.DocumentRevision line line model

    /// Mark a range of lines as needing re-tokenization.
    let private tokenizeRange (model: SyntaxModel) (startLine: int) (endLine: int) =
        SyntaxModel.markDirtyRange model.DocumentRevision startLine endLine model

    // ────────────────────────────────────────────────
    // Syntax Invalidation
    // ────────────────────────────────────────────────

    let private markSyntaxDirty (model: SyntaxModel) =
        SyntaxModel.markDirty model.DocumentRevision model

    let private markSyntaxCleanFrom (model: SyntaxModel) line =
        SyntaxModel.markCleanFrom line model

    // ────────────────────────────────────────────────
    // Token Cache Updates
    // ────────────────────────────────────────────────

    let private setTokens documentId revision tokens (model: SyntaxModel) =
        SyntaxModel.setTokens documentId revision tokens model

    let private setTokenRange documentId revision startLine endLine tokens (model: SyntaxModel) =
        SyntaxModel.setTokenRange documentId revision startLine endLine tokens model

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

        | MarkSyntaxCleanFrom line ->
            markSyntaxCleanFrom model line

        | SetTokens(documentId, revision, tokens) ->
            setTokens documentId revision tokens model

        | SetTokenRange(documentId, revision, startLine, endLine, tokens) ->
            setTokenRange documentId revision startLine endLine tokens model
