namespace Functor.Domain.Syntax

open Functor.Domain.Document

/// Events related to syntax processing:
/// - language changes
/// - tokenization requests
/// - incremental tokenization
/// - syntax invalidation
/// - metadata updates
type LanguageEvent =
    // ────────────────────────────────────────────────
    // Language
    // ────────────────────────────────────────────────
    | SetLanguage of language: string

type TokenizationEvent =
    | TokenizeFull // tokenize entire document
    | TokenizeLine of line: int // tokenize a single line
    | TokenizeRange of start: int * endLine: int

type SyntaxInvalidationEvent =
    | MarkSyntaxDirty // editing occurred → syntax outdated
    | MarkSyntaxCleanFrom of line: int

type TokenCacheEvent =
    | SetTokens of documentId: DocumentId * revision: int64 * tokens: LineTokens list
    | SetTokenRange of
        documentId: DocumentId *
        revision: int64 *
        startLine: int *
        endLine: int *
        tokens: LineTokens list

type SyntaxEvent =
    | Language of LanguageEvent
    | Tokenization of TokenizationEvent
    | Invalidation of SyntaxInvalidationEvent
    | TokenCache of TokenCacheEvent

    static member SetLanguage language =
        Language(LanguageEvent.SetLanguage language)

    static member TokenizeFull = Tokenization TokenizationEvent.TokenizeFull

    static member TokenizeLine line =
        Tokenization(TokenizationEvent.TokenizeLine line)

    static member TokenizeRange(startLine, endLine) =
        Tokenization(TokenizationEvent.TokenizeRange(startLine, endLine))

    static member MarkSyntaxDirty = Invalidation SyntaxInvalidationEvent.MarkSyntaxDirty

    static member MarkSyntaxCleanFrom line =
        Invalidation(SyntaxInvalidationEvent.MarkSyntaxCleanFrom line)

    static member SetTokens(documentId, revision, tokens) =
        TokenCache(TokenCacheEvent.SetTokens(documentId, revision, tokens))

    static member SetTokenRange(documentId, revision, startLine, endLine, tokens) =
        TokenCache(TokenCacheEvent.SetTokenRange(documentId, revision, startLine, endLine, tokens))

[<AutoOpen>]
module SyntaxEventConstructors =
    let SetLanguage language = SyntaxEvent.SetLanguage language
    let TokenizeFull = SyntaxEvent.TokenizeFull
    let TokenizeLine line = SyntaxEvent.TokenizeLine line

    let TokenizeRange (startLine, endLine) =
        SyntaxEvent.TokenizeRange(startLine, endLine)

    let MarkSyntaxDirty = SyntaxEvent.MarkSyntaxDirty
    let MarkSyntaxCleanFrom line = SyntaxEvent.MarkSyntaxCleanFrom line

    let SetTokens (documentId, revision, tokens) =
        SyntaxEvent.SetTokens(documentId, revision, tokens)

    let SetTokenRange (documentId, revision, startLine, endLine, tokens) =
        SyntaxEvent.SetTokenRange(documentId, revision, startLine, endLine, tokens)
