namespace Functor.Tests.Avalonia

open Avalonia
open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Input
open Avalonia.Interactivity
open Functor.Application
open Functor.Avalonia.Controls
open Functor.Avalonia.Rendering
open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Rendering
open Xunit

module EditorControlTests =

    [<AvaloniaFact>]
    let ``shifted cursor movement updates the keyboard selection`` () =
        let editor = EditorControl()
        editor.NewDocument()
        editor.DispatchApplicationCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "abc")))

        editor.RaiseEvent(
            KeyEventArgs(RoutedEvent = InputElement.KeyDownEvent, Key = Key.Left, KeyModifiers = KeyModifiers.Shift)
        )

        let expectedSelection: Selection option =
            Some
                { Start = { Line = 0; Column = 3 }
                  End = { Line = 0; Column = 2 } }

        Assert.Equal(expectedSelection, editor.SessionState.Model.Editing.Selection)

        editor.RaiseEvent(
            KeyEventArgs(RoutedEvent = InputElement.KeyDownEvent, Key = Key.Left, KeyModifiers = KeyModifiers.Shift)
        )

        let extendedSelection: Selection option =
            Some
                { Start = { Line = 0; Column = 3 }
                  End = { Line = 0; Column = 1 } }

        Assert.Equal(extendedSelection, editor.SessionState.Model.Editing.Selection)

        editor.RaiseEvent(KeyEventArgs(RoutedEvent = InputElement.KeyDownEvent, Key = Key.Left))

        Assert.Null(editor.SessionState.Model.Editing.Selection)

    [<AvaloniaFact>]
    let ``mouse wheel routes vertical and shifted horizontal scrolling with clamping`` () =
        let editor = EditorControl()
        editor.Arrange(Rect(0.0, 0.0, 160.0, 32.0))

        editor.DispatchApplicationCommand(
            AppCommand.toCoreEvent (
                LoadDocument(
                    "C:\\diagnostic\\scroll.fs",
                    String.concat
                        "\n"
                        [ "line 1 with enough horizontal content to require scrolling"
                          "line 2"
                          "line 3"
                          "line 4"
                          "line 5"
                          "line 6" ]
                )
            )
        )

        let verticalWheel =
            PointerWheelEventArgs(
                null,
                null,
                editor,
                Point(0.0, 0.0),
                0UL,
                Unchecked.defaultof<PointerPointProperties>,
                KeyModifiers.None,
                Vector(0.0, -2.0)
            )

        editor.RaiseEvent(verticalWheel)

        Assert.Equal(2, editor.VerticalOffset)
        Assert.Equal(0, editor.HorizontalOffset)
        Assert.True(verticalWheel.Handled)

        let horizontalWheel =
            PointerWheelEventArgs(
                null,
                null,
                editor,
                Point(0.0, 0.0),
                0UL,
                Unchecked.defaultof<PointerPointProperties>,
                KeyModifiers.Shift,
                Vector(0.0, -3.0)
            )

        editor.RaiseEvent(horizontalWheel)

        Assert.Equal(2, editor.VerticalOffset)
        Assert.Equal(3, editor.HorizontalOffset)
        Assert.True(horizontalWheel.Handled)

        editor.ScrollVerticalTo(System.Int32.MaxValue)
        editor.ScrollHorizontalTo(System.Int32.MaxValue)

        Assert.Equal(editor.VerticalScrollMaximum, editor.VerticalOffset)
        Assert.Equal(editor.HorizontalScrollMaximum, editor.HorizontalOffset)

    [<AvaloniaFact>]
    let ``opening a file while the editor was hidden behind the welcome view still renders every line`` () =
        let editor = EditorControl()
        let welcome = Functor.Avalonia.Views.WelcomeView()
        let panel = Panel()
        panel.Children.Add(editor)
        panel.Children.Add(welcome)
        let window = Window(Content = panel, Width = 800.0, Height = 600.0)
        window.Show()

        editor.CloseDocument()
        editor.IsVisible <- false
        welcome.IsVisible <- true

        editor.DispatchApplicationCommand(
            AppCommand.fileOpened "C:\\diagnostic\\multi.fs" "one\ntwo\nthree\nfour\nfive"
        )

        editor.IsVisible <- true
        welcome.IsVisible <- false

        let lineHeight = 16.0f

        let measurer =
            TextMeasurer.create (
                TextMetrics.createWithGraphemeAdvance
                    lineHeight
                    (RenderingSurface.measureDefaultAdvance lineHeight)
                    4
                    (RenderingSurface.measureGraphemeAdvance lineHeight)
            )

        let renderingConfig: RenderingPipeline.RenderingConfig = { Measurer = measurer }
        let frame = RenderingPipeline.render renderingConfig editor.SessionState.Model
        let visibleLineTexts = frame.VisibleLines |> List.map (fun line -> line.Text)

        Assert.Equal<string list>([ "one"; "two"; "three"; "four"; "five" ], editor.SessionState.Model.Editing.Buffer)
        Assert.Equal<string list>([ "one"; "two"; "three"; "four"; "five" ], visibleLineTexts)

    [<AvaloniaFact>]
    let ``a standalone space grapheme measures the same advance as a default character`` () =
        let lineHeight = 16.0f
        let defaultAdvance = RenderingSurface.measureDefaultAdvance lineHeight
        let spaceAdvance = RenderingSurface.measureGraphemeAdvance lineHeight 0 " "

        Assert.True(spaceAdvance > 0.0f)
        Assert.Equal(float defaultAdvance, float spaceAdvance, 3)

    [<AvaloniaFact>]
    let ``horizontal offset bounds are computed quickly for a large document`` () =
        let lineHeight = 16.0f

        let measurer =
            TextMeasurer.create (
                TextMetrics.createWithGraphemeAdvance
                    lineHeight
                    (RenderingSurface.measureDefaultAdvance lineHeight)
                    4
                    (RenderingSurface.measureGraphemeAdvance lineHeight)
            )

        let buffer =
            List.replicate 5000 "let value = someFunctionCall(argumentOne, argumentTwo, argumentThree)"

        let gutter = LayoutEngine.gutterWidth measurer buffer.Length

        let stopwatch = System.Diagnostics.Stopwatch.StartNew()
        let maximum = LayoutEngine.maxHorizontalOffset measurer 400.0f gutter buffer
        stopwatch.Stop()

        Assert.True(maximum > 0)

        Assert.True(
            stopwatch.ElapsedMilliseconds < 2000L,
            $"Expected cached measurement to complete quickly but took {stopwatch.ElapsedMilliseconds}ms"
        )

    [<AvaloniaFact>]
    let ``cursor advances to the end of the line after typing plain ascii text`` () =
        let editor = EditorControl()
        editor.DispatchApplicationCommand(AppCommand.toCoreEvent (LoadDocument("C:\\diagnostic\\sample.fs", "")))
        editor.DispatchApplicationCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "123")))

        let lineHeight = 16.0f

        let measurer =
            TextMeasurer.create (
                TextMetrics.createWithGraphemeAdvance
                    lineHeight
                    (RenderingSurface.measureDefaultAdvance lineHeight)
                    4
                    (RenderingSurface.measureGraphemeAdvance lineHeight)
            )

        let renderingConfig: RenderingPipeline.RenderingConfig = { Measurer = measurer }
        let frame = RenderingPipeline.render renderingConfig editor.SessionState.Model
        let cursor = frame.Cursors |> List.exactlyOne
        let firstRun = frame.TextRuns |> List.head

        Assert.Equal(3, cursor.Position.Column)
        Assert.Equal(firstRun.X + measurer.MeasureText "123", cursor.X)

    [<AvaloniaFact>]
    let ``cursor advances past a typed space to the true end of the line`` () =
        let editor = EditorControl()
        editor.DispatchApplicationCommand(AppCommand.toCoreEvent (LoadDocument("C:\\diagnostic\\sample.fs", "")))

        for ch in "123 456" do
            editor.DispatchApplicationCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertChar ch)))

        let lineHeight = 16.0f

        let measurer =
            TextMeasurer.create (
                TextMetrics.createWithGraphemeAdvance
                    lineHeight
                    (RenderingSurface.measureDefaultAdvance lineHeight)
                    4
                    (RenderingSurface.measureGraphemeAdvance lineHeight)
            )

        let renderingConfig: RenderingPipeline.RenderingConfig = { Measurer = measurer }
        let frame = RenderingPipeline.render renderingConfig editor.SessionState.Model
        let cursor = frame.Cursors |> List.exactlyOne
        let firstRun = frame.TextRuns |> List.exactlyOne

        Assert.Equal(7, editor.SessionState.Model.Editing.Cursor.Column)
        Assert.Equal(firstRun.X + measurer.MeasureText "123 456", cursor.X)

    [<AvaloniaFact>]
    let ``tokenized runs separated by spaces do not overlap`` () =
        let editor = EditorControl()

        editor.DispatchApplicationCommand(
            AppCommand.toCoreEvent (LoadDocument("C:\\diagnostic\\sample.fs", "let value = 42"))
        )

        let document = editor.SessionState.Model.ActiveDocument.Value

        let tokenizeResult =
            Functor.Application.DefaultTokenizerService() :> Functor.Application.ITokenizerService
            |> fun service ->
                service.Tokenize(
                    { DocumentId = document.Id
                      Revision = editor.SessionState.Model.Editing.Revision
                      Language = "fsharp"
                      Scope = FullDocument
                      Lines = editor.SessionState.Model.Editing.Buffer
                      InitialState = Functor.Application.Initial },
                    System.Threading.CancellationToken.None
                )
                |> Async.RunSynchronously

        let output =
            match tokenizeResult with
            | Ok value -> value
            | Error message -> failwith message

        editor.DispatchApplicationCommand(
            AppCommand.tokenizationCompleted
                { DocumentId = document.Id
                  Revision = editor.SessionState.Model.Editing.Revision
                  Scope = FullDocument
                  Provider = output.Provider
                  Layer = output.Layer
                  Tokens = output.Tokens
                  Snapshots = output.Snapshots
                  FinalState = output.FinalState }
        )

        let lineHeight = 16.0f

        let measurer =
            TextMeasurer.create (
                TextMetrics.createWithGraphemeAdvance
                    lineHeight
                    (RenderingSurface.measureDefaultAdvance lineHeight)
                    4
                    (RenderingSurface.measureGraphemeAdvance lineHeight)
            )

        let renderingConfig: RenderingPipeline.RenderingConfig = { Measurer = measurer }
        let frame = RenderingPipeline.render renderingConfig editor.SessionState.Model
        let runXPositions = frame.TextRuns |> List.map (fun run -> run.X)

        Assert.Equal(7, frame.TextRuns.Length)
        Assert.Equal<float32 list>(runXPositions |> List.distinct, runXPositions)

    [<AvaloniaFact>]
    let ``emoji hit testing returns utf16 boundary after the rendered grapheme`` () =
        let editor = EditorControl()
        editor.NewDocument()
        editor.DispatchApplicationCommand(AppCommand.toCoreEvent (ApplyEditingEvent(InsertString "a🚧b")))

        let lineHeight = 16.0f
        let defaultAdvance = RenderingSurface.measureDefaultAdvance lineHeight
        let emojiAdvance = RenderingSurface.measureGraphemeAdvance lineHeight 1 "🚧"

        let gutter =
            LayoutEngine.gutterWidth (TextMeasurer.create (TextMetrics.create lineHeight defaultAdvance 4)) 1

        let point = Point(float (gutter + defaultAdvance + (emojiAdvance * 0.75f)), 8.0)

        let position = editor.DocumentPositionAtPoint point

        Assert.Equal({ Line = 0; Column = 3 }, position)
        Assert.True(emojiAdvance > 0.0f)

    [<AvaloniaFact>]
    let ``emoji cursor spacing is wider than the preceding ascii cell when measured`` () =
        let lineHeight = 16.0f
        let defaultAdvance = RenderingSurface.measureDefaultAdvance lineHeight
        let emojiAdvance = RenderingSurface.measureGraphemeAdvance lineHeight 1 "🚧"

        Assert.True(defaultAdvance > 0.0f)
        Assert.True(emojiAdvance > 0.0f)
