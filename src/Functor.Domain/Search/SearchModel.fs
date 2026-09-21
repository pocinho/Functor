namespace Functor.Domain.Search

open Functor.Domain.Document
open Functor.Domain.Editing

type SearchOptions = { Query: string; CaseSensitive: bool }

module SearchOptions =
    let create query =
        { Query = query; CaseSensitive = false }

type SearchDocument =
    { Id: DocumentId
      Path: string option
      Name: string
      Lines: string list }

type SearchMatch =
    { DocumentId: DocumentId
      Path: string option
      Name: string
      Line: int
      Column: int
      Length: int
      Range: Selection
      Preview: string }

type SearchResult =
    { Line: int
      Column: int
      Preview: string }

type SearchModel =
    { Query: string option
      Results: SearchResult list
      Matches: SearchMatch list
      Options: SearchOptions
      Revision: int64 option
      Index: int option
      IsDirty: bool }

module SearchModel =
    let create () =
        { Query = None
          Results = []
          Matches = []
          Options = SearchOptions.create ""
          Revision = None
          Index = None
          IsDirty = false }
