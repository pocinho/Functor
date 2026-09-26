namespace Functor.Application

open System
open Functor.Domain.Document
open Functor.Domain.Search
open Functor.Workspace

type WorkspaceSearchRequest =
    { RequestId: Guid
      WorkspaceId: WorkspaceId
      RootPath: string
      Options: SearchOptions
      OpenDocuments: SearchDocument list
      OpenDocumentPaths: Set<string>
      OpenDocumentRevisions: Map<DocumentId, int64> }

type WorkspaceSearchResult =
    { RequestId: Guid
      WorkspaceId: WorkspaceId
      Options: SearchOptions
      Documents: SearchDocument list
      Sources: Map<string, string>
      Matches: SearchMatch list
      Errors: string list
      OpenDocumentRevisions: Map<DocumentId, int64> }

type WorkspaceReplacementRequest =
    { RequestId: Guid
      WorkspaceId: WorkspaceId
      StalePaths: string list
      Sources: Map<string, string>
      Replacements: Map<string, string>
      MatchCounts: Map<string, int>
      AlreadyReplacedMatches: int
      AlreadyReplacedFiles: int
      AlreadyReplacedPaths: string list }

type WorkspaceReplacementResult =
    { RequestId: Guid
      WorkspaceId: WorkspaceId
      ReplacedPaths: string list
      ReplacedMatches: int
      ReplacedFiles: int
      StalePaths: string list
      Errors: string list }
