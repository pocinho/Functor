namespace Functor.Avalonia.Views

open Avalonia
open Avalonia.Controls
open Avalonia.Input
open Avalonia.Markup.Xaml
open Functor.Application
open Functor.Avalonia.ViewModels
open Functor.Platform

type MainWindow() as this =
    inherit Window()

    let viewModel = ShellViewModel()
    let titleBarDragSurface = lazy (this.FindControl<Border>("TitleBarDragSurface"))
    let mainView = lazy (this.FindControl<MainView>("MainView"))
    let commandPaletteOverlay = lazy (this.FindControl<Border>("CommandPaletteOverlay"))
    let commandPaletteView = lazy (this.FindControl<CommandPaletteView>("CommandPaletteView"))
    let minimizeButton = lazy (this.FindControl<Button>("MinimizeButton"))
    let maximizeButton = lazy (this.FindControl<Button>("MaximizeButton"))
    let closeButton = lazy (this.FindControl<Button>("CloseButton"))
    let commandCenterButton = lazy (this.FindControl<Button>("CommandCenterButton"))
    let mutable commandPaletteViewModel: CommandPaletteViewModel option = None

    let applySettings settings =
        viewModel.AppSettings <- settings
        mainView.Value.ApplySettings settings

    let loadSettings() =
        match Settings.tryReadText Settings.themeFilePath with
        | Ok text ->
            match AppSettingsLoader.loadText text with
            | Ok settings -> applySettings settings
            | Error _ -> ()
        | Error _ -> ()

    let saveSettings settings =
        let json = AppSettingsLoader.toJson settings

        match Settings.tryWriteText Settings.themeFilePath json with
        | Ok () ->
            applySettings settings
            Ok()
        | Error error -> Error error

    let hideCommandPalette() =
        viewModel.IsCommandPaletteOpen <- false
        commandPaletteOverlay.Value.IsVisible <- false
        mainView.Value.Editor.Focus() |> ignore

    let rec showCommandPalette() =
        let paletteViewModel = CommandPaletteViewModel(AppCommandCatalog.all, mainView.Value.SessionState)
        commandPaletteViewModel <- Some paletteViewModel
        commandPaletteView.Value.Configure(paletteViewModel, executeSelectedCommand)
        viewModel.IsCommandPaletteOpen <- true
        commandPaletteOverlay.Value.IsVisible <- true
        commandPaletteView.Value.FocusSearch()

    and executeCommand (command: AppCommand) =
        match command with
        | OpenCommandPaletteRequested -> showCommandPalette()
        | OpenSettingsRequested -> showSettingsDialog()
        | other -> mainView.Value.Editor.DispatchApplicationCommand(other)

    and executeDescriptor (descriptor: AppCommandDescriptor) =
        executeCommand descriptor.Command

    and executeSelectedCommand() =
        match commandPaletteViewModel |> Option.bind (fun model -> model.SelectedDescriptor) with
        | Some descriptor ->
            hideCommandPalette()
            executeDescriptor descriptor
        | None -> ()

    and showSettingsDialog() =
        let dialog = SettingsWindow(viewModel.AppSettings, applySettings, saveSettings)
        dialog.ShowDialog(this) |> ignore

    let executeCommandById id =
        AppCommandCatalog.tryFindById id
        |> Option.iter executeDescriptor

    let toggleMaximize() =
        this.WindowState <-
            if this.WindowState = WindowState.Maximized then
                WindowState.Normal
            else
                WindowState.Maximized

    do
        this.InitializeComponent()
        this.DataContext <- viewModel

        loadSettings()

        this.FindControl<MenuItem>("NewMenuItem").Click.Add(fun _ -> executeCommandById "file.new")
        this.FindControl<MenuItem>("OpenMenuItem").Click.Add(fun _ -> executeCommandById "file.open")
        this.FindControl<MenuItem>("SaveMenuItem").Click.Add(fun _ -> executeCommandById "file.save")
        this.FindControl<MenuItem>("SaveAsMenuItem").Click.Add(fun _ -> executeCommandById "file.saveAs")
        this.FindControl<MenuItem>("CloseMenuItem").Click.Add(fun _ -> executeCommandById "file.close")
        this.FindControl<MenuItem>("CommandPaletteMenuItem").Click.Add(fun _ -> executeCommandById "workbench.commandPalette")
        this.FindControl<MenuItem>("SettingsMenuItem").Click.Add(fun _ -> executeCommandById "workbench.settings")

        commandCenterButton.Value.Click.Add(fun _ -> showCommandPalette())
        commandPaletteView.Value.CloseRequested.Add(fun _ -> hideCommandPalette())

        minimizeButton.Value.Click.Add(fun _ -> this.WindowState <- WindowState.Minimized)
        maximizeButton.Value.Click.Add(fun _ -> toggleMaximize())
        closeButton.Value.Click.Add(fun _ -> this.Close())

        titleBarDragSurface.Value.PointerPressed.Add(fun args ->
            if args.GetCurrentPoint(this).Properties.IsLeftButtonPressed then
                if args.ClickCount = 2 then
                    toggleMaximize()
                else
                    this.BeginMoveDrag(args))

        this.KeyDown.Add(fun args ->
            let commandModifier =
                args.KeyModifiers.HasFlag(KeyModifiers.Control)
                || args.KeyModifiers.HasFlag(KeyModifiers.Meta)

            if args.Key = Key.Escape && viewModel.IsCommandPaletteOpen then
                hideCommandPalette()
                args.Handled <- true
            elif commandModifier && args.KeyModifiers.HasFlag(KeyModifiers.Shift) && args.Key = Key.P then
                showCommandPalette()
                args.Handled <- true
            elif commandModifier && args.Key = Key.OemComma then
                showSettingsDialog()
                args.Handled <- true)

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
