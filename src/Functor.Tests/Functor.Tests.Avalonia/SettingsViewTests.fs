namespace Functor.Tests.Avalonia

open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Media
open Avalonia.Threading
open Functor.Application
open Functor.Avalonia.Views
open Xunit

module SettingsViewTests =
    [<AvaloniaFact>]
    let ``configure projects settings into controls and draft`` () =
        let view = SettingsView()
        view.Configure(AppSettings.defaults)

        let background = view.FindControl<TextBox>("Background")
        let tabCloseIconSize = view.FindControl<TextBox>("TabCloseIconSize")
        let editorFontSize = view.FindControl<TextBox>("EditorFontSize")

        let expectedBackground =
            (SettingsDraft.fromSettings AppSettings.defaults).Background

        Assert.Equal(expectedBackground, background.Text)
        Assert.Equal((SettingsDraft.fromSettings AppSettings.defaults).TabCloseIconSize, tabCloseIconSize.Text)
        Assert.Equal((SettingsDraft.fromSettings AppSettings.defaults).EditorFontSize, editorFontSize.Text)
        Assert.Equal(Some(SettingsDraft.fromSettings AppSettings.defaults), view.Draft)

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

        Assert.Equal(Some expectedBackground, view.Draft |> Option.map (fun draft -> draft.Background))

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
        Assert.Equal(Some "#FF112233", view.Draft |> Option.map (fun draft -> draft.Background))

    [<AvaloniaFact>]
    let ``font lookup combo updates the draft`` () =
        let view = SettingsView()
        let window = Window(Content = view)
        window.Show()
        view.Configure(AppSettings.defaults)
        let editorFont = view.FindControl<ComboBox>("EditorFontFamily")

        editorFont.Text <- "Cascadia Code"
        Dispatcher.UIThread.RunJobs()

        Assert.Equal(Some "Cascadia Code", view.Draft |> Option.map (fun draft -> draft.EditorFontFamily))

    [<AvaloniaFact>]
    let ``selecting a preset updates the draft without recursive control edits`` () =
        let view = SettingsView()
        view.Configure(AppSettings.defaults)
        let preset = view.FindControl<ComboBox>("ThemePreset")

        preset.SelectedItem <- "Graphite Light"

        let draft = view.Draft.Value
        Assert.Equal("Graphite Light", draft.ThemePreset)
        Assert.Equal("#FFE7E5EA", draft.Background)

    [<AvaloniaFact>]
    let ``reconfiguring the view discards the previous draft`` () =
        let view = SettingsView()
        view.Configure(AppSettings.defaults)
        let background = view.FindControl<TextBox>("Background")
        background.Text <- "#FF112233"

        view.Configure(AppSettings.defaults)

        Assert.Equal(Some(SettingsDraft.fromSettings AppSettings.defaults), view.Draft)
        Assert.Equal((SettingsDraft.fromSettings AppSettings.defaults).Background, background.Text)
