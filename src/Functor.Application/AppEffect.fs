namespace Functor.Application

/// Effects requested by the application layer and handled by frontend or platform adapters.
type AppEffect =
    | NoEffect
    | NotifyStatus of string
    | NotifyError of string
    | WriteClipboard of string
    | ReadClipboard
    | PasteText of string
    | OpenFile
    | ReadFile of path: string
    | SaveFile of suggestedName: string option * contents: string
    | WriteFile of path: string * contents: string

module AppEffect =
    let noEffect = NoEffect

    let notifyStatus (message: string) = NotifyStatus message

    let notifyError (error: string) = NotifyError error

    let writeClipboard (text: string) = WriteClipboard text

    let readClipboard = ReadClipboard

    let pasteText (text: string) = PasteText text

    let openFile = OpenFile

    let readFile path = ReadFile path

    let saveFile suggestedName contents = SaveFile(suggestedName, contents)

    let writeFile path contents = WriteFile(path, contents)
