namespace Functor.Domain.Navigation

open Functor.Domain.Editing
open Functor.Domain.Search

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

            if newIdx >= model.JumpList.Length then
                model
            else
                { model with JumpIndex = Some newIdx }

    let private jumpForward (model: NavigationModel) =
        match model.JumpIndex with
        | None -> model
        | Some idx ->
            if idx = 0 then
                model
            else
                { model with JumpIndex = Some(idx - 1) }

    let private clearJumpList (model: NavigationModel) =
        { model with
            JumpList = []
            JumpIndex = None }

    // ────────────────────────────────────────────────
    // Symbols
    // ────────────────────────────────────────────────

    let private setSymbols (model: NavigationModel) (symbols: (string * Position) list) =
        { model with Symbols = symbols }

    // ────────────────────────────────────────────────
    // Navigation Invalidation
    // ────────────────────────────────────────────────

    let private markNavigationDirty (model: NavigationModel) = { model with IsDirty = true }

    // ────────────────────────────────────────────────
    // Main update function
    // ────────────────────────────────────────────────

    let update (evt: NavigationEvent) (model: NavigationModel) : NavigationModel =
        match evt with
        | JumpList jump ->
            match jump with
            | PushJump(pos, desc) -> pushJump model pos desc
            | JumpBack -> jumpBack model
            | JumpForward -> jumpForward model
            | ClearJumpList -> clearJumpList model
        | Search search ->
            let searchModel = SearchLogic.update search model.Search

            { model with
                Search = searchModel
                IsDirty = searchModel.IsDirty }
        | Symbols(SymbolEvent.SetSymbols symbols) -> setSymbols model symbols
        | Invalidation NavigationInvalidationEvent.MarkNavigationDirty -> markNavigationDirty model
