namespace Functor.Application

type AppCommandDescriptor =
    { Id: string
      Title: string
      Category: string
      GestureText: string option
      Command: AppCommand
      IsEnabled: AppSessionState -> bool }

module AppCommandCatalog =
    let private alwaysEnabled _ = true

    let private hasActiveDocument state =
        state.Model.ActiveDocument.IsSome

    let private hasRecentlyClosedDocument state =
      not state.Workspace.RecentlyClosedDocuments.IsEmpty

    let all =
        [ { Id = "file.new"
            Title = "New File"
            Category = "File"
            GestureText = Some "Ctrl+N"
            Command = AppCommand.newDocument
            IsEnabled = alwaysEnabled }
          { Id = "file.open"
            Title = "Open File"
            Category = "File"
            GestureText = Some "Ctrl+O"
            Command = AppCommand.openFile
            IsEnabled = alwaysEnabled }
          { Id = "workspace.openFolder"
            Title = "Open Folder"
            Category = "Workspace"
            GestureText = None
            Command = AppCommand.openFolder
            IsEnabled = alwaysEnabled }
          { Id = "file.save"
            Title = "Save File"
            Category = "File"
            GestureText = Some "Ctrl+S"
            Command = AppCommand.saveFile
            IsEnabled = hasActiveDocument }
          { Id = "file.saveAs"
            Title = "Save File As"
            Category = "File"
            GestureText = Some "Ctrl+Shift+S"
            Command = AppCommand.saveFileAs
            IsEnabled = alwaysEnabled }
          { Id = "file.close"
            Title = "Close File"
            Category = "File"
            GestureText = None
            Command = AppCommand.closeDocument
            IsEnabled = hasActiveDocument }
          { Id = "file.reopenClosedTab"
            Title = "Reopen Closed Tab"
            Category = "File"
            GestureText = None
            Command = AppCommand.reopenClosedTab
            IsEnabled = hasRecentlyClosedDocument }
          { Id = "file.clearRecentDocuments"
            Title = "Clear Recent Documents"
            Category = "File"
            GestureText = None
            Command = AppCommand.clearRecentDocuments
            IsEnabled = hasRecentlyClosedDocument }
          { Id = "workbench.commandPalette"
            Title = "Open Command Palette"
            Category = "Workbench"
            GestureText = Some "Ctrl+Shift+P"
            Command = AppCommand.openCommandPalette
            IsEnabled = alwaysEnabled }
          { Id = "workbench.settings"
            Title = "Open Settings"
            Category = "Preferences"
            GestureText = Some "Ctrl+,"
            Command = AppCommand.openSettings
            IsEnabled = alwaysEnabled } ]

    let tryFindById id =
        all |> List.tryFind (fun descriptor -> descriptor.Id = id)

    let filter (query: string) (commands: AppCommandDescriptor list) =
        let normalized = query.Trim()

        if System.String.IsNullOrWhiteSpace normalized then
            commands
        else
            let contains (value: string) =
                value.Contains(normalized, System.StringComparison.OrdinalIgnoreCase)

            commands
            |> List.filter (fun command ->
                contains command.Title
                || contains command.Category
                || (command.GestureText |> Option.exists contains))