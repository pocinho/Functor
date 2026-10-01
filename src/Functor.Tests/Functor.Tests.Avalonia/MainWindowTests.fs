namespace Functor.Tests.Avalonia

open Avalonia
open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Input
open Avalonia.Interactivity
open Avalonia.Media
open Avalonia.Styling
open Avalonia.Threading
open System.Diagnostics
open Functor.Application
open Functor.Avalonia
open Functor.Avalonia.Controls
open Functor.Avalonia.Views
open Functor.Rendering
open Xunit

module MainWindowTests =
    let private invokeSettingsCommand (window: MainWindow) =
        let commandBar = window.FindControl<TextBox>("CommandBar")
        commandBar.Text <- "settings"

        let palette = window.FindControl<CommandPaletteView>("CommandPaletteView")
        let commandList = palette.FindControl<ListBox>("CommandList")

        commandList.SelectedIndex <-
            ShellCommands.all
            |> List.findIndex (fun descriptor -> descriptor.Id = "workbench.settings")

        palette.RaiseEvent(KeyEventArgs(RoutedEvent = InputElement.KeyDownEvent, Key = Key.Enter))

    [<AvaloniaFact>]
    let ``command bar is a live textbox that opens fuzzy recommendations`` () =
        let window = MainWindow()
        window.Show()

        let commandBar = window.FindControl<TextBox>("CommandBar")
        let watermark = window.FindControl<TextBlock>("CommandBarWatermark")
        Assert.True(watermark.IsVisible)
        commandBar.Text <- "stng"
        Dispatcher.UIThread.RunJobs()
        Assert.False(watermark.IsVisible)

        let palette = window.FindControl<CommandPaletteView>("CommandPaletteView")
        let commandList = palette.FindControl<ListBox>("CommandList")

        Assert.True(window.FindControl<Border>("CommandPaletteOverlay").IsVisible)
        Assert.Equal(1, commandList.ItemCount)

        Assert.Equal(
            Some "workbench.settings",
            palette.SelectedDescriptor |> Option.map (fun descriptor -> descriptor.Id)
        )

        commandBar.Text <- ""
        Dispatcher.UIThread.RunJobs()
        Assert.True(watermark.IsVisible)

        window.Close()

    [<AvaloniaFact>]
    let ``recent document menu follows close and clear lifecycle`` () =
        let window = MainWindow()
        window.Show()

        let editor = window.FindControl<ShellHostView>("ShellHostView").Editor
        let recentMenu = window.FindControl<MenuItem>("RecentDocumentsMenuItem")
        let reopenMenu = window.FindControl<MenuItem>("ReopenClosedTabMenuItem")
        let clearMenu = window.FindControl<MenuItem>("ClearRecentDocumentsMenuItem")

        Assert.False(recentMenu.IsEnabled)
        Assert.False(reopenMenu.IsEnabled)
        Assert.False(clearMenu.IsEnabled)

        editor.DispatchApplicationCommand(AppCommand.fileOpened "C:\\work\\file.fs" "text")
        editor.CloseDocument()

        Assert.Equal(1, recentMenu.Items.Count)
        Assert.True(recentMenu.IsEnabled)
        Assert.True(reopenMenu.IsEnabled)
        Assert.True(clearMenu.IsEnabled)

        clearMenu.RaiseEvent(RoutedEventArgs(MenuItem.ClickEvent))

        Assert.Empty(recentMenu.Items)
        Assert.False(recentMenu.IsEnabled)
        Assert.False(reopenMenu.IsEnabled)
        Assert.False(clearMenu.IsEnabled)

        window.Close()

    [<AvaloniaFact>]
    let ``settings tab visibility follows open and close lifecycle`` () =
        let window = MainWindow()
        window.Show()

        let shellHost = window.FindControl<ShellHostView>("ShellHostView")
        let settingsDocument = shellHost.FindControl<Grid>("SettingsDocument")
        let settingsMenu = window.FindControl<MenuItem>("SettingsMenuItem")

        settingsMenu.RaiseEvent(RoutedEventArgs(MenuItem.ClickEvent))
        Assert.True(settingsDocument.IsVisible)

        let settingsTabButton = shellHost.FindControl<Button>("SettingsTabButton")
        let closeSettingsButton = shellHost.FindControl<Button>("CloseSettingsButton")

        Assert.True(settingsTabButton.IsVisible)

        settingsMenu.RaiseEvent(RoutedEventArgs(MenuItem.ClickEvent))
        Assert.False(settingsDocument.IsVisible)
        Assert.False(settingsTabButton.IsVisible)

        settingsMenu.RaiseEvent(RoutedEventArgs(MenuItem.ClickEvent))
        Assert.True(settingsDocument.IsVisible)

        let welcomeView = shellHost.FindControl<WelcomeView>("WelcomeView")
        let newFileButton = welcomeView.FindControl<Button>("NewFileButton")
        newFileButton.RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        Assert.False(settingsDocument.IsVisible)
        Assert.True(settingsTabButton.IsVisible)

        let tabsPanel = shellHost.FindControl<DocumentListView>("TabsPanel")
        Assert.Equal(2, tabsPanel.TabCount)
        Assert.False(tabsPanel.GetTab(0).Classes.Contains("selected"))
        Assert.True(tabsPanel.GetTab(1).Classes.Contains("selected"))
        Assert.False(settingsTabButton.Classes.Contains("selected"))

        settingsMenu.RaiseEvent(RoutedEventArgs(MenuItem.ClickEvent))
        Assert.True(settingsDocument.IsVisible)

        closeSettingsButton.RaiseEvent(RoutedEventArgs(Button.ClickEvent))
        Assert.False(settingsDocument.IsVisible)
        Assert.False(settingsTabButton.IsVisible)

        window.Close()

    [<AvaloniaFact>]
    let ``theme manager switches runtime light and dark variants`` () =
        let window = Window()
        window.Show()

        let lightSettings =
            ThemeSettings.fromPaletteWithPresetAndUi "Graphite Light" Theme.graphiteLight UiThemeDefaults.defaultTheme
            |> AppSettings.fromTheme

        let darkSettings =
            ThemeSettings.fromPaletteWithPresetAndUi "Graphite Dark" Theme.dark UiThemeDefaults.defaultTheme
            |> AppSettings.fromTheme

        ThemeManager.apply Application.Current window lightSettings
        Assert.Equal(ThemeVariant.Light, window.RequestedThemeVariant)

        ThemeManager.apply Application.Current window darkSettings
        Assert.Equal(ThemeVariant.Dark, window.RequestedThemeVariant)

        window.Close()

    [<AvaloniaFact>]
    let ``reapplying a theme remains within the CI budget`` () =
        let window = Window()
        window.Show()

        let settings =
            ThemeSettings.fromPaletteWithPresetAndUi "Graphite Dark" Theme.dark UiThemeDefaults.defaultTheme
            |> AppSettings.fromTheme

        let stopwatch = Stopwatch.StartNew()

        for _ in 1..100 do
            ThemeManager.apply Application.Current window settings

        stopwatch.Stop()
        window.Close()

        Assert.True(stopwatch.ElapsedMilliseconds < 5000)

    [<AvaloniaFact>]
    let ``theme manager applies runtime visual resources`` () =
        let window = Window()
        window.Show()

        let palette =
            { Theme.dark with
                Background = 0xFF010203u }

        let ui =
            { UiThemeDefaults.defaultTheme with
                WorkspaceFontSize = 17.0
                EditorFontSize = 15.0
                ControlCornerRadius = 4.0 }

        let settings =
            ThemeSettings.fromPaletteWithPresetAndUi "Graphite Dark" palette ui
            |> AppSettings.fromTheme

        ThemeManager.apply Application.Current window settings

        let surfaceBrush =
            Application.Current.Resources["Theme.SurfaceBackgroundBrush"] :?> SolidColorBrush

        let semanticSurfaceBrush =
            Application.Current.Resources["Theme.SurfaceBackground"] :?> SolidColorBrush

        let cornerRadius =
            Application.Current.Resources["Theme.ControlCornerRadius"] :?> CornerRadius

        let workspaceFontSize =
            Application.Current.Resources["Theme.WorkspaceFontSize"] :?> float

        let editorFontSize = Application.Current.Resources["Theme.EditorFontSize"] :?> float

        Assert.Equal(Color.FromArgb(0xFFuy, 0x01uy, 0x02uy, 0x03uy), surfaceBrush.Color)
        Assert.Equal(surfaceBrush.Color, semanticSurfaceBrush.Color)
        Assert.Equal(CornerRadius(4.0), cornerRadius)
        Assert.Equal(17.0, workspaceFontSize)
        Assert.Equal(15.0, editorFontSize)

        window.Close()

    [<AvaloniaFact>]
    let ``theme manager publishes semantic control resources`` () =
        let window = Window()
        window.Show()

        let settings =
            ThemeSettings.fromPaletteWithPresetAndUi "Graphite Dark" Theme.dark UiThemeDefaults.defaultTheme
            |> AppSettings.fromTheme

        ThemeManager.apply Application.Current window settings

        [ "Theme.SurfaceBackground"
          "Theme.PanelBackground"
          "Theme.InputBackground"
          "Theme.TextPrimary"
          "Theme.TextMuted"
          "Theme.TextMutedOpacity"
          "Theme.Border"
          "Theme.Selection"
          "Theme.Hover"
          "Theme.Pressed"
          "Theme.Disabled"
          "Theme.Focus"
          "Theme.Accent"
          "Theme.Error"
          "Theme.Warning"
          "Theme.Information"
          "Theme.ResizeHandle"
          "Theme.Shadow"
          "Theme.ControlFontFamily"
          "Theme.IconFontFamily"
          "Theme.EditorFontFamily"
          "Theme.EditorFallbackFontFamily"
          "Theme.EditorFontSize"
          "Theme.WorkspaceFontSize"
          "Theme.CommandPaletteFontSize"
          "Theme.CommandPaletteWidth"
          "Theme.CommandPaletteOverlayMargin"
          "Theme.CommandPalettePadding"
          "Theme.CommandPaletteMaxHeight"
          "Theme.CommandPaletteItemMargin"
          "Theme.CommandPaletteGestureMargin"
          "Theme.WelcomeTitleFontSize"
          "Theme.ControlCornerRadius"
          "Theme.DocumentTabMinHeight"
          "Theme.DocumentTabPadding"
          "Theme.DocumentTabSpacing"
          "Theme.TabNavigationButtonWidth"
          "Theme.WorkspaceRowSpacing"
          "Theme.TitleBarHeight"
          "Theme.WindowControlWidth"
          "Theme.WindowControlPadding"
          "Theme.CommandBarWidth"
          "Theme.MinimumWindowWidth"
          "Theme.TitleBarHorizontalPadding"
          "Theme.SettingsControlWidth"
          "Theme.SettingsColorPickerWidth"
          "Theme.SettingsColorPickerHeight"
          "Theme.SettingsRowMargin"
          "Theme.SettingsPagePadding"
          "Theme.SettingsPageSpacing"
          "Theme.SettingsSectionHeadingMargin"
          "Theme.SettingsFirstHeadingMargin" ]
        |> List.iter (fun key -> Assert.NotNull(Application.Current.Resources[key]))

        window.Close()

    [<AvaloniaFact>]
    let ``themed dialog refreshes chrome when application resources change`` () =
        let settings =
            ThemeSettings.fromPaletteWithPresetAndUi "Graphite Dark" Theme.dark UiThemeDefaults.defaultTheme
            |> AppSettings.fromTheme

        let dialog, _ = ThemedDialogWindow.create settings "Test dialog" 320.0 120.0
        dialog.Show()

        let root = dialog.Content :?> Grid
        let titleBar = root.Children[0] :?> Border
        let initialColor = (titleBar.Background :?> SolidColorBrush).Color

        let updatedPalette =
            { Theme.dark with
                GutterBackground = 0xFF102030u }

        let updatedSettings =
            ThemeSettings.fromPaletteWithPresetAndUi "Graphite Dark" updatedPalette UiThemeDefaults.defaultTheme
            |> AppSettings.fromTheme

        ThemeManager.apply Application.Current dialog updatedSettings

        let updatedColor = (titleBar.Background :?> SolidColorBrush).Color

        Assert.NotEqual(initialColor, updatedColor)
        dialog.Close()
