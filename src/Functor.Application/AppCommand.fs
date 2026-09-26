namespace Functor.Application

open Functor.Domain.Core
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Search
open Functor.Workspace

/// High-level application commands emitted by the UI layer.
/// These commands remain free of platform-specific behavior.
type AppCommand =
    | ExecuteCoreEvent of CoreEvent
    | SetStatus of string
    | ClearStatus
    | ReportError of string
    | NewDocumentRequested
    | OpenFileRequested
    | OpenDocumentRequested of path: string
    | OpenFolderRequested
    | SaveFileRequested
    | SaveFileAsRequested
    | CloseDocumentRequested
    | ReopenClosedTabRequested
    | ReopenRecentDocument of path: string
    | ClearRecentDocumentsRequested
    | OpenCommandPaletteRequested
    | OpenSearchRequested
    | OpenSearchReplaceRequested
    | OpenSearchReplaceAllRequested
    | OpenSettingsRequested
    | ConfirmDiscardChanges
    | CancelPendingOperation
    | FileOpened of path: string * contents: string
    | FolderOpened of path: string
    | FileSaved of path: string
    | FileSavedForDocument of documentId: DocumentId * revision: int64 * path: string
    | FileOperationFailed of message: string
    | RequestTokenization of language: string
    | TokenizationCompleted of TokenizationResult
    | ToggleAgentPanel
    | SetAgentOpen of documentId: DocumentId * isOpen: bool
    | SearchQueryChanged of query: string
    | SearchOptionsChanged of options: SearchOptions
    | RefreshSearchRequested
    | NextSearchResultRequested
    | PreviousSearchResultRequested
    | SearchResultActivated of result: SearchMatch
    | ReplaceCurrentSearch of replacement: string
    | ReplaceCurrentSearchWithOptions of options: SearchOptions * replacement: string
    | ReplaceAllSearch of replacement: string
    | ClearSearchRequested
    | ClearSearchHistoryRequested
    | OpenDocumentsSearchRequested
    | WorkspaceSearchRequested
    | WorkspaceSearchCompleted of result: WorkspaceSearchResult
    | WorkspaceReplacementCompleted of result: WorkspaceReplacementResult

module AppCommand =
    let toCoreEvent (event: CoreEvent) = ExecuteCoreEvent event

    let setStatus (message: string) = SetStatus message

    let clearStatus = ClearStatus

    let reportError (error: string) = ReportError error

    let openFile = OpenFileRequested

    let openDocument path = OpenDocumentRequested path

    let openFolder = OpenFolderRequested

    let saveFile = SaveFileRequested

    let newDocument = NewDocumentRequested

    let saveFileAs = SaveFileAsRequested

    let closeDocument = CloseDocumentRequested

    let reopenClosedTab = ReopenClosedTabRequested

    let reopenRecentDocument path = ReopenRecentDocument path

    let clearRecentDocuments = ClearRecentDocumentsRequested

    let openCommandPalette = OpenCommandPaletteRequested

    let openSearch = OpenSearchRequested

    let openSearchReplace = OpenSearchReplaceRequested

    let openSearchReplaceAll = OpenSearchReplaceAllRequested

    let openSettings = OpenSettingsRequested

    let confirmDiscardChanges = ConfirmDiscardChanges

    let cancelPendingOperation = CancelPendingOperation

    let fileOpened path contents = FileOpened(path, contents)

    let folderOpened path = FolderOpened path

    let fileSaved path = FileSaved path

    let fileSavedForDocument documentId revision path =
        FileSavedForDocument(documentId, revision, path)

    let fileOperationFailed message = FileOperationFailed message

    let requestTokenization language = RequestTokenization language

    let tokenizationCompleted result = TokenizationCompleted result

    let toggleAgentPanel = ToggleAgentPanel

    let setAgentOpen documentId isOpen = SetAgentOpen(documentId, isOpen)

    let searchQueryChanged query = SearchQueryChanged query

    let searchOptionsChanged options = SearchOptionsChanged options

    let refreshSearch = RefreshSearchRequested

    let nextSearchResult = NextSearchResultRequested

    let previousSearchResult = PreviousSearchResultRequested

    let activateSearchResult result = SearchResultActivated result

    let replaceCurrentSearch replacement = ReplaceCurrentSearch replacement

    let replaceCurrentSearchWithOptions options replacement =
        ReplaceCurrentSearchWithOptions(options, replacement)

    let replaceAllSearch replacement = ReplaceAllSearch replacement

    let clearSearch = ClearSearchRequested

    let clearSearchHistory = ClearSearchHistoryRequested

    let searchOpenDocuments = OpenDocumentsSearchRequested

    let workspaceSearch = WorkspaceSearchRequested

    let workspaceSearchCompleted result = WorkspaceSearchCompleted result

    let workspaceReplacementCompleted result = WorkspaceReplacementCompleted result
