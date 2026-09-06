namespace Functor.Domain.Syntax

open System

/// Events related to syntax processing:
/// - language changes
/// - tokenization requests
/// - incremental tokenization
/// - syntax invalidation
/// - metadata updates
type SyntaxEvent =
    // ────────────────────────────────────────────────
    // Language
    // ────────────────────────────────────────────────
    | SetLanguage of language:string

    // ────────────────────────────────────────────────
    // Tokenization
    // ────────────────────────────────────────────────
    | TokenizeFull                       // tokenize entire document
    | TokenizeLine of line:int           // tokenize a single line
    | TokenizeRange of start:int * endLine:int

    // ────────────────────────────────────────────────
    // Syntax Invalidation
    // ────────────────────────────────────────────────
    | MarkSyntaxDirty                    // editing occurred → syntax outdated

    // ────────────────────────────────────────────────
    // Metadata
    // ────────────────────────────────────────────────
    | UpdateTokenizationTimestamp of timestamp:DateTime

    // ────────────────────────────────────────────────
    // Token Cache Updates
    // ────────────────────────────────────────────────
    | SetTokens of tokens:LineTokens list
