namespace Functor.Application

type AppCommandDescriptor =
    { Id: string
      Title: string
      Description: string
      Category: string
      GestureText: string option
      Command: AppCommand
      IsEnabled: AppSessionState -> bool }

module AppCommandCatalog =
    let private alwaysEnabled _ = true

    let private hasActiveDocument state = state.Model.ActiveDocument.IsSome

    let private hasRecentlyClosedDocument state =
        not state.Workspace.RecentlyClosedDocuments.IsEmpty

    let all =
        [ { Id = "file.new"
            Title = "New File"
            Description = "Create a new untitled document"
            Category = "File"
            GestureText = Some "Ctrl+N"
            Command = AppCommand.newDocument
            IsEnabled = alwaysEnabled }
          { Id = "file.open"
            Title = "Open File"
            Description = "Open a document from disk"
            Category = "File"
            GestureText = Some "Ctrl+O"
            Command = AppCommand.openFile
            IsEnabled = alwaysEnabled }
          { Id = "workspace.openFolder"
            Title = "Open Folder"
            Description = "Open a folder as the workspace root"
            Category = "Workspace"
            GestureText = None
            Command = AppCommand.openFolder
            IsEnabled = alwaysEnabled }
          { Id = "file.save"
            Title = "Save File"
            Description = "Save the active document"
            Category = "File"
            GestureText = Some "Ctrl+S"
            Command = AppCommand.saveFile
            IsEnabled = hasActiveDocument }
          { Id = "file.saveAs"
            Title = "Save File As"
            Description = "Save the active document to a new path"
            Category = "File"
            GestureText = Some "Ctrl+Shift+S"
            Command = AppCommand.saveFileAs
            IsEnabled = alwaysEnabled }
          { Id = "file.close"
            Title = "Close File"
            Description = "Close the active document"
            Category = "File"
            GestureText = None
            Command = AppCommand.closeDocument
            IsEnabled = hasActiveDocument }
          { Id = "file.reopenClosedTab"
            Title = "Reopen Closed Tab"
            Description = "Restore the most recently closed document"
            Category = "File"
            GestureText = None
            Command = AppCommand.reopenClosedTab
            IsEnabled = hasRecentlyClosedDocument }
          { Id = "file.clearRecentDocuments"
            Title = "Clear Recent Documents"
            Description = "Remove the recent document history"
            Category = "File"
            GestureText = None
            Command = AppCommand.clearRecentDocuments
            IsEnabled = hasRecentlyClosedDocument }
          { Id = "workbench.commandPalette"
            Title = "Open Command Palette"
            Description = "Browse and run available commands"
            Category = "Workbench"
            GestureText = Some "Ctrl+Shift+P"
            Command = AppCommand.openCommandPalette
            IsEnabled = alwaysEnabled }
          { Id = "workbench.settings"
            Title = "Open Settings"
            Description = "Configure Functor preferences"
            Category = "Preferences"
            GestureText = Some "Ctrl+,"
            Command = AppCommand.openSettings
            IsEnabled = alwaysEnabled }
          { Id = "search.refresh"
            Title = "Refresh Search"
            Description = "Refresh search results from the current buffers"
            Category = "Search"
            GestureText = None
            Command = AppCommand.refreshSearch
            IsEnabled = alwaysEnabled }
          { Id = "search.next"
            Title = "Next Search Result"
            Description = "Select the next search result"
            Category = "Search"
            GestureText = None
            Command = AppCommand.nextSearchResult
            IsEnabled = alwaysEnabled }
          { Id = "search.previous"
            Title = "Previous Search Result"
            Description = "Select the previous search result"
            Category = "Search"
            GestureText = None
            Command = AppCommand.previousSearchResult
            IsEnabled = alwaysEnabled }
          { Id = "search.clearHistory"
            Title = "Clear Search History"
            Description = "Remove saved search queries"
            Category = "Search"
            GestureText = None
            Command = AppCommand.clearSearchHistory
            IsEnabled = alwaysEnabled }
          { Id = "workbench.toggleAgentPanel"
            Title = "Toggle Agent Panel"
            Description = "Show or hide the agent panel"
            Category = "Workbench"
            GestureText = None
            Command = AppCommand.toggleAgentPanel
            IsEnabled = hasActiveDocument } ]

    let tryFindById id =
        all |> List.tryFind (fun descriptor -> descriptor.Id = id)

    let private fuzzyScore (query: string) (value: string) =
        let normalizedQuery = query.Trim().ToLowerInvariant()
        let normalizedValue = value.ToLowerInvariant()

        if normalizedValue = normalizedQuery then
            Some 0
        elif normalizedValue.StartsWith(normalizedQuery, System.StringComparison.Ordinal) then
            Some 1
        elif normalizedValue.Contains(normalizedQuery, System.StringComparison.Ordinal) then
            Some 2
        else
            let rec matchCharacters queryIndex valueIndex score =
                if queryIndex = normalizedQuery.Length then
                    Some(score + 10)
                elif valueIndex = normalizedValue.Length then
                    None
                elif normalizedQuery[queryIndex] = normalizedValue[valueIndex] then
                    matchCharacters (queryIndex + 1) (valueIndex + 1) (score + valueIndex)
                else
                    matchCharacters queryIndex (valueIndex + 1) score

            matchCharacters 0 0 0

    let filter (query: string) (commands: AppCommandDescriptor list) =
        let normalized = if isNull query then "" else query.Trim()

        if System.String.IsNullOrWhiteSpace normalized then
            commands
        else
            commands
        |> List.choose (fun command ->
            [ fuzzyScore normalized command.Title
              fuzzyScore normalized command.Category
              command.GestureText |> Option.bind (fuzzyScore normalized) ]
            |> List.choose id
            |> List.sort
            |> List.tryHead
            |> Option.map (fun score -> score, command))
        |> List.sortBy fst
        |> List.map snd
