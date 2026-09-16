namespace Functor.Tests.Input

open Functor.Input
open Xunit

module private Helpers =
    let keyboard key modifiers = { Key = key; Modifiers = modifiers }

type KeymapTests() =
    [<Fact>]
    member _.``navigation keys resolve without command modifiers``() =
        Assert.Equal(Some KeyAction.MoveLeft, Keymap.resolve (Helpers.keyboard Key.Left KeyModifiers.None))
        Assert.Equal(Some KeyAction.MoveRight, Keymap.resolve (Helpers.keyboard Key.Right KeyModifiers.Shift))
        Assert.Equal(Some KeyAction.InsertNewLine, Keymap.resolve (Helpers.keyboard Key.Enter KeyModifiers.None))

    [<Fact>]
    member _.``editing keys resolve``() =
        Assert.Equal(Some KeyAction.Backspace, Keymap.resolve (Helpers.keyboard Key.Backspace KeyModifiers.None))
        Assert.Equal(Some KeyAction.Delete, Keymap.resolve (Helpers.keyboard Key.Delete KeyModifiers.None))

    [<Fact>]
    member _.``command modifiers are left for command routing``() =
        Assert.Equal(None, Keymap.resolve (Helpers.keyboard Key.Left KeyModifiers.Control))
        Assert.Equal(None, Keymap.resolve (Helpers.keyboard Key.Enter KeyModifiers.Meta))

    [<Fact>]
    member _.``unmapped character does not become an editing action``() =
        Assert.Equal(None, Keymap.resolve (Helpers.keyboard (Key.Character 'a') KeyModifiers.None))

    [<Fact>]
    member _.``custom bindings can intentionally use command modifiers``() =
        let input = Helpers.keyboard (Key.Character 'p') KeyModifiers.Control
        let keymap = Keymap.defaultKeymap |> Keymap.bind input KeyAction.InsertNewLine

        Assert.Equal(Some KeyAction.InsertNewLine, Keymap.resolveWith keymap input)

    [<Fact>]
    member _.``shell shortcuts resolve command palette and settings actions``() =
        Assert.Equal(
            Some ShellAction.OpenCommandPalette,
            Keymap.resolveShell (Helpers.keyboard (Key.Character 'p') (KeyModifiers.Control ||| KeyModifiers.Shift))
        )

        Assert.Equal(
            Some ShellAction.OpenSettings,
            Keymap.resolveShell (Helpers.keyboard (Key.Character ',') KeyModifiers.Meta)
        )

        Assert.Equal(None, Keymap.resolveShell (Helpers.keyboard (Key.Character 'p') KeyModifiers.Shift))

    [<Fact>]
    member _.``editor shortcuts resolve command actions with shift variants``() =
        Assert.Equal(
            Some EditorAction.Copy,
            Keymap.resolveEditor (Helpers.keyboard (Key.Character 'c') KeyModifiers.Control)
        )

        Assert.Equal(
            Some EditorAction.SaveFileAs,
            Keymap.resolveEditor (Helpers.keyboard (Key.Character 's') (KeyModifiers.Control ||| KeyModifiers.Shift))
        )

        Assert.Equal(
            Some EditorAction.ReopenClosedTab,
            Keymap.resolveEditor (Helpers.keyboard (Key.Character 't') (KeyModifiers.Meta ||| KeyModifiers.Shift))
        )

        Assert.Equal(
            None,
            Keymap.resolveEditor (Helpers.keyboard (Key.Character 'w') (KeyModifiers.Control ||| KeyModifiers.Shift))
        )
