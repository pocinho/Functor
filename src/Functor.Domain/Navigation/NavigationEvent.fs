namespace Functor.Domain.Navigation

open Functor.Domain.Editing

/// Events related to navigation:
/// - jump list operations
/// - search operations
/// - symbol index updates
/// - navigation cursor movement
/// - invalidation after edits
type NavigationEvent =
    // ────────────────────────────────────────────────
    // Jump List
    // ────────────────────────────────────────────────
    | PushJump of position:Position * description:string option
    | JumpBack
    | JumpForward
    | ClearJumpList

    // ────────────────────────────────────────────────
    // Search
    // ────────────────────────────────────────────────
    | SetSearchQuery of query:string
    | SetSearchResults of results:SearchResult list
    | NextSearchResult
    | PrevSearchResult
    | ClearSearch

    // ────────────────────────────────────────────────
    // Symbols (future extension)
    // ────────────────────────────────────────────────
    | SetSymbols of symbols:(string * Position) list

    // ────────────────────────────────────────────────
    // Navigation Invalidation
    // ────────────────────────────────────────────────
    | MarkNavigationDirty
