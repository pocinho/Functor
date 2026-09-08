namespace Functor.Core.Tests

open Functor.Domain.Editing
open Xunit
open TestFixtures

type EditingTests() =
    [<Fact>]
    member _.``new model starts empty``() =
        let model = EditingModel.create ()

        Assert.True([ "" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 0 }, model.Cursor)
        Assert.False(model.IsDirty)

    [<Fact>]
    member _.``insert string updates buffer cursor and undo stack``() =
        let model =
            EditingModel.create () |> applyEditing (EditingEvent.InsertString "hello")

        Assert.True([ "hello" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 5 }, model.Cursor)
        Assert.True(model.IsDirty)
        Assert.Equal(5, model.UndoStack.Length)

    [<Fact>]
    member _.``backspace removes character and moves cursor left``() =
        let model =
            EditingModel.create ()
            |> applyEditing (EditingEvent.InsertString "abc")
            |> applyEditing EditingEvent.Backspace

        Assert.True([ "ab" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 2 }, model.Cursor)

    [<Fact>]
    member _.``backspace at start of first line is a no-op``() =
        let model = EditingModel.create () |> applyEditing EditingEvent.Backspace

        Assert.Equal(EditingModel.create (), model)

    [<Fact>]
    member _.``backspace at start joins with previous line``() =
        let model =
            EditingModel.create ()
            |> applyEditing (EditingEvent.InsertString "one")
            |> applyEditing EditingEvent.InsertNewLine
            |> applyEditing (EditingEvent.InsertString "two")
            |> applyEditing EditingEvent.MoveToLineStart
            |> applyEditing EditingEvent.Backspace

        Assert.True([ "onetwo" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 3 }, model.Cursor)

    [<Fact>]
    member _.``delete removes character at cursor without moving cursor``() =
        let model =
            EditingModel.create ()
            |> applyEditing (EditingEvent.InsertString "abc")
            |> applyEditing EditingEvent.MoveToDocumentStart
            |> applyEditing EditingEvent.Delete

        Assert.True([ "bc" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 0 }, model.Cursor)

    [<Fact>]
    member _.``cursor movement is clamped to document bounds``() =
        let model =
            EditingModel.create ()
            |> applyEditing (EditingEvent.InsertString "abc")
            |> applyEditing EditingEvent.MoveRight
            |> applyEditing EditingEvent.MoveRight
            |> applyEditing EditingEvent.MoveLeft
            |> applyEditing EditingEvent.MoveLeft
            |> applyEditing EditingEvent.MoveLeft
            |> applyEditing EditingEvent.MoveLeft

        Assert.Equal({ Line = 0; Column = 0 }, model.Cursor)

    [<Fact>]
    member _.``undo and redo restore text``() =
        let model =
            EditingModel.create ()
            |> applyEditing (EditingEvent.InsertString "abc")
            |> applyEditing EditingEvent.Undo

        Assert.True([ "ab" ] = model.Buffer)

        let redone = model |> applyEditing EditingEvent.Redo
        Assert.True([ "abc" ] = redone.Buffer)

    [<Fact>]
    member _.``selection tracks cursor movement``() =
        let model =
            EditingModel.create ()
            |> applyEditing (EditingEvent.InsertString "abc")
            |> applyEditing EditingEvent.MoveToDocumentStart
            |> applyEditing EditingEvent.StartSelection
            |> applyEditing EditingEvent.MoveRight
            |> applyEditing EditingEvent.MoveRight
            |> applyEditing EditingEvent.UpdateSelection

        Assert.Equal(
            Some
                { Start = { Line = 0; Column = 0 }
                  End = { Line = 0; Column = 2 } },
            model.Selection
        )

    [<Fact>]
    member _.``overwrite mode can be toggled``() =
        let model = EditingModel.create () |> applyEditing EditingEvent.ToggleOverwriteMode

        Assert.True(model.OverwriteMode)

    [<Fact>]
    member _.``typing replaces the selected text and clears selection``() =
        let model =
            EditingModel.create ()
            |> applyEditing (EditingEvent.InsertString "hello")
            |> applyEditing EditingEvent.MoveToDocumentStart
            |> applyEditing EditingEvent.StartSelection
            |> applyEditing EditingEvent.MoveRight
            |> applyEditing EditingEvent.MoveRight
            |> applyEditing EditingEvent.UpdateSelection
            |> applyEditing (EditingEvent.InsertString "X")

        Assert.True([ "Xllo" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 1 }, model.Cursor)
        Assert.True(model.Selection.IsNone)

    [<Fact>]
    member _.``backspace deletes selected text``() =
        let model =
            EditingModel.create ()
            |> applyEditing (EditingEvent.InsertString "hello")
            |> applyEditing EditingEvent.MoveToDocumentStart
            |> applyEditing EditingEvent.StartSelection
            |> applyEditing EditingEvent.MoveRight
            |> applyEditing EditingEvent.MoveRight
            |> applyEditing EditingEvent.UpdateSelection
            |> applyEditing EditingEvent.Backspace

        Assert.True([ "llo" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 0 }, model.Cursor)
        Assert.True(model.Selection.IsNone)

    [<Fact>]
    member _.``vertical movement preserves preferred column``() =
        let model =
            { EditingModel.create () with
                Buffer = [ "12345"; "1"; "12345" ]
                Cursor = { Line = 0; Column = 5 } }
            |> applyEditing EditingEvent.MoveDown
            |> applyEditing EditingEvent.MoveDown

        Assert.Equal({ Line = 2; Column = 5 }, model.Cursor)

    [<Fact>]
    member _.``overwrite mode replaces existing text``() =
        let model =
            { EditingModel.create () with
                Buffer = [ "abc" ]
                Cursor = { Line = 0; Column = 1 }
                OverwriteMode = true }
            |> applyEditing (EditingEvent.InsertString "Z")

        Assert.True([ "aZc" ] = model.Buffer)

    [<Fact>]
    member _.``inserted newlines are normalized``() =
        let model =
            EditingModel.create () |> applyEditing (EditingEvent.InsertString "a\r\nb\rc")

        Assert.True([ "a"; "b"; "c" ] = model.Buffer)
