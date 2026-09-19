namespace Functor.Avalonia.Views

open System
open System.IO
open System.Threading.Tasks
open Avalonia
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Layout
open Avalonia.Media
open Avalonia.Markup.Xaml
open Avalonia.Threading
open Functor.Application
open Functor.Avalonia
open Functor.Avalonia.Controls
open Functor.Domain.Document
open Functor.Workspace

type ShellHostView() as this =
    inherit UserControl()

    let editor = lazy (this.FindControl<EditorControl>("EditorControl"))
    let welcomeView = lazy (this.FindControl<WelcomeView>("WelcomeView"))
    let verticalScrollBar = lazy (this.FindControl<ScrollBar>("VerticalScrollBar"))
    let horizontalScrollBar = lazy (this.FindControl<ScrollBar>("HorizontalScrollBar"))
    let statusBar = lazy (this.FindControl<Border>("StatusBar"))
    let positionText = lazy (this.FindControl<TextBlock>("PositionText"))
    let fileTypeText = lazy (this.FindControl<TextBlock>("FileTypeText"))
    let messageText = lazy (this.FindControl<TextBlock>("MessageText"))
    let fileNameText = lazy (this.FindControl<TextBlock>("FileNameText"))
    let dirtyText = lazy (this.FindControl<TextBlock>("DirtyText"))
    let tabsPanel = lazy (this.FindControl<DocumentListView>("TabsPanel"))
    let sidePanelHost = lazy (this.FindControl<SidePanelControl>("SidePanelHost"))
    let settingsTabButton = lazy (this.FindControl<Button>("SettingsTabButton"))
    let settingsDocument = lazy (this.FindControl<Grid>("SettingsDocument"))
    let settingsView = lazy (this.FindControl<SettingsView>("SettingsView"))
    let applySettingsButton = lazy (this.FindControl<Button>("ApplySettingsButton"))
    let saveSettingsButton = lazy (this.FindControl<Button>("SaveSettingsButton"))
    let closeSettingsButton = lazy (this.FindControl<Button>("CloseSettingsButton"))

    let auxiliaryPanelHost =
        lazy (this.FindControl<SidePanelControl>("AuxiliaryPanelHost"))

    let mutable model = ShellModel.initial
    let mutable subscriptions: IDisposable list = []
    let mutable confirmationOpen = false
    let mutable refreshQueued = false
    let mutable cachedWorkspace: WorkspaceModel option = None
    let mutable cachedWorkspaceTree: WorkspaceFileTreeNode option = None
    let mutable pendingWorkspaceTree: WorkspaceModel option = None
    let mutable workspaceTreeGeneration = 0L
    let mutable requestRefresh: unit -> unit = ignore
    let mutable settingsOpen = false
    let mutable currentSettings = AppSettings.defaults
    let mutable applySettingsCallback: AppSettings -> unit = ignore

    let mutable saveSettingsCallback: AppSettings -> Result<unit, string> =
        fun _ -> Ok()

    let updateEmptyStateFromProjection hasActiveDocument =
        editor.Value.IsVisible <- not settingsOpen && hasActiveDocument
        welcomeView.Value.IsVisible <- not settingsOpen && not hasActiveDocument
        settingsDocument.Value.IsVisible <- settingsOpen
        settingsTabButton.Value.IsVisible <- settingsOpen
        settingsTabButton.Value.Classes.Set("selected", settingsOpen)

    let updateScrollBarFromProjection (scroll: ShellScrollPresentation) =
        let verticalScrollBar = verticalScrollBar.Value
        let horizontalScrollBar = horizontalScrollBar.Value

        verticalScrollBar.Maximum <- scroll.VerticalMaximum
        verticalScrollBar.ViewportSize <- scroll.VerticalViewport
        verticalScrollBar.LargeChange <- max 1.0 scroll.VerticalViewport
        verticalScrollBar.IsEnabled <- scroll.VerticalMaximum > 0.0
        verticalScrollBar.Value <- scroll.VerticalOffset
        horizontalScrollBar.Maximum <- scroll.HorizontalMaximum
        horizontalScrollBar.ViewportSize <- scroll.HorizontalViewport
        horizontalScrollBar.LargeChange <- max 1.0 scroll.HorizontalViewport
        horizontalScrollBar.IsEnabled <- scroll.HorizontalMaximum > 0.0
        horizontalScrollBar.Value <- scroll.HorizontalOffset

    let updateScrollBar () =
        updateScrollBarFromProjection (ShellProjection.fromEditor editor.Value).Scroll

    let updateEditorStatus (status: Functor.Application.EditorStatus) =
        positionText.Value.Text <- sprintf "Ln %d, Col %d" status.Line status.Column
        fileTypeText.Value.Text <- status.FileType
        messageText.Value.Text <- status.Error |> Option.orElse status.Message |> Option.defaultValue ""
        fileNameText.Value.Text <- status.FileName
        dirtyText.Value.Text <- if status.IsDirty then "Modified" else ""

    let updateTabsFromProjection tabs = tabsPanel.Value.ApplyTabs tabs

    let updateTabs (state: AppSessionState) =
        updateTabsFromProjection (WorkspaceProjection.tabs state.Workspace)

    let updateEmptyState (state: AppSessionState) =
        updateEmptyStateFromProjection state.Model.ActiveDocument.IsSome

    let updateToolRail () =
        let activeTool = model.Layout.ActiveTool
        let isPanelOpen = model.Layout.IsToolPanelOpen

        this
            .FindControl<Button>("WorkspaceToolButton")
            .Classes.Set("selected", isPanelOpen && activeTool = Some WorkspaceTool)

        this
            .FindControl<Button>("SearchToolButton")
            .Classes.Set("selected", isPanelOpen && activeTool = Some SearchTool)

    let updateToolRailMetadata () =
        let applyDescriptor controlName (descriptor: ToolDescriptor) =
            let button = this.FindControl<Button>(controlName)
            button.Content <- descriptor.Glyph
            button.SetValue(ToolTip.TipProperty, descriptor.AccessibilityName)

        applyDescriptor "WorkspaceToolButton" (ToolDescriptor.get WorkspaceTool)
        |> ignore

        applyDescriptor "SearchToolButton" (ToolDescriptor.get SearchTool) |> ignore

    let showDiscardDialog () =
        match TopLevel.GetTopLevel(this) with
        | :? Window as owner ->
            confirmationOpen <- true

            let dialog, setContent =
                ThemedDialogWindow.create currentSettings "Unsaved changes" 420.0 160.0

            let message =
                TextBlock(Text = "This document has unsaved changes. Discard them?", TextWrapping = TextWrapping.Wrap)

            let discardButton = Button(Content = "Discard")
            let cancelButton = Button(Content = "Cancel")
            let buttons = StackPanel(Orientation = Orientation.Horizontal, Spacing = 8.0)
            let content = StackPanel(Spacing = 16.0, Margin = Thickness(16.0))

            buttons.Children.Add(discardButton) |> ignore
            buttons.Children.Add(cancelButton) |> ignore
            content.Children.Add(message) |> ignore
            content.Children.Add(buttons) |> ignore
            setContent content

            discardButton.Click.Add(fun _ ->
                editor.Value.ConfirmDiscardChanges()
                dialog.Close())

            cancelButton.Click.Add(fun _ ->
                editor.Value.CancelPendingOperation()
                dialog.Close())

            dialog.Closed.Add(fun _ -> confirmationOpen <- false)
            dialog.ShowDialog(owner) |> ignore
        | _ -> editor.Value.CancelPendingOperation()

    let closeSettingsTab () =
        settingsOpen <- false
        settingsDocument.Value.IsVisible <- false
        settingsTabButton.Value.IsVisible <- false
        updateEmptyStateFromProjection editor.Value.SessionState.Workspace.ActiveDocumentId.IsSome
        editor.Value.Focus() |> ignore

    let openSettingsTab () =
        settingsView.Value.Configure(currentSettings)
        settingsOpen <- true
        settingsTabButton.Value.IsVisible <- true
        updateEmptyStateFromProjection editor.Value.SessionState.Workspace.ActiveDocumentId.IsSome

    let tryApplySettings save =
        match settingsView.Value.Draft with
        | None -> settingsView.Value.SetError("Settings draft is not initialized.")
        | Some draft ->
            match SettingsDraft.tryCreateSettings draft with
            | Error error -> settingsView.Value.SetError(error)
            | Ok settings ->
                let result =
                    if save then
                        saveSettingsCallback settings
                    else
                        Ok(applySettingsCallback settings)

                match result with
                | Ok() -> settingsView.Value.SetError("")
                | Error error -> settingsView.Value.SetError(error)

    let hasDiskWorkspace (workspace: WorkspaceModel) =
        match workspace.RootPath with
        | Some root -> Directory.Exists root
        | None -> false

    let workspaceTreeKey (workspace: WorkspaceModel) =
        let documents =
            workspace.Documents
            |> Map.toList
            |> List.choose (fun (_, documentState) ->
                let isDirty =
                    documentState.Editing.IsDirty || documentState.Document.Metadata.IsDirty

                if isDirty then
                    Some(
                        documentState.Document.Metadata.Path
                        |> Option.map DocumentModel.canonicalizePath,
                        documentState.Document.Metadata.Name,
                        isDirty
                    )
                else
                    None)

        match workspace.RootPath with
        | Some root -> Some(DocumentModel.canonicalizePath root), documents, None
        | None -> None, documents, Some workspace.TabOrder

    let startWorkspaceTreeLoad (workspace: WorkspaceModel) =
        workspaceTreeGeneration <- workspaceTreeGeneration + 1L
        let generation = workspaceTreeGeneration
        pendingWorkspaceTree <- Some workspace

        Task
            .Run(fun () -> WorkspaceFileTree.create workspace)
            .ContinueWith(fun (completed: Task<WorkspaceFileTreeNode>) ->
                if completed.Status = System.Threading.Tasks.TaskStatus.RanToCompletion then
                    Dispatcher.UIThread.Post(
                        Action(fun () ->
                            if generation = workspaceTreeGeneration then
                                pendingWorkspaceTree <- None
                                cachedWorkspace <- Some workspace
                                cachedWorkspaceTree <- Some completed.Result
                                requestRefresh ())
                    )
                    |> ignore)
        |> ignore

    let refresh () =
        refreshQueued <- false
        let editorControl = editor.Value
        let sessionState = editorControl.SessionState
        let treeKey = workspaceTreeKey sessionState.Workspace
        let tabs = WorkspaceProjection.tabs sessionState.Workspace

        let fileTree =
            match cachedWorkspace with
            | Some previous when workspaceTreeKey previous = treeKey -> cachedWorkspaceTree.Value
            | _ when
                pendingWorkspaceTree
                |> Option.exists (fun pending -> workspaceTreeKey pending = treeKey)
                ->
                WorkspaceFileTree.loading sessionState.Workspace
            | _ ->
                if hasDiskWorkspace sessionState.Workspace then
                    startWorkspaceTreeLoad sessionState.Workspace
                    WorkspaceFileTree.loading sessionState.Workspace
                else
                    workspaceTreeGeneration <- workspaceTreeGeneration + 1L
                    pendingWorkspaceTree <- None
                    let fileTree = WorkspaceFileTree.create sessionState.Workspace
                    cachedWorkspace <- Some sessionState.Workspace
                    cachedWorkspaceTree <- Some fileTree
                    fileTree

        let input = ShellProjection.fromEditorWithWorkspace editorControl tabs fileTree

        ShellView.applyModel
            this
            sidePanelHost.Value
            auxiliaryPanelHost.Value
            model
            input
            (fun documentId -> editor.Value.ActivateDocument(documentId))
            (fun path -> editor.Value.DispatchApplicationCommand(AppCommand.openDocument path))
        |> ignore

        updateToolRail ()

    let refreshOnUiThread () =
        if Dispatcher.UIThread.CheckAccess() then
            refresh ()
        elif not refreshQueued then
            refreshQueued <- true
            Dispatcher.UIThread.Post(Action refresh) |> ignore

    let invalidateWorkspaceTree () =
        workspaceTreeGeneration <- workspaceTreeGeneration + 1L
        pendingWorkspaceTree <- None
        cachedWorkspace <- None
        cachedWorkspaceTree <- None
        refreshOnUiThread ()

    let disposeSubscriptions () =
        subscriptions |> List.iter (fun subscription -> subscription.Dispose())
        subscriptions <- []
        workspaceTreeGeneration <- workspaceTreeGeneration + 1L
        pendingWorkspaceTree <- None

    let attachSubscriptions () =
        if subscriptions.IsEmpty then
            let editorControl = editor.Value

            let stateSubscription =
                editorControl.StateChanged.Subscribe(fun _ -> refreshOnUiThread ())

            let structureSubscription =
                editorControl.WorkspaceStructureChanged.Subscribe(fun _ -> invalidateWorkspaceTree ())

            subscriptions <- [ stateSubscription; structureSubscription ]

            refreshOnUiThread ()

    do
        this.InitializeComponent()
        requestRefresh <- refreshOnUiThread

        let editor = editor.Value
        let verticalScrollBar = verticalScrollBar.Value
        let horizontalScrollBar = horizontalScrollBar.Value

        editor.StatusChanged.Add(fun status ->
            if status.PendingAction.IsSome && not confirmationOpen then
                showDiscardDialog ())

        verticalScrollBar.ValueChanged.Add(fun args ->
            let offset = int (Math.Round(args.NewValue))

            if offset <> editor.VerticalOffset then
                editor.ScrollVerticalTo(offset))

        horizontalScrollBar.ValueChanged.Add(fun args ->
            let offset = int (Math.Round(args.NewValue))

            if offset <> editor.HorizontalOffset then
                editor.ScrollHorizontalTo(offset))

        welcomeView.Value.NewFileRequested.Add(fun _ -> editor.NewDocument())
        welcomeView.Value.OpenFileRequested.Add(fun _ -> editor.OpenFile())
        welcomeView.Value.OpenFolderRequested.Add(fun _ -> editor.DispatchApplicationCommand(AppCommand.openFolder))

        updateScrollBar ()
        updateEditorStatus editor.EditorStatus
        updateTabs editor.SessionState
        updateEmptyState editor.SessionState
        updateToolRailMetadata ()

        this.FindControl<Button>("WorkspaceToolButton").Click.Add(fun _ -> this.Dispatch(ToggleTool WorkspaceTool))

        this.FindControl<Button>("SearchToolButton").Click.Add(fun _ -> this.Dispatch(ToggleTool SearchTool))

        tabsPanel.Value.DocumentActivated.Add(fun documentId ->
            settingsOpen <- false
            editor.ActivateDocument(documentId)
            updateEmptyStateFromProjection editor.SessionState.Workspace.ActiveDocumentId.IsSome
            editor.Focus() |> ignore)

        tabsPanel.Value.DocumentCloseRequested.Add(fun documentId ->
            editor.ActivateDocument(documentId)
            editor.CloseDocument())

        sidePanelHost.Value.ResizeRequested.Add(fun width -> this.Dispatch(SetSidePanelWidth width))

        settingsTabButton.Value.Click.Add(fun _ -> openSettingsTab ())

        applySettingsButton.Value.Click.Add(fun _ -> tryApplySettings false)
        saveSettingsButton.Value.Click.Add(fun _ -> tryApplySettings true)
        closeSettingsButton.Value.Click.Add(fun _ -> closeSettingsTab ())

        this.AttachedToVisualTree.Add(fun _ -> attachSubscriptions ())
        this.DetachedFromVisualTree.Add(fun _ -> disposeSubscriptions ())

    member _.Editor = editor.Value

    member _.SessionState = editor.Value.SessionState

    member _.InvalidateWorkspaceTree() = invalidateWorkspaceTree ()

    member _.Layout: WorkspaceLayout =
        { SidePanelWidth = model.Layout.SidePanelWidth
          ActiveToolId =
            model.Layout.ActiveTool
            |> Option.map (ToolDescriptor.get >> fun descriptor -> descriptor.Id)
          IsToolPanelOpen = model.Layout.IsToolPanelOpen }

    member _.ApplyLayout(layout: WorkspaceLayout) =
        let widthMessage = SetSidePanelWidth layout.SidePanelWidth

        let toolMessage =
            layout.ActiveToolId
            |> Option.bind (fun toolId -> ToolDescriptor.all |> List.tryFind (fun descriptor -> descriptor.Id = toolId))
            |> Option.map (fun descriptor -> descriptor.Kind)
            |> fun tool -> RestoreTool(tool, layout.IsToolPanelOpen)

        model <-
            [ widthMessage; toolMessage ]
            |> List.fold (fun current message -> ShellUpdate.update message current |> fst) model

        refreshOnUiThread ()

    member _.ApplySettings(settings: AppSettings) =
        currentSettings <- settings
        editor.Value.ThemeSettings <- settings.Theme
        tabsPanel.Value.ApplyUiTheme(settings.Theme.Ui)

        match sidePanelHost.Value.PanelContent with
        | :? WorkspaceDocumentControl as workspace -> workspace.ApplyUiTheme(settings.Theme.Ui)
        | _ -> ()

        updateTabs editor.Value.SessionState

    member _.OpenSettings
        (settings: AppSettings, applySettings: AppSettings -> unit, saveSettings: AppSettings -> Result<unit, string>)
        =
        applySettingsCallback <- applySettings
        saveSettingsCallback <- saveSettings
        currentSettings <- settings
        openSettingsTab ()

    member _.ToggleSettings
        (settings: AppSettings, applySettings: AppSettings -> unit, saveSettings: AppSettings -> Result<unit, string>)
        =
        if settingsOpen then
            closeSettingsTab ()
        else
            applySettingsCallback <- applySettings
            saveSettingsCallback <- saveSettings
            currentSettings <- settings
            openSettingsTab ()

    member _.SetSettingsActions(applySettings: AppSettings -> unit, saveSettings: AppSettings -> Result<unit, string>) =
        applySettingsCallback <- applySettings
        saveSettingsCallback <- saveSettings

    interface IShellProjectionTarget with
        member _.ApplyShellInput(input) =
            updateTabsFromProjection input.Tabs
            updateEmptyStateFromProjection input.HasActiveDocument
            updateEditorStatus input.Status
            updateScrollBarFromProjection input.Scroll

    member _.Model = model

    member _.Dispatch(message: ShellMsg) =
        let updatedModel, effects = ShellUpdate.update message model
        model <- updatedModel

        effects
        |> List.iter (fun effect ->
            match effect with
            | DispatchAppCommand command -> editor.Value.DispatchApplicationCommand(command))

        refreshOnUiThread ()

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
