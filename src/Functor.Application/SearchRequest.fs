namespace Functor.Application

open System
open Functor.Domain.Search
open Functor.Workspace

type WorkspaceSearchRequest =
    { RequestId: Guid
      WorkspaceId: WorkspaceId
      RootPath: string
      Options: SearchOptions
      OpenDocuments: SearchDocument list
      OpenDocumentPaths: Set<string> }

type WorkspaceSearchResult =
    { RequestId: Guid
      WorkspaceId: WorkspaceId
      Options: SearchOptions
      Matches: SearchMatch list
      Errors: string list }
