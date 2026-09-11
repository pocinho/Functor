namespace Functor.Domain.Syntax

open Functor.Domain.Document

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
    | MarkSyntaxCleanFrom of line:int

    // ────────────────────────────────────────────────
    // Token Cache Updates
    // ────────────────────────────────────────────────
    | SetTokens of documentId:DocumentId * revision:int64 * tokens:LineTokens list
    | SetTokenRange of documentId:DocumentId * revision:int64 * startLine:int * endLine:int * tokens:LineTokens list
