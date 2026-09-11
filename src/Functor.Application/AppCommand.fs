namespace Functor.Application

open Functor.Domain.Core
open Functor.Domain.Document
open Functor.Domain.Editing

/// High-level application commands emitted by the UI layer.
/// These commands remain free of platform-specific behavior.
type AppCommand =
    | ExecuteCoreEvent of CoreEvent
    | SetStatus of string
    | ClearStatus
    | ReportError of string
    | NewDocumentRequested
    | OpenFileRequested
    | OpenFolderRequested
    | SaveFileRequested
    | SaveFileAsRequested
    | CloseDocumentRequested
    | ReopenClosedTabRequested
    | ReopenRecentDocument of path: string
    | ClearRecentDocumentsRequested
    | OpenCommandPaletteRequested
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

module AppCommand =
    let toCoreEvent (event: CoreEvent) = ExecuteCoreEvent event

    let setStatus (message: string) = SetStatus message

    let clearStatus = ClearStatus

    let reportError (error: string) = ReportError error

    let openFile = OpenFileRequested

    let openFolder = OpenFolderRequested

    let saveFile = SaveFileRequested

    let newDocument = NewDocumentRequested

    let saveFileAs = SaveFileAsRequested

    let closeDocument = CloseDocumentRequested

    let reopenClosedTab = ReopenClosedTabRequested

    let reopenRecentDocument path = ReopenRecentDocument path

    let clearRecentDocuments = ClearRecentDocumentsRequested

    let openCommandPalette = OpenCommandPaletteRequested

    let openSettings = OpenSettingsRequested

    let confirmDiscardChanges = ConfirmDiscardChanges

    let cancelPendingOperation = CancelPendingOperation

    let fileOpened path contents = FileOpened(path, contents)

    let folderOpened path = FolderOpened path

    let fileSaved path = FileSaved path

    let fileSavedForDocument documentId revision path = FileSavedForDocument(documentId, revision, path)

    let fileOperationFailed message = FileOperationFailed message

    let requestTokenization language = RequestTokenization language

    let tokenizationCompleted result = TokenizationCompleted result
