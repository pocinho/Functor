namespace Functor.Domain.Navigation

open Functor.Domain.Editing

/// Pure navigation logic:
/// Applies a NavigationEvent to a NavigationModel and returns a new NavigationModel.
module NavigationLogic =

    // ────────────────────────────────────────────────
    // Jump List
    // ────────────────────────────────────────────────

    let private pushJump (model: NavigationModel) (pos: Position) (desc: string option) =
        let newJump = { Position = pos; Description = desc }
        let newList = newJump :: model.JumpList
        { model with
            JumpList = newList
            JumpIndex = Some 0 }

    let private jumpBack (model: NavigationModel) =
        match model.JumpIndex with
        | None -> model
        | Some idx ->
            let newIdx = idx + 1
            if newIdx >= model.JumpList.Length then model
            else { model with JumpIndex = Some newIdx }

    let private jumpForward (model: NavigationModel) =
        match model.JumpIndex with
        | None -> model
        | Some idx ->
            if idx = 0 then model
            else { model with JumpIndex = Some (idx - 1) }

    let private clearJumpList (model: NavigationModel) =
        { model with JumpList = []; JumpIndex = None }

    // ────────────────────────────────────────────────
    // Search
    // ────────────────────────────────────────────────

    let private setSearchQuery (model: NavigationModel) (query: string) =
        { model with
            SearchQuery = Some query
            SearchResults = []
            SearchIndex = None
            IsDirty = true }

    let private setSearchResults (model: NavigationModel) (results: SearchResult list) =
        { model with
            SearchResults = results
            SearchIndex = (if results.IsEmpty then None else Some 0)
            IsDirty = false }

    let private nextSearchResult (model: NavigationModel) =
        match model.SearchIndex with
        | None -> model
        | Some idx ->
            let newIdx = idx + 1
            if newIdx >= model.SearchResults.Length then model
            else { model with SearchIndex = Some newIdx }

    let private prevSearchResult (model: NavigationModel) =
        match model.SearchIndex with
        | None -> model
        | Some idx ->
            if idx = 0 then model
            else { model with SearchIndex = Some (idx - 1) }

    let private clearSearch (model: NavigationModel) =
        { model with
            SearchQuery = None
            SearchResults = []
            SearchIndex = None }

    // ────────────────────────────────────────────────
    // Symbols
    // ────────────────────────────────────────────────

    let private setSymbols (model: NavigationModel) (symbols: (string * Position) list) =
        { model with Symbols = symbols }

    // ────────────────────────────────────────────────
    // Navigation Invalidation
    // ────────────────────────────────────────────────

    let private markNavigationDirty (model: NavigationModel) =
        { model with IsDirty = true }

    // ────────────────────────────────────────────────
    // Main update function
    // ────────────────────────────────────────────────

    let update (evt: NavigationEvent) (model: NavigationModel) : NavigationModel =
        match evt with
        | PushJump (pos, desc) -> pushJump model pos desc
        | JumpBack -> jumpBack model
        | JumpForward -> jumpForward model
        | ClearJumpList -> clearJumpList model

        | SetSearchQuery q -> setSearchQuery model q
        | SetSearchResults results -> setSearchResults model results
        | NextSearchResult -> nextSearchResult model
        | PrevSearchResult -> prevSearchResult model
        | ClearSearch -> clearSearch model

        | SetSymbols symbols -> setSymbols model symbols

        | MarkNavigationDirty -> markNavigationDirty model
