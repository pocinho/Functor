namespace Functor.Application

type EditorServices =
    { Clipboard: IClipboardService
      File: IFileService
      Dialog: IDialogService
      Tokenizer: ITokenizerService option }

module EditorServices =
    let create clipboard file dialog tokenizer =
        { Clipboard = clipboard
          File = file
          Dialog = dialog
          Tokenizer = tokenizer }
