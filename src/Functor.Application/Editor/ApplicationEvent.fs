namespace Functor.Application

open System
open Functor.Domain.Document

type ApplicationEventOutcome =
    | CommandApplied
    | CommandDeferred
    | CommandRejected of failureCategory: string

type ApplicationEvent =
    { CorrelationId: Guid
      CommandName: string
      DocumentId: DocumentId option
      WorkspaceId: Guid
      Duration: TimeSpan
      Outcome: ApplicationEventOutcome }

module AppCommandMetadata =
    let name command =
        match command with
        | ExecuteCoreEvent _ -> "core.event"
        | SetStatus _ -> "status.set"
        | ClearStatus -> "status.clear"
        | ReportError _ -> "error.report"
        | NewDocumentRequested -> "document.new"
        | OpenFileRequested -> "file.open"
        | OpenDocumentRequested _ -> "document.open"
        | OpenFolderRequested -> "folder.open"
        | SaveFileRequested -> "file.save"
        | SaveFileAsRequested -> "file.saveAs"
        | CloseDocumentRequested -> "document.close"
        | ReopenClosedTabRequested -> "tab.reopenClosed"
        | ReopenRecentDocument _ -> "document.reopenRecent"
        | ClearRecentDocumentsRequested -> "recent.clear"
        | OpenCommandPaletteRequested -> "commandPalette.open"
        | OpenSearchRequested -> "search.open"
        | OpenSearchReplaceRequested -> "search.replace.open"
        | OpenSearchReplaceAllRequested -> "search.replaceAll.open"
        | OpenSettingsRequested -> "settings.open"
        | ConfirmDiscardChanges -> "pending.confirm"
        | CancelPendingOperation -> "pending.cancel"
        | FileOpened _ -> "file.opened"
        | FolderOpened _ -> "folder.opened"
        | FileSaved _ -> "file.saved"
        | FileSavedForDocument _ -> "file.saved.document"
        | FileOperationFailed _ -> "file.operationFailed"
        | RequestTokenization _ -> "tokenization.request"
        | TokenizationCompleted _ -> "tokenization.completed"
        | ToggleAgentPanel -> "agentPanel.toggle"
        | SetAgentOpen _ -> "agentPanel.setOpen"
        | SearchQueryChanged _ -> "search.queryChanged"
        | SearchOptionsChanged _ -> "search.optionsChanged"
        | RefreshSearchRequested -> "search.refresh"
        | NextSearchResultRequested -> "search.next"
        | PreviousSearchResultRequested -> "search.previous"
        | SearchResultActivated _ -> "search.result.activate"
        | ReplaceCurrentSearch _ -> "search.replace"
        | ReplaceCurrentSearchWithOptions _ -> "search.replaceWithOptions"
        | ReplaceAllSearch _ -> "search.replaceAll"
        | ClearSearchRequested -> "search.clear"
        | ClearSearchHistoryRequested -> "search.history.clear"
        | OpenDocumentsSearchRequested -> "search.documents.open"
        | WorkspaceSearchRequested -> "search.workspace"
        | WorkspaceSearchCompleted _ -> "search.workspace.completed"
        | WorkspaceReplacementCompleted _ -> "search.workspace.replace.completed"

module AppCommandFailureMetadata =
    let category failure =
        match failure with
        | NoActiveDocument -> "no-active-document"
        | UnsupportedDocumentPath _ -> "unsupported-document-path"
        | NoRecentlyClosedTab -> "no-recently-closed-tab"
        | NoPendingOperation -> "no-pending-operation"