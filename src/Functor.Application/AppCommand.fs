namespace Functor.Application

open Functor.Domain.Core
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
    | SaveFileRequested
    | SaveFileAsRequested
    | CloseDocumentRequested
    | OpenCommandPaletteRequested
    | OpenSettingsRequested
    | ConfirmDiscardChanges
    | CancelPendingOperation
    | FileOpened of path: string * contents: string
    | FileSaved of path: string
    | FileOperationFailed of message: string

module AppCommand =
    let toCoreEvent (event: CoreEvent) = ExecuteCoreEvent event

    let setStatus (message: string) = SetStatus message

    let clearStatus = ClearStatus

    let reportError (error: string) = ReportError error

    let openFile = OpenFileRequested

    let saveFile = SaveFileRequested

    let newDocument = NewDocumentRequested

    let saveFileAs = SaveFileAsRequested

    let closeDocument = CloseDocumentRequested

    let openCommandPalette = OpenCommandPaletteRequested

    let openSettings = OpenSettingsRequested

    let confirmDiscardChanges = ConfirmDiscardChanges

    let cancelPendingOperation = CancelPendingOperation

    let fileOpened path contents = FileOpened(path, contents)

    let fileSaved path = FileSaved path

    let fileOperationFailed message = FileOperationFailed message
