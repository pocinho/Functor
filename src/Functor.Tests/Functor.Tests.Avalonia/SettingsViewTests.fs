namespace Functor.Tests.Avalonia

open System
open System.Globalization
open System.IO
open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Media
open Avalonia.Threading
open Functor.Application
open Functor.Avalonia.Views
open Functor.Platform
open Xunit

module SettingsViewTests =
    [<AvaloniaFact>]
    let ``configure projects settings into controls and draft`` () =
        let view = SettingsView()
        view.Configure(AppSettings.defaults)

        let background = view.FindControl<TextBox>("Background")
        let tabCloseIconSize = view.FindControl<NumericUpDown>("TabCloseIconSize")
        let editorFontSize = view.FindControl<NumericUpDown>("EditorFontSize")
        let gutterPadding = view.FindControl<NumericUpDown>("GutterPadding")
        let gutterMinimumWidth = view.FindControl<NumericUpDown>("GutterMinimumWidth")
        let documentTabMinHeight = view.FindControl<NumericUpDown>("DocumentTabMinHeight")

        let documentTabCloseButtonSize =
            view.FindControl<NumericUpDown>("DocumentTabCloseButtonSize")

        let documentTabPaddingHorizontal =
            view.FindControl<NumericUpDown>("DocumentTabPaddingHorizontal")

        let documentTabPaddingVertical =
            view.FindControl<NumericUpDown>("DocumentTabPaddingVertical")

        let documentTabSpacing = view.FindControl<NumericUpDown>("DocumentTabSpacing")

        let expectedBackground =
            (SettingsForm.fromAppSettings AppSettings.defaults).Background

        Assert.Equal(expectedBackground, background.Text)

        let numericValue (control: NumericUpDown) =
            control.Value.Value.ToString(CultureInfo.InvariantCulture)

        Assert.Equal((SettingsForm.fromAppSettings AppSettings.defaults).TabCloseIconSize, numericValue tabCloseIconSize)
        Assert.Equal((SettingsForm.fromAppSettings AppSettings.defaults).EditorFontSize, numericValue editorFontSize)
        Assert.Equal((SettingsForm.fromAppSettings AppSettings.defaults).GutterPadding, numericValue gutterPadding)

        Assert.Equal(
            (SettingsForm.fromAppSettings AppSettings.defaults).GutterMinimumWidth,
            numericValue gutterMinimumWidth
        )

        Assert.Equal(
            (SettingsForm.fromAppSettings AppSettings.defaults).DocumentTabMinHeight,
            numericValue documentTabMinHeight
        )

        Assert.Equal(
            (SettingsForm.fromAppSettings AppSettings.defaults).DocumentTabCloseButtonSize,
            numericValue documentTabCloseButtonSize
        )

        Assert.Equal(
            (SettingsForm.fromAppSettings AppSettings.defaults).DocumentTabPaddingHorizontal,
            numericValue documentTabPaddingHorizontal
        )

        Assert.Equal(
            (SettingsForm.fromAppSettings AppSettings.defaults).DocumentTabPaddingVertical,
            numericValue documentTabPaddingVertical
        )

        Assert.Equal(
            (SettingsForm.fromAppSettings AppSettings.defaults).DocumentTabSpacing,
            numericValue documentTabSpacing
        )

        Assert.Equal(Some(SettingsForm.fromAppSettings AppSettings.defaults), view.Form)

    [<AvaloniaFact>]
    let ``editing a control updates the draft`` () =
        let view = SettingsView()
        let window = Window(Content = view)
        window.Show()
        view.Configure(AppSettings.defaults)
        let background = view.FindControl<TextBox>("Background")
        let expectedBackground = "#FF112233"

        background.Text <- expectedBackground
        Dispatcher.UIThread.RunJobs()

        Assert.Equal(Some expectedBackground, view.Form |> Option.map (fun draft -> draft.Background))

        let gutterPadding = view.FindControl<NumericUpDown>("GutterPadding")
        let gutterMinimumWidth = view.FindControl<NumericUpDown>("GutterMinimumWidth")
        gutterPadding.Value <- Nullable 6M
        gutterMinimumWidth.Value <- Nullable 20M
        Dispatcher.UIThread.RunJobs()

        Assert.Equal(Some "6", view.Form |> Option.map (fun draft -> draft.GutterPadding))
        Assert.Equal(Some "20", view.Form |> Option.map (fun draft -> draft.GutterMinimumWidth))

        let documentTabMinHeight = view.FindControl<NumericUpDown>("DocumentTabMinHeight")

        let documentTabCloseButtonSize =
            view.FindControl<NumericUpDown>("DocumentTabCloseButtonSize")

        let documentTabPaddingHorizontal =
            view.FindControl<NumericUpDown>("DocumentTabPaddingHorizontal")

        let documentTabPaddingVertical =
            view.FindControl<NumericUpDown>("DocumentTabPaddingVertical")

        let documentTabSpacing = view.FindControl<NumericUpDown>("DocumentTabSpacing")
        documentTabMinHeight.Value <- Nullable 30M
        documentTabCloseButtonSize.Value <- Nullable 24M
        documentTabPaddingHorizontal.Value <- Nullable 12M
        documentTabPaddingVertical.Value <- Nullable 5M
        documentTabSpacing.Value <- Nullable 3M
        Dispatcher.UIThread.RunJobs()

        let draft = view.Form.Value
        Assert.Equal("30", draft.DocumentTabMinHeight)
        Assert.Equal("24", draft.DocumentTabCloseButtonSize)
        Assert.Equal("12", draft.DocumentTabPaddingHorizontal)
        Assert.Equal("5", draft.DocumentTabPaddingVertical)
        Assert.Equal("3", draft.DocumentTabSpacing)

    [<AvaloniaFact>]
    let ``editing editor line height synchronizes editor font size`` () =
        let view = SettingsView()
        let window = Window(Content = view)
        window.Show()
        view.Configure(AppSettings.defaults)

        view.FindControl<NumericUpDown>("EditorLineHeight").Value <- Nullable 18M
        Dispatcher.UIThread.RunJobs()

        Assert.Equal(15M, view.FindControl<NumericUpDown>("EditorFontSize").Value.Value)
        Assert.Equal(Some "15", view.Form |> Option.map (fun draft -> draft.EditorFontSize))

    [<AvaloniaFact>]
    let ``editing editor font size synchronizes editor line height`` () =
        let view = SettingsView()
        let window = Window(Content = view)
        window.Show()
        view.Configure(AppSettings.defaults)

        view.FindControl<NumericUpDown>("EditorFontSize").Value <- Nullable 15M
        Dispatcher.UIThread.RunJobs()

        Assert.Equal(18M, view.FindControl<NumericUpDown>("EditorLineHeight").Value.Value)
        Assert.Equal(Some "18", view.Form |> Option.map (fun draft -> draft.EditorLineHeight))

    [<AvaloniaFact>]
    let ``color picker updates the hex field and draft`` () =
        let view = SettingsView()
        let window = Window(Content = view)
        window.Show()
        view.Configure(AppSettings.defaults)
        let picker = view.FindControl<ColorPicker>("BackgroundPicker")

        picker.Color <- Color.FromArgb(0xFFuy, 0x11uy, 0x22uy, 0x33uy)
        Dispatcher.UIThread.RunJobs()

        Assert.Equal("#FF112233", view.FindControl<TextBox>("Background").Text)
        Assert.Equal(Some "#FF112233", view.Form |> Option.map (fun draft -> draft.Background))

    [<AvaloniaFact>]
    let ``font lookup combo updates the draft`` () =
        let view = SettingsView()
        let window = Window(Content = view)
        window.Show()
        view.Configure(AppSettings.defaults)
        let editorFont = view.FindControl<ComboBox>("EditorFontFamily")

        editorFont.Text <- "Cascadia Code"
        Dispatcher.UIThread.RunJobs()

        Assert.Equal(Some "Cascadia Code", view.Form |> Option.map (fun draft -> draft.EditorFontFamily))

    [<AvaloniaFact>]
    let ``selecting a preset updates the draft without recursive control edits`` () =
        let view = SettingsView()
        view.Configure(AppSettings.defaults)
        let preset = view.FindControl<ComboBox>("ThemePreset")

        preset.SelectedItem <- "Graphite Light"

        let draft = view.Form.Value
        Assert.Equal("Graphite Light", draft.ThemePreset)
        Assert.Equal("#FFE7E5EA", draft.Background)

    [<AvaloniaFact>]
    let ``reconfiguring the view discards the previous draft`` () =
        let view = SettingsView()
        view.Configure(AppSettings.defaults)
        let background = view.FindControl<TextBox>("Background")
        background.Text <- "#FF112233"

        view.Configure(AppSettings.defaults)

        Assert.Equal(Some(SettingsForm.fromAppSettings AppSettings.defaults), view.Form)
        Assert.Equal((SettingsForm.fromAppSettings AppSettings.defaults).Background, background.Text)

    [<Fact>]
    let ``theme catalog loads only valid functortheme files`` () =
        let directory = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"))
        Directory.CreateDirectory(directory) |> ignore

        try
            let validTheme =
                "{ \"theme\": { \"schemaVersion\": 1, \"preset\": \"Graphite Light\", \"foreground\": \"#FF010203\" } }"

            File.WriteAllText(Path.Combine(directory, "Ocean.functortheme"), validTheme)
            File.WriteAllText(Path.Combine(directory, "Ignored.json"), validTheme)
            File.WriteAllText(Path.Combine(directory, "Broken.functortheme"), "{ invalid")

            let themes = ThemeCatalog.loadFromDirectory directory

            Assert.Single(themes) |> ignore
            Assert.Equal("Ocean", themes.Head.Name)
            Assert.Equal(0xFF010203u, themes.Head.Settings.Theme.ThemeSource.Resolve().Foreground)
        finally
            Directory.Delete(directory, true)

    [<Fact>]
    let ``theme catalog exports new names and rejects duplicates`` () =
        let directory = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"))
        Directory.CreateDirectory(directory) |> ignore

        try
            match ThemeCatalog.exportToDirectory directory "Ocean" AppSettings.defaults with
            | Ok() -> ()
            | Error error -> Assert.Fail(error)

            match ThemeCatalog.exportToDirectory directory "ocean" AppSettings.defaults with
            | Error error -> Assert.Equal("A theme named 'ocean' already exists.", error)
            | Ok() -> Assert.Fail("Duplicate theme export unexpectedly succeeded.")

            Assert.True(File.Exists(Path.Combine(directory, "Ocean.functortheme")))
        finally
            Directory.Delete(directory, true)
