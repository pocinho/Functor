namespace Functor.Avalonia.Views

open System
open Avalonia
open Avalonia.Controls
open Avalonia.Input
open Avalonia.Markup.Xaml
open Avalonia.Media
open Avalonia.Styling
open Avalonia.Threading
open Functor.Application
open Functor.Platform

type MainWindow() as this =
    inherit Window()

    let mutable shellState = ShellState.initial
    let titleBarDragSurface = lazy (this.FindControl<Border>("TitleBarDragSurface"))
    let shellHostView = lazy (this.FindControl<ShellHostView>("ShellHostView"))
    let editor = lazy shellHostView.Value.Editor
    let commandPaletteOverlay = lazy (this.FindControl<Border>("CommandPaletteOverlay"))

    let commandPaletteView =
        lazy (this.FindControl<CommandPaletteView>("CommandPaletteView"))

    let minimizeButton = lazy (this.FindControl<Button>("MinimizeButton"))
    let maximizeButton = lazy (this.FindControl<Button>("MaximizeButton"))
    let closeButton = lazy (this.FindControl<Button>("CloseButton"))
    let commandCenterButton = lazy (this.FindControl<Button>("CommandCenterButton"))
    let applicationRoot = lazy (this.FindControl<Grid>("ApplicationRoot"))
    let titleBar = lazy (this.FindControl<Border>("TitleBar"))

    let recentDocumentsMenuItem =
        lazy (this.FindControl<MenuItem>("RecentDocumentsMenuItem"))

    let clearRecentDocumentsMenuItem =
        lazy (this.FindControl<MenuItem>("ClearRecentDocumentsMenuItem"))

    let colorFromArgb (argb: uint32) =
        Color.FromArgb(byte (argb >>> 24), byte (argb >>> 16), byte (argb >>> 8), byte argb)

    let applyTheme (settings: AppSettings) =
        let palette = settings.Theme.ThemeSource.Resolve()
        let background = SolidColorBrush(colorFromArgb palette.Background)
        let gutterBackground = SolidColorBrush(colorFromArgb palette.GutterBackground)
        let foreground = SolidColorBrush(colorFromArgb palette.Foreground)
        let borderColor = palette.GutterSeparator |> Option.defaultValue palette.Foreground
        let border = SolidColorBrush(colorFromArgb borderColor)

        this.RequestedThemeVariant <-
            if settings.Theme.Preset = "Graphite Light" then
                ThemeVariant.Light
            else
                ThemeVariant.Dark

        applicationRoot.Value.Background <- background
        this.Foreground <- foreground
        titleBar.Value.Background <- gutterBackground
        titleBar.Value.BorderBrush <- border
        commandCenterButton.Value.Background <- SolidColorBrush(colorFromArgb palette.Selection)
        commandCenterButton.Value.BorderBrush <- border
        commandCenterButton.Value.Foreground <- foreground

    let applySettings settings =
        shellState <- ShellState.withSettings settings shellState
        shellHostView.Value.ApplySettings settings
        applyTheme settings
        commandPaletteView.Value.ApplyTheme settings.Theme

    let loadSettings () =
        match Settings.tryReadText Settings.themeFilePath with
        | Ok text ->
            match AppSettingsLoader.loadText text with
            | Ok settings -> applySettings settings
            | Error _ -> ()
        | Error _ -> ()

    let saveSettings settings =
        let json = AppSettingsLoader.toJson settings

        match Settings.tryWriteText Settings.themeFilePath json with
        | Ok() ->
            applySettings settings
            Ok()
        | Error error -> Error error

    let updateRecentDocumentsMenu (state: AppSessionState) =
        let recentMenu = recentDocumentsMenuItem.Value
        recentMenu.Items.Clear()

        state.Workspace.RecentlyClosedDocuments
        |> List.iter (fun closed ->
            let item = MenuItem(Header = sprintf "%s (%s)" closed.Name closed.Path)

            item.Click.Add(fun _ ->
                editor.Value.DispatchApplicationCommand(AppCommand.reopenRecentDocument closed.Path))

            recentMenu.Items.Add(item) |> ignore)

        recentMenu.IsEnabled <- not state.Workspace.RecentlyClosedDocuments.IsEmpty
        clearRecentDocumentsMenuItem.Value.IsEnabled <- not state.Workspace.RecentlyClosedDocuments.IsEmpty

    let hideCommandPalette () =
        shellState <- ShellState.closeCommandPalette shellState
        commandPaletteOverlay.Value.IsVisible <- false
        editor.Value.Focus() |> ignore

    let rec showCommandPalette () =
        commandPaletteView.Value.ApplyTheme editor.Value.ThemeSettings

        commandPaletteView.Value.Configure(
            AppCommandCatalog.all,
            shellHostView.Value.SessionState,
            executeSelectedCommand
        )

        shellState <- ShellState.openCommandPalette shellState
        commandPaletteOverlay.Value.IsVisible <- true
        commandPaletteView.Value.FocusSearch()

    and executeCommand (command: AppCommand) =
        match command with
        | OpenCommandPaletteRequested -> showCommandPalette ()
        | OpenSettingsRequested -> showSettingsDialog ()
        | other -> editor.Value.DispatchApplicationCommand(other)

    and executeDescriptor (descriptor: AppCommandDescriptor) = executeCommand descriptor.Command

    and executeSelectedCommand () =
        match commandPaletteView.Value.SelectedDescriptor with
        | Some descriptor ->
            hideCommandPalette ()
            executeDescriptor descriptor
        | None -> ()

    and showSettingsDialog () =
        let dialog = SettingsWindow(shellState.AppSettings, applySettings, saveSettings)
        dialog.ShowDialog(this) |> ignore

    let executeCommandById id =
        AppCommandCatalog.tryFindById id |> Option.iter executeDescriptor

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

        commandCenterButton.Value.Click.Add(fun _ -> showCommandPalette ())
        commandPaletteView.Value.CloseRequested.Add(fun _ -> hideCommandPalette ())
        editor.Value.StateChanged.Add(updateRecentDocumentsMenu)
        this.Activated.Add(fun _ -> shellHostView.Value.InvalidateWorkspaceTree())
        updateRecentDocumentsMenu shellHostView.Value.SessionState

        editor.Value.StateChanged.Add(fun state ->
            let hasRecentDocuments = not state.Workspace.RecentlyClosedDocuments.IsEmpty
            this.FindControl<MenuItem>("ReopenClosedTabMenuItem").IsEnabled <- hasRecentDocuments)

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
