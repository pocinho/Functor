namespace Functor.Avalonia.Views

open System
open Avalonia
open Avalonia.Controls
open Avalonia.Input
open Avalonia.Layout
open Avalonia.Media
open Avalonia.Markup.Xaml
open Avalonia.Threading
open Avalonia.VisualTree
open Functor.Avalonia
open Functor.Application
open Functor.Platform
open Functor.Rendering

type MainWindow() as this =
    inherit Window()

    let mutable shellState = ShellState.initial
    let mutable closeAfterDiscardConfirmation = false
    let titleBarDragSurface = lazy (this.FindControl<Border>("TitleBarDragSurface"))
    let shellHostView = lazy (this.FindControl<ShellHostView>("ShellHostView"))
    let editor = lazy shellHostView.Value.Editor
    let commandPaletteOverlay = lazy (this.FindControl<Border>("CommandPaletteOverlay"))

    let commandPaletteView =
        lazy (this.FindControl<CommandPaletteView>("CommandPaletteView"))

    let minimizeButton = lazy (this.FindControl<Button>("MinimizeButton"))
    let maximizeButton = lazy (this.FindControl<Button>("MaximizeButton"))
    let closeButton = lazy (this.FindControl<Button>("CloseButton"))
    let commandBar = lazy (this.FindControl<TextBox>("CommandBar"))
    let commandBarWatermark = lazy (this.FindControl<TextBlock>("CommandBarWatermark"))
    let mutable updatingCommandBar = false
    let commandRequested = Event<AppCommand>()

    let updateCommandBarWatermark () =
        commandBarWatermark.Value.IsVisible <- String.IsNullOrWhiteSpace commandBar.Value.Text

    let recentDocumentsMenuItem =
        lazy (this.FindControl<MenuItem>("RecentDocumentsMenuItem"))

    let clearRecentDocumentsMenuItem =
        lazy (this.FindControl<MenuItem>("ClearRecentDocumentsMenuItem"))

    let applyTheme (settings: AppSettings) =
        ThemeManager.apply Application.Current this settings

    let applySettings settings =
        shellState <- ShellState.withSettings settings shellState
        applyTheme settings
        shellHostView.Value.ApplySettings settings

    let loadSettings () =
        match Settings.tryReadText Settings.themeFilePath with
        | Ok text ->
            match AppSettingsLoader.loadText text with
            | Ok settings -> applySettings settings
            | Error error when error.StartsWith("Theme schema warning:", StringComparison.Ordinal) ->
                eprintfn "Warning: %s" error
            | Error error -> eprintfn "Unable to load theme settings: %s" error
        | Error _ -> ()

    let saveSettings settings =
        let json = AppSettingsLoader.toJson settings

        match Settings.tryWriteText Settings.themeFilePath json with
        | Ok() ->
            applySettings settings
            Ok()
        | Error error -> Error error

    let hasUnsavedChanges () =
        shellHostView.Value.SessionState.Workspace.Documents
        |> Map.exists (fun _ documentState -> documentState.Editing.IsDirty || documentState.Document.Metadata.IsDirty)

    let showCloseConfirmation () =
        match TopLevel.GetTopLevel(this) with
        | :? Window as owner ->
            let dialog, setContent =
                ThemedDialogWindow.create shellState.AppSettings "Unsaved changes" 420.0 160.0

            let layout = ThemeLayoutDensity.fromUiTheme shellState.AppSettings.Theme.Ui

            let message =
                TextBlock(
                    Text = "There are unsaved changes. Discard them and close Functor?",
                    TextWrapping = TextWrapping.Wrap
                )

            let discardButton = Button(Content = "Discard and Close")
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
                closeAfterDiscardConfirmation <- true
                dialog.Close()
                this.Close())

            cancelButton.Click.Add(fun _ -> dialog.Close())
            dialog.ShowDialog(owner) |> ignore
        | _ -> ()

    let updateRecentDocumentsMenu (state: AppSessionState) =
        let recentMenu = recentDocumentsMenuItem.Value
        recentMenu.Items.Clear()

        state.Workspace.RecentlyClosedDocuments
        |> List.iter (fun closed ->
            let item = MenuItem(Header = sprintf "%s (%s)" closed.Name closed.Path)

            item.Click.Add(fun _ -> commandRequested.Trigger(AppCommand.reopenRecentDocument closed.Path))

            recentMenu.Items.Add(item) |> ignore)

        recentMenu.IsEnabled <- not state.Workspace.RecentlyClosedDocuments.IsEmpty
        clearRecentDocumentsMenuItem.Value.IsEnabled <- not state.Workspace.RecentlyClosedDocuments.IsEmpty

    let updateRecentDocumentsMenuOnUiThread state =
        if Dispatcher.UIThread.CheckAccess() then
            updateRecentDocumentsMenu state
        else
            Dispatcher.UIThread.Post(Action(fun () -> updateRecentDocumentsMenu state))
            |> ignore

    let hideCommandPalette () =
        shellState <- ShellState.closeCommandPalette shellState
        commandPaletteOverlay.Value.IsVisible <- false
        updatingCommandBar <- true
        commandBar.Value.Text <- ""
        updatingCommandBar <- false
        updateCommandBarWatermark ()
        editor.Value.Focus() |> ignore

    let rec showCommandPalette () =
        commandPaletteView.Value.Configure(
            AppCommandCatalog.all,
            shellHostView.Value.SessionState,
            executeSelectedCommand
        )

        shellState <- ShellState.openCommandPalette shellState
        commandPaletteOverlay.Value.IsVisible <- true
        commandPaletteView.Value.SetQuery(commandBar.Value.Text)
        commandBar.Value.Focus() |> ignore

    and executeCommand (command: AppCommand) =
        match command with
        | OpenCommandPaletteRequested -> showCommandPalette ()
        | OpenSettingsRequested -> showSettingsDialog ()
        | NewDocumentRequested
        | OpenFileRequested
        | OpenDocumentRequested _
        | OpenFolderRequested ->
            shellHostView.Value.PrepareForDocumentNavigation()
            editor.Value.DispatchApplicationCommand(command)
        | other -> editor.Value.DispatchApplicationCommand(other)

    and executeDescriptor (descriptor: AppCommandDescriptor) =
        commandRequested.Trigger(descriptor.Command)

    and executeSelectedCommand () =
        match commandPaletteView.Value.SelectedDescriptor with
        | Some descriptor ->
            hideCommandPalette ()
            executeDescriptor descriptor
        | None -> ()

    and showSettingsDialog () =
        shellHostView.Value.ToggleSettings(shellState.AppSettings, applySettings, saveSettings)

    let executeCommandById id =
        AppCommandCatalog.tryFindById id |> Option.iter executeDescriptor

    let handleCommandBarTextChanged _ =
        updateCommandBarWatermark ()

        if not updatingCommandBar then
            if not shellState.IsCommandPaletteOpen then
                showCommandPalette ()
            else
                commandPaletteView.Value.SetQuery(commandBar.Value.Text)

    let toggleMaximize () =
        this.WindowState <-
            if this.WindowState = WindowState.Maximized then
                WindowState.Normal
            else
                WindowState.Maximized

    let updateMaximizeGlyph () =
        maximizeButton.Value.Content <-
            if this.WindowState = WindowState.Maximized then
                "\uE923"
            else
                "\uE922"

    do
        this.InitializeComponent()

        shellHostView.Value.SetSettingsActions(applySettings, saveSettings)
        commandRequested.Publish.Add(executeCommand)
        shellHostView.Value.CommandRequested.Add(commandRequested.Trigger)
        loadSettings ()

        this.FindControl<MenuItem>("NewMenuItem").Click.Add(fun _ -> executeCommandById "file.new")
        this.FindControl<MenuItem>("OpenMenuItem").Click.Add(fun _ -> executeCommandById "file.open")
        this.FindControl<MenuItem>("OpenFolderMenuItem").Click.Add(fun _ -> executeCommandById "workspace.openFolder")
        this.FindControl<MenuItem>("SaveMenuItem").Click.Add(fun _ -> executeCommandById "file.save")
        this.FindControl<MenuItem>("SaveAsMenuItem").Click.Add(fun _ -> executeCommandById "file.saveAs")

        this
            .FindControl<MenuItem>("ReopenClosedTabMenuItem")
            .Click.Add(fun _ -> executeCommandById "file.reopenClosedTab")

        this
            .FindControl<MenuItem>("ClearRecentDocumentsMenuItem")
            .Click.Add(fun _ -> executeCommandById "file.clearRecentDocuments")

        this.FindControl<MenuItem>("CloseMenuItem").Click.Add(fun _ -> executeCommandById "file.close")

        this
            .FindControl<MenuItem>("CommandPaletteMenuItem")
            .Click.Add(fun _ -> executeCommandById "workbench.commandPalette")

        this.FindControl<MenuItem>("SettingsMenuItem").Click.Add(fun _ -> executeCommandById "workbench.settings")

        this.Closing.Add(fun args ->
            if hasUnsavedChanges () && not closeAfterDiscardConfirmation then
                args.Cancel <- true
                showCloseConfirmation ())

        commandBar.Value.TextChanged.Add(handleCommandBarTextChanged)

        commandBar.Value.GotFocus.Add(fun _ ->
            if not shellState.IsCommandPaletteOpen then
                showCommandPalette ())

        commandBar.Value.LostFocus.Add(fun args ->
            match args.NewFocusedElement with
            | :? Visual as focusedVisual when commandPaletteOverlay.Value.IsVisualAncestorOf(focusedVisual) -> ()
            | _ when shellState.IsCommandPaletteOpen -> hideCommandPalette ()
            | _ -> ())

        commandBar.Value.KeyDown.Add(fun args ->
            if shellState.IsCommandPaletteOpen then
                match args.Key with
                | Key.Enter ->
                    executeSelectedCommand ()
                    args.Handled <- true
                | Key.Up ->
                    commandPaletteView.Value.MoveSelection(-1)
                    args.Handled <- true
                | Key.Down ->
                    commandPaletteView.Value.MoveSelection(1)
                    args.Handled <- true
                | _ -> ())

        updateCommandBarWatermark ()

        commandPaletteView.Value.CloseRequested.Add(fun _ -> hideCommandPalette ())
        editor.Value.StateChanged.Add(updateRecentDocumentsMenuOnUiThread)
        this.Activated.Add(fun _ -> shellHostView.Value.InvalidateWorkspaceTree())
        updateRecentDocumentsMenu shellHostView.Value.SessionState

        editor.Value.StateChanged.Add(fun state ->
            let update =
                fun () ->
                    let hasRecentDocuments = not state.Workspace.RecentlyClosedDocuments.IsEmpty
                    this.FindControl<MenuItem>("ReopenClosedTabMenuItem").IsEnabled <- hasRecentDocuments

            if Dispatcher.UIThread.CheckAccess() then
                update ()
            else
                Dispatcher.UIThread.Post(Action update) |> ignore)

        this.FindControl<MenuItem>("ReopenClosedTabMenuItem").IsEnabled <-
            not shellHostView.Value.SessionState.Workspace.RecentlyClosedDocuments.IsEmpty

        minimizeButton.Value.Click.Add(fun _ -> this.WindowState <- WindowState.Minimized)
        maximizeButton.Value.Click.Add(fun _ -> toggleMaximize ())
        closeButton.Value.Click.Add(fun _ -> this.Close())

        this.GetObservable(Window.WindowStateProperty).Subscribe(fun _ -> updateMaximizeGlyph ())
        |> ignore

        updateMaximizeGlyph ()

        titleBarDragSurface.Value.PointerPressed.Add(fun args ->
            if args.GetCurrentPoint(this).Properties.IsLeftButtonPressed then
                if args.ClickCount = 2 then
                    toggleMaximize ()
                else
                    this.BeginMoveDrag(args))

        this.KeyDown.Add(fun args ->
            if args.Key = Key.Escape && shellState.IsCommandPaletteOpen then
                hideCommandPalette ()
                args.Handled <- true
            else
                match Functor.Avalonia.InputAdapter.shellAction args.Key args.KeyModifiers with
                | Some Functor.Input.ShellAction.OpenCommandPalette ->
                    showCommandPalette ()
                    args.Handled <- true
                | Some Functor.Input.ShellAction.OpenSettings ->
                    showSettingsDialog ()
                    args.Handled <- true
                | None -> ())

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
