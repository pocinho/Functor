namespace Functor.Domain.Search

open System

module SearchLogic =
    let private historyLimit = 20

    let private recordHistory (model: SearchModel) (options: SearchOptions) =
        if String.IsNullOrEmpty options.Query then
            model.History
        else
            options :: (model.History |> List.filter ((<>) options))
            |> List.truncate historyLimit

    let private setSearchQuery (model: SearchModel) (query: string) =
        { model with
            Query = if String.IsNullOrEmpty query then None else Some query
            Options = { model.Options with Query = query }
            Results = []
            Matches = []
            ModifiedFiles = []
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
            ModifiedFiles = []
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
            History = recordHistory model model.Options
            Revision = Some revision
            Index = if matches.IsEmpty then None else Some 0
            IsDirty = false }

    let private setModifiedFiles (model: SearchModel) files =
        { model with
            ModifiedFiles = files |> List.distinct }

    let private setSearchIndex (model: SearchModel) index =
        if index >= 0 && index < model.Matches.Length then
            { model with Index = Some index }
        else
            model

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
            ModifiedFiles = []
            Options = SearchOptions.create ""
            Revision = None
            Index = None }

    let private clearSearchHistory (model: SearchModel) = { model with History = [] }

    let private invalidateSearch (model: SearchModel) =
        { model with
            Matches = []
            Results = []
            ModifiedFiles = []
            Revision = None
            Index = None
            IsDirty = true }

    let update (event: SearchEvent) (model: SearchModel) : SearchModel =
        match event with
        | SetSearchQuery query -> setSearchQuery model query
        | SetSearchResults results -> setSearchResults model results
        | SetSearchMatches(revision, matches) -> setSearchMatches model revision matches
        | SetModifiedFiles files -> setModifiedFiles model files
        | SetSearchIndex index -> setSearchIndex model index
        | SetSearchOptions options -> setSearchOptions model options
        | NextSearchResult -> nextSearchResult model
        | PrevSearchResult -> prevSearchResult model
        | ClearSearch -> clearSearch model
        | ClearSearchHistory -> clearSearchHistory model
        | InvalidateSearch -> invalidateSearch model
