namespace Functor.Domain.Search

type SearchEvent =
    | SetSearchQuery of query: string
    | SetSearchResults of results: SearchResult list
    | SetSearchMatches of revision: int64 * matches: SearchMatch list
    | SetSearchOptions of options: SearchOptions
    | NextSearchResult
    | PrevSearchResult
    | ClearSearch
    | ClearSearchHistory
    | InvalidateSearch
