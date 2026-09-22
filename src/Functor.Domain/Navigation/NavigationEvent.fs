namespace Functor.Domain.Navigation

open Functor.Domain.Editing
open Functor.Domain.Search

/// Events related to navigation:
/// - jump list operations
/// - search operations
/// - symbol index updates
/// - navigation cursor movement
/// - invalidation after edits
type JumpListEvent =
    // ────────────────────────────────────────────────
    // Jump List
    // ────────────────────────────────────────────────
    | PushJump of position: Position * description: string option
    | JumpBack
    | JumpForward
    | ClearJumpList

type SymbolEvent = SetSymbols of symbols: (string * Position) list

type NavigationInvalidationEvent = | MarkNavigationDirty

type NavigationEvent =
    | JumpList of JumpListEvent
    | Search of SearchEvent
    | Symbols of SymbolEvent
    | Invalidation of NavigationInvalidationEvent

    static member PushJump(position, description) =
        JumpList(JumpListEvent.PushJump(position, description))

    static member JumpBack = JumpList JumpListEvent.JumpBack
    static member JumpForward = JumpList JumpListEvent.JumpForward
    static member ClearJumpList = JumpList JumpListEvent.ClearJumpList

    static member SetSearchQuery query =
        Search(SearchEvent.SetSearchQuery query)

    static member SetSearchResults results =
        Search(SearchEvent.SetSearchResults results)

    static member SetSearchMatches(revision, matches) =
        Search(SearchEvent.SetSearchMatches(revision, matches))

    static member SetSearchOptions options =
        Search(SearchEvent.SetSearchOptions options)

    static member NextSearchResult = Search SearchEvent.NextSearchResult
    static member PrevSearchResult = Search SearchEvent.PrevSearchResult
    static member ClearSearch = Search SearchEvent.ClearSearch
    static member ClearSearchHistory = Search SearchEvent.ClearSearchHistory
    static member InvalidateSearch = Search SearchEvent.InvalidateSearch
    static member SetSymbols symbols = Symbols(SymbolEvent.SetSymbols symbols)

    static member MarkNavigationDirty =
        Invalidation NavigationInvalidationEvent.MarkNavigationDirty

[<AutoOpen>]
module NavigationEventConstructors =
    let PushJump (position, description) =
        NavigationEvent.PushJump(position, description)

    let JumpBack = NavigationEvent.JumpBack
    let JumpForward = NavigationEvent.JumpForward
    let ClearJumpList = NavigationEvent.ClearJumpList
    let SetSearchQuery query = NavigationEvent.SetSearchQuery query

    let SetSearchResults results =
        NavigationEvent.SetSearchResults results

    let SetSearchMatches (revision, matches) =
        NavigationEvent.SetSearchMatches(revision, matches)

    let SetSearchOptions options =
        NavigationEvent.SetSearchOptions options

    let NextSearchResult = NavigationEvent.NextSearchResult
    let PrevSearchResult = NavigationEvent.PrevSearchResult
    let ClearSearch = NavigationEvent.ClearSearch
    let ClearSearchHistory = NavigationEvent.ClearSearchHistory
    let InvalidateSearch = NavigationEvent.InvalidateSearch
    let SetSymbols symbols = NavigationEvent.SetSymbols symbols
    let MarkNavigationDirty = NavigationEvent.MarkNavigationDirty
