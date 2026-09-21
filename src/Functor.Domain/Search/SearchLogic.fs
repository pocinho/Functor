namespace Functor.Domain.Search

open System

module SearchLogic =
    let private setSearchQuery (model: SearchModel) (query: string) =
        { model with
            Query = if String.IsNullOrEmpty query then None else Some query
            Options = { model.Options with Query = query }
            Results = []
            Matches = []
            Revision = None
            Index = None
            IsDirty = true }

    let private setSearchResults (model: SearchModel) (results: SearchResult list) =
        { model with
            Results = results
            Index = if results.IsEmpty then None else Some 0
            IsDirty = false }

    let private setSearchOptions (model: SearchModel) (options: SearchOptions) =
        { model with
            Options = options
            Query =
                if String.IsNullOrEmpty options.Query then
                    None
                else
                    Some options.Query
            Results = []
            Matches = []
            Revision = None
            Index = None
            IsDirty = true }

    let private setSearchMatches (model: SearchModel) revision (matches: SearchMatch list) =
        { model with
            Results =
                matches
                |> List.map (fun matchValue ->
                    { Line = matchValue.Line
                      Column = matchValue.Column
                      Preview = matchValue.Preview })
            Matches = matches
            Revision = Some revision
            Index = if matches.IsEmpty then None else Some 0
            IsDirty = false }

    let private nextSearchResult (model: SearchModel) =
        match model.Index with
        | None -> model
        | Some index ->
            let nextIndex = index + 1

            if nextIndex >= model.Results.Length then
                model
            else
                { model with Index = Some nextIndex }

    let private prevSearchResult (model: SearchModel) =
        match model.Index with
        | None -> model
        | Some index ->
            if index = 0 then
                model
            else
                { model with Index = Some(index - 1) }

    let private clearSearch (model: SearchModel) =
        { model with
            Query = None
            Results = []
            Matches = []
            Options = SearchOptions.create ""
            Revision = None
            Index = None }

    let private invalidateSearch (model: SearchModel) =
        { model with
            Matches = []
            Results = []
            Revision = None
            Index = None
            IsDirty = true }

    let update (event: SearchEvent) (model: SearchModel) : SearchModel =
        match event with
        | SetSearchQuery query -> setSearchQuery model query
        | SetSearchResults results -> setSearchResults model results
        | SetSearchMatches(revision, matches) -> setSearchMatches model revision matches
        | SetSearchOptions options -> setSearchOptions model options
        | NextSearchResult -> nextSearchResult model
        | PrevSearchResult -> prevSearchResult model
        | ClearSearch -> clearSearch model
        | InvalidateSearch -> invalidateSearch model
