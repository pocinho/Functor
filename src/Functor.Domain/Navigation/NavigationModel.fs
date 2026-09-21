namespace Functor.Domain.Navigation

open Functor.Domain.Editing
open Functor.Domain.Search

/// Represents a jump target in the editor (e.g., after Go To Definition).
type Jump =
    { Position: Position
      Description: string option }

/// Represents a search result (line + column + preview).
type SearchResult =
    { Line: int
      Column: int
      Preview: string }

/// Represents the navigation state of the editor:
/// - jump list
/// - symbol index (future)
/// - navigation cursor
/// - history pointer
type NavigationModel =
    {
        /// Stack of jump locations (like VSCode, Helix, Zed).
        JumpList: Jump list

        /// Current index in the jump list.
        JumpIndex: int option

        /// Search state and current result navigation.
        Search: SearchModel

        /// Symbol index (future extension).
        Symbols: (string * Position) list

        /// Whether navigation state is outdated (e.g., after edits).
        IsDirty: bool
    }

module NavigationModel =

    /// Empty navigation state.
    let create () =
        { JumpList = []
          JumpIndex = None
          Search = SearchModel.create ()
          Symbols = []
          IsDirty = false }

    /// Marks navigation state as needing refresh.
    let markDirty (model: NavigationModel) = { model with IsDirty = true }
