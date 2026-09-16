namespace Functor.Tests.Avalonia

open Avalonia
open Avalonia.Input
open Functor.Avalonia
open Functor.Input
open Xunit

module InputAdapterTests =
    [<Fact>]
    let ``pointer translation preserves position button state and modifiers`` () =
        let input =
            InputAdapter.pointerInput
                (Point(12.5, 24.0))
                (Some MouseButton.Right)
                true
                Avalonia.Input.KeyModifiers.Shift

        Assert.Equal(12.5, input.Position.X)
        Assert.Equal(24.0, input.Position.Y)
        Assert.Equal(Some PointerButton.Right, input.Button)
        Assert.True(input.IsPressed)
        Assert.Equal(Functor.Input.KeyModifiers.Shift, input.Modifiers)

    [<Fact>]
    let ``text translation ignores empty input`` () =
        Assert.Equal(None, InputAdapter.textInput "")
        Assert.Equal(Some { Text = "hello" }, InputAdapter.textInput "hello")

    [<Fact>]
    let ``unsupported and modifier-only keys produce no actions`` () =
        Assert.Equal(None, InputAdapter.editingEvent Avalonia.Input.Key.F Avalonia.Input.KeyModifiers.None)
        Assert.Equal(None, InputAdapter.shellAction Avalonia.Input.Key.Left Avalonia.Input.KeyModifiers.Control)
        Assert.Equal(None, InputAdapter.editorAction Avalonia.Input.Key.Left Avalonia.Input.KeyModifiers.Control)

    [<Fact>]
    let ``pointer translation preserves extreme coordinates`` () =
        let input =
            InputAdapter.pointerInput
                (Point(System.Double.NegativeInfinity, System.Double.PositiveInfinity))
                None
                false
                Avalonia.Input.KeyModifiers.None

        Assert.Equal(System.Double.NegativeInfinity, input.Position.X)
        Assert.Equal(System.Double.PositiveInfinity, input.Position.Y)
        Assert.Equal(None, input.Button)

    [<Fact>]
    let ``text translation preserves whitespace`` () =
        Assert.Equal(Some { Text = " " }, InputAdapter.textInput " ")

    [<Fact>]
    let ``editing adapter leaves command-modified keys for command routing`` () =
        Assert.Equal(None, InputAdapter.editingEvent Avalonia.Input.Key.Left Avalonia.Input.KeyModifiers.Control)

        Assert.Equal(
            Some Functor.Domain.Editing.EditingEvent.MoveLeft,
            InputAdapter.editingEvent Avalonia.Input.Key.Left Avalonia.Input.KeyModifiers.None
        )

    [<Fact>]
    let ``shell adapter routes command palette and settings shortcuts`` () =
        Assert.Equal(
            Some ShellAction.OpenCommandPalette,
            InputAdapter.shellAction
                Avalonia.Input.Key.P
                (Avalonia.Input.KeyModifiers.Control ||| Avalonia.Input.KeyModifiers.Shift)
        )

        Assert.Equal(
            Some ShellAction.OpenSettings,
            InputAdapter.shellAction Avalonia.Input.Key.OemComma Avalonia.Input.KeyModifiers.Meta
        )

        Assert.Equal(None, InputAdapter.shellAction Avalonia.Input.Key.P Avalonia.Input.KeyModifiers.None)

    [<Fact>]
    let ``editor adapter routes command shortcuts`` () =
        Assert.Equal(
            Some EditorAction.Copy,
            InputAdapter.editorAction Avalonia.Input.Key.C Avalonia.Input.KeyModifiers.Control
        )

        Assert.Equal(
            Some EditorAction.SaveFileAs,
            InputAdapter.editorAction
                Avalonia.Input.Key.S
                (Avalonia.Input.KeyModifiers.Meta ||| Avalonia.Input.KeyModifiers.Shift)
        )

        Assert.Equal(
            None,
            InputAdapter.editorAction
                Avalonia.Input.Key.W
                (Avalonia.Input.KeyModifiers.Control ||| Avalonia.Input.KeyModifiers.Shift)
        )

    [<Fact>]
    let ``command shortcuts reject unrelated extra modifiers`` () =
        Assert.Equal(
            None,
            InputAdapter.editorAction
                Avalonia.Input.Key.C
                (Avalonia.Input.KeyModifiers.Control ||| Avalonia.Input.KeyModifiers.Alt)
        )

        Assert.Equal(
            None,
            InputAdapter.shellAction
                Avalonia.Input.Key.P
                (Avalonia.Input.KeyModifiers.Control
                 ||| Avalonia.Input.KeyModifiers.Alt
                 ||| Avalonia.Input.KeyModifiers.Shift)
        )

    [<Fact>]
    let ``editing adapter routes deletion and newline keys`` () =
        Assert.Equal(
            Some Functor.Domain.Editing.EditingEvent.Backspace,
            InputAdapter.editingEvent Avalonia.Input.Key.Back Avalonia.Input.KeyModifiers.None
        )

        Assert.Equal(
            Some Functor.Domain.Editing.EditingEvent.Delete,
            InputAdapter.editingEvent Avalonia.Input.Key.Delete Avalonia.Input.KeyModifiers.None
        )

        Assert.Equal(
            Some Functor.Domain.Editing.EditingEvent.InsertNewLine,
            InputAdapter.editingEvent Avalonia.Input.Key.Enter Avalonia.Input.KeyModifiers.None
        )

    [<Fact>]
    let ``editor adapter routes remaining file commands`` () =
        Assert.Equal(
            Some EditorAction.Cut,
            InputAdapter.editorAction Avalonia.Input.Key.X Avalonia.Input.KeyModifiers.Control
        )

        Assert.Equal(
            Some EditorAction.Paste,
            InputAdapter.editorAction Avalonia.Input.Key.V Avalonia.Input.KeyModifiers.Meta
        )

        Assert.Equal(
            Some EditorAction.NewDocument,
            InputAdapter.editorAction Avalonia.Input.Key.N Avalonia.Input.KeyModifiers.Control
        )
