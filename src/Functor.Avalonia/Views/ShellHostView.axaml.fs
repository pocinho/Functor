namespace Functor.Avalonia.Views

open System
open Avalonia
open Avalonia.Controls
open Avalonia.Layout
open Avalonia.Media
open Avalonia.Markup.Xaml
open Avalonia.Threading
open Functor.Application
open Functor.Avalonia
open Functor.Avalonia.Controls
open Functor.Domain.Document
open Functor.Platform
open Functor.Workspace

type ShellHostView() as this =
    inherit UserControl()

    let editor = lazy (this.FindControl<EditorControl>("EditorControl"))
    let welcomeView = lazy (this.FindControl<WelcomeView>("WelcomeView"))
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
    let exportThemeButton = lazy (this.FindControl<Button>("ExportThemeButton"))
    let closeSettingsButton = lazy (this.FindControl<Button>("CloseSettingsButton"))

    let auxiliaryPanelHost =
        lazy (this.FindControl<SidePanelControl>("AuxiliaryPanelHost"))

    let mutable model = ShellModel.initial
    let mutable subscriptions: IDisposable list = []
    let mutable confirmationOpen = false
    let mutable refreshQueued = false
    let mutable requestRefresh: unit -> unit = ignore
    let mutable settingsOpen = false
    let mutable settingsActive = false
    let mutable currentSettings = AppSettings.defaults
    let mutable applySettingsCallback: AppSettings -> unit = ignore
    let commandRequested = Event<AppCommand>()

    let mutable saveSettingsCallback: AppSettings -> Result<unit, string> =
        fun _ -> Ok()

    let workspaceTreeCoordinator =
        WorkspaceTreeCoordinator(
            WorkspaceFileTree.create,
            (fun action -> Dispatcher.UIThread.Post(Action action) |> ignore),
            (fun () -> requestRefresh ())
        )

    let updateEmptyStateFromProjection hasActiveDocument =
        editor.Value.IsVisible <- not settingsActive && hasActiveDocument
        welcomeView.Value.IsVisible <- not settingsActive && not hasActiveDocument
        settingsDocument.Value.IsVisible <- settingsActive
        settingsTabButton.Value.Classes.Set("selected", settingsActive)

    let settingsCoordinator =
        lazy
            (SettingsCoordinator(
                settingsView.Value,
                settingsTabButton.Value,
                settingsDocument.Value,
                (fun () -> editor.Value.SessionState.Workspace.ActiveDocumentId.IsSome),
                updateEmptyStateFromProjection,
                (fun active -> settingsActive <- active),
                (fun () -> editor.Value.Focus() |> ignore)
            ))

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

            let layout = ThemeLayoutDensity.fromUiTheme currentSettings.Theme.Ui

            let message =
                TextBlock(Text = "This document has unsaved changes. Discard them?", TextWrapping = TextWrapping.Wrap)

            let discardButton = Button(Content = "Discard")
            let cancelButton = Button(Content = "Cancel")

            let buttons =
                StackPanel(Orientation = Orientation.Horizontal, Spacing = layout.DialogButtonSpacing)

            let content =
                StackPanel(Spacing = layout.DialogContentSpacing, Margin = Thickness(layout.DialogContentPadding))

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

    let closeSettingsTab () = settingsCoordinator.Value.Close()

    let closeSettingsForDocumentNavigation () =
        settingsCoordinator.Value.CloseForDocumentNavigation()

    let openSettingsTab () =
        settingsCoordinator.Value.Configure(currentSettings, applySettingsCallback, saveSettingsCallback)
        settingsCoordinator.Value.Open()

    let tryApplySettings save =
        settingsCoordinator.Value.TryApply(save)

    let tryExportTheme () =
        settingsCoordinator.Value.TryExportTheme()

    let refresh () =
        refreshQueued <- false
        let editorControl = editor.Value
        let sessionState = editorControl.SessionState
        let tabs = WorkspaceProjection.tabs sessionState.Workspace
        let fileTree = workspaceTreeCoordinator.GetTree sessionState.Workspace

        let input = ShellProjection.fromEditorWithWorkspace editorControl tabs fileTree

        ShellView.applyModel
            this
            sidePanelHost.Value
            auxiliaryPanelHost.Value
            model
            input
            (fun documentId ->
                closeSettingsForDocumentNavigation ()
                editor.Value.ActivateDocument(documentId))
            (fun path -> commandRequested.Trigger(AppCommand.openDocument path))
            commandRequested.Trigger
        |> ignore

        updateToolRail ()

    let refreshOnUiThread () =
        if Dispatcher.UIThread.CheckAccess() then
            refresh ()
        elif not refreshQueued then
            refreshQueued <- true
            Dispatcher.UIThread.Post(Action refresh) |> ignore

    let invalidateWorkspaceTree () =
        workspaceTreeCoordinator.Invalidate()
        refreshOnUiThread ()

    let disposeSubscriptions () =
        subscriptions |> List.iter (fun subscription -> subscription.Dispose())
        subscriptions <- []
        workspaceTreeCoordinator.Dispose()

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

        editor.StatusChanged.Add(fun status ->
            if status.PendingAction.IsSome && not confirmationOpen then
                showDiscardDialog ())

        welcomeView.Value.NewFileRequested.Add(fun _ -> commandRequested.Trigger(AppCommand.newDocument))

        welcomeView.Value.OpenFileRequested.Add(fun _ -> commandRequested.Trigger(AppCommand.openFile))

        welcomeView.Value.OpenFolderRequested.Add(fun _ -> commandRequested.Trigger(AppCommand.openFolder))

        updateEditorStatus editor.EditorStatus
        updateTabs editor.SessionState
        updateEmptyState editor.SessionState
        updateToolRailMetadata ()

        this.FindControl<Button>("WorkspaceToolButton").Click.Add(fun _ -> this.Dispatch(ToggleTool WorkspaceTool))

        this.FindControl<Button>("SearchToolButton").Click.Add(fun _ -> this.Dispatch(ToggleTool SearchTool))

        tabsPanel.Value.DocumentActivated.Add(fun documentId ->
            closeSettingsForDocumentNavigation ()
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
        exportThemeButton.Value.Click.Add(fun _ -> tryExportTheme ())
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
        if settingsActive then
            closeSettingsTab ()
        else
            applySettingsCallback <- applySettings
            saveSettingsCallback <- saveSettings
            currentSettings <- settings
            openSettingsTab ()

    member _.SetSettingsActions(applySettings: AppSettings -> unit, saveSettings: AppSettings -> Result<unit, string>) =
        applySettingsCallback <- applySettings
        saveSettingsCallback <- saveSettings

    member _.CommandRequested = commandRequested.Publish

    member _.PrepareForDocumentNavigation() = closeSettingsForDocumentNavigation ()

    interface IShellProjectionTarget with
        member _.ApplyShellInput(input) =
            updateTabsFromProjection input.Tabs
            updateEmptyStateFromProjection input.HasActiveDocument
            updateEditorStatus input.Status

    member _.Model = model

    member _.Dispatch(message: ShellMsg) =
        let updatedModel, effects = ShellUpdate.update message model
        model <- updatedModel

        effects
        |> List.iter (fun effect ->
            match effect with
            | DispatchAppCommand command -> commandRequested.Trigger(command))

        refreshOnUiThread ()

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
