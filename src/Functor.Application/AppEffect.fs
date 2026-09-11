namespace Functor.Application

open Functor.Domain.Document

/// Effects requested by the application layer and handled by frontend or platform adapters.
type AppEffect =
    | NoEffect
    | NotifyStatus of string
    | NotifyError of string
    | WriteClipboard of string
    | ReadClipboard
    | PasteText of string
    | OpenFile
    | OpenFolder
    | ReadFile of path: string
    | SaveFile of suggestedName: string option * contents: string
    | SaveFileForDocument of documentId: DocumentId * revision: int64 * suggestedName: string option * contents: string
    | WriteFile of path: string * contents: string
    | WriteFileForDocument of documentId: DocumentId * revision: int64 * path: string * contents: string
    | Tokenize of TokenizationRequest

module AppEffect =
    let noEffect = NoEffect

    let notifyStatus (message: string) = NotifyStatus message

    let notifyError (error: string) = NotifyError error

    let writeClipboard (text: string) = WriteClipboard text

    let readClipboard = ReadClipboard

    let pasteText (text: string) = PasteText text

    let openFile = OpenFile

    let openFolder = OpenFolder

    let readFile path = ReadFile path

    let saveFile suggestedName contents = SaveFile(suggestedName, contents)

    let saveFileForDocument documentId revision suggestedName contents =
        SaveFileForDocument(documentId, revision, suggestedName, contents)

    let writeFile path contents = WriteFile(path, contents)

    let writeFileForDocument documentId revision path contents =
        WriteFileForDocument(documentId, revision, path, contents)

    let tokenize request = Tokenize request
