namespace Functor.Core.Tests

open System
open Functor.Domain.Core
open Functor.Domain.Diagnostics
open Functor.Domain.Document
open Functor.Domain.Editing
open Functor.Domain.Navigation
open Functor.Domain.Syntax
open Xunit

module private Helpers =
    let applyEditing event model = EditingLogic.update event model

    let applyNavigation event model = NavigationLogic.update event model

    let position line column = { Line = line; Column = column }

    let diagnostic line message =
        { Severity = DiagnosticSeverity.Error
          Message = message
          RangeStart = position line 0
          RangeEnd = position line 1
          Code = None
          Source = None }

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
            EditingModel.create ()
            |> Helpers.applyEditing (EditingEvent.InsertString "hello")

        Assert.True([ "hello" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 5 }, model.Cursor)
        Assert.True(model.IsDirty)
        Assert.Equal(5, model.UndoStack.Length)

    [<Fact>]
    member _.``backspace removes character and moves cursor left``() =
        let model =
            EditingModel.create ()
            |> Helpers.applyEditing (EditingEvent.InsertString "abc")
            |> Helpers.applyEditing EditingEvent.Backspace

        Assert.True([ "ab" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 2 }, model.Cursor)

    [<Fact>]
    member _.``backspace at start of first line is a no-op``() =
        let model = EditingModel.create () |> Helpers.applyEditing EditingEvent.Backspace

        Assert.Equal(EditingModel.create (), model)

    [<Fact>]
    member _.``backspace at start joins with previous line``() =
        let model =
            EditingModel.create ()
            |> Helpers.applyEditing (EditingEvent.InsertString "one")
            |> Helpers.applyEditing EditingEvent.InsertNewLine
            |> Helpers.applyEditing (EditingEvent.InsertString "two")
            |> Helpers.applyEditing EditingEvent.MoveToLineStart
            |> Helpers.applyEditing EditingEvent.Backspace

        Assert.True([ "onetwo" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 3 }, model.Cursor)

    [<Fact>]
    member _.``delete removes character at cursor without moving cursor``() =
        let model =
            EditingModel.create ()
            |> Helpers.applyEditing (EditingEvent.InsertString "abc")
            |> Helpers.applyEditing EditingEvent.MoveToDocumentStart
            |> Helpers.applyEditing EditingEvent.Delete

        Assert.True([ "bc" ] = model.Buffer)
        Assert.Equal({ Line = 0; Column = 0 }, model.Cursor)

    [<Fact>]
    member _.``cursor movement is clamped to document bounds``() =
        let model =
            EditingModel.create ()
            |> Helpers.applyEditing (EditingEvent.InsertString "abc")
            |> Helpers.applyEditing EditingEvent.MoveRight
            |> Helpers.applyEditing EditingEvent.MoveRight
            |> Helpers.applyEditing EditingEvent.MoveLeft
            |> Helpers.applyEditing EditingEvent.MoveLeft
            |> Helpers.applyEditing EditingEvent.MoveLeft
            |> Helpers.applyEditing EditingEvent.MoveLeft

        Assert.Equal({ Line = 0; Column = 0 }, model.Cursor)

    [<Fact>]
    member _.``undo and redo restore text``() =
        let model =
            EditingModel.create ()
            |> Helpers.applyEditing (EditingEvent.InsertString "abc")
            |> Helpers.applyEditing EditingEvent.Undo

        Assert.True([ "ab" ] = model.Buffer)

        let redone = model |> Helpers.applyEditing EditingEvent.Redo
        Assert.True([ "abc" ] = redone.Buffer)

    [<Fact>]
    member _.``selection tracks cursor movement``() =
        let model =
            EditingModel.create ()
            |> Helpers.applyEditing (EditingEvent.InsertString "abc")
            |> Helpers.applyEditing EditingEvent.MoveToDocumentStart
            |> Helpers.applyEditing EditingEvent.StartSelection
            |> Helpers.applyEditing EditingEvent.MoveRight
            |> Helpers.applyEditing EditingEvent.MoveRight
            |> Helpers.applyEditing EditingEvent.UpdateSelection

        Assert.Equal(
            Some
                { Start = { Line = 0; Column = 0 }
                  End = { Line = 0; Column = 2 } },
            model.Selection
        )

    [<Fact>]
    member _.``overwrite mode can be toggled``() =
        let model =
            EditingModel.create () |> Helpers.applyEditing EditingEvent.ToggleOverwriteMode

        Assert.True(model.OverwriteMode)

type DocumentTests() =
    [<Fact>]
    member _.``file document derives name from path``() =
        let document = DocumentModel.createFromFile "C:\\work\\notes.txt" "contents"

        Assert.Equal(Some "C:\\work\\notes.txt", document.Metadata.Path)
        Assert.Equal("notes.txt", document.Metadata.Name)
        Assert.Equal("contents", document.InitialText)
        Assert.False(document.Metadata.IsDirty)

    [<Fact>]
    member _.``document events update metadata without changing identity``() =
        let document = DocumentModel.createUntitled "untitled"
        let renamed = DocumentLogic.update (RenameDocument "renamed") document
        let dirty = DocumentLogic.update MarkDocumentDirty renamed
        let timestamp = DateTime(2024, 1, 2, 3, 4, 5, DateTimeKind.Utc)
        let updated = DocumentLogic.update (UpdateModifiedTimestamp timestamp) dirty

        Assert.Equal(document.Id, updated.Id)
        Assert.Equal("renamed", updated.Metadata.Name)
        Assert.True(updated.Metadata.IsDirty)
        Assert.Equal(Some timestamp, updated.Metadata.ModifiedAt)

type NavigationTests() =
    [<Fact>]
    member _.``jump list moves back and forward``() =
        let first =
            { Position = Helpers.position 1 2
              Description = Some "first" }

        let second =
            { Position = Helpers.position 3 4
              Description = None }

        let model =
            NavigationModel.create ()
            |> Helpers.applyNavigation (PushJump(first.Position, first.Description))
            |> Helpers.applyNavigation (PushJump(second.Position, second.Description))

        Assert.Equal(Some 0, model.JumpIndex)
        Assert.True([ second; first ] = model.JumpList)

        let back = model |> Helpers.applyNavigation JumpBack
        Assert.Equal(Some 1, back.JumpIndex)

        let forward = back |> Helpers.applyNavigation JumpForward
        Assert.Equal(Some 0, forward.JumpIndex)

    [<Fact>]
    member _.``search results are indexed and bounded``() =
        let results =
            [ { Line = 1
                Column = 2
                Preview = "one" }
              { Line = 4
                Column = 5
                Preview = "two" } ]

        let model =
            NavigationModel.create ()
            |> Helpers.applyNavigation (SetSearchQuery "term")
            |> Helpers.applyNavigation (SetSearchResults results)
            |> Helpers.applyNavigation NextSearchResult
            |> Helpers.applyNavigation NextSearchResult

        Assert.Equal(Some "term", model.SearchQuery)
        Assert.Equal(Some 1, model.SearchIndex)
        Assert.False(model.IsDirty)

type SyntaxAndDiagnosticsTests() =
    [<Fact>]
    member _.``setting language invalidates syntax``() =
        let model = SyntaxModel.create () |> SyntaxLogic.update (SetLanguage "fsharp")

        Assert.Equal(Some "fsharp", model.Language)
        Assert.True(model.IsDirty)

    [<Fact>]
    member _.``setting tokens clears syntax dirty state``() =
        let token =
            { Kind = "identifier"
              Line = 0
              Column = 0
              Length = 3 }

        let tokens = [ { Line = 0; Tokens = [ token ] } ]
        let model = SyntaxModel.create () |> SyntaxLogic.update (SetTokens tokens)

        Assert.True((tokens = model.Tokens), "Token cache did not match")
        Assert.False(model.IsDirty)
        Assert.True(model.LastTokenized.IsSome)

    [<Fact>]
    member _.``diagnostics are grouped by starting line``() =
        let diagnostics =
            [ Helpers.diagnostic 2 "warning"
              Helpers.diagnostic 1 "error"
              Helpers.diagnostic 2 "hint" ]

        let model =
            DiagnosticsModel.create () |> DiagnosticsModel.setDiagnostics <| diagnostics

        Assert.True((diagnostics = model.All), "Diagnostics did not match")
        Assert.Equal(2, model.ByLine.Length)

        Assert.Equal(
            2,
            model.ByLine
            |> List.find (fun group -> group.Line = 2)
            |> fun group -> group.Diagnostics.Length
        )

        Assert.False(model.IsDirty)

type CoreLogicTests() =
    [<Fact>]
    member _.``core routes editing events``() =
        let model =
            CoreModel.empty |> CoreLogic.update (ApplyEditingEvent(InsertString "text"))

        Assert.True([ "text" ] = model.Editing.Buffer)
        Assert.Equal(EditorMode.Normal, model.Mode)

    [<Fact>]
    member _.``core routes mode viewport and scrolling events``() =
        let model =
            CoreModel.empty
            |> CoreLogic.update (ChangeMode EditorMode.Insert)
            |> CoreLogic.update (ResizeViewport(800, 600))
            |> CoreLogic.update (ScrollTo -10)
            |> CoreLogic.update (ScrollBy 25)

        Assert.Equal(EditorMode.Insert, model.Mode)
        Assert.Equal({ Width = 800; Height = 600 }, model.Viewport)
        Assert.Equal(25, model.VerticalOffset)

    [<Fact>]
    member _.``core opens and closes documents``() =
        let opened = CoreLogic.update (OpenDocument "C:\\work\\file.fs") CoreModel.empty
        let document = opened.ActiveDocument |> Option.get
        let closed = CoreLogic.update (CloseDocument document.Id) opened

        Assert.Equal(Some document.Id, opened.OpenDocuments |> List.tryHead |> Option.map (fun item -> item.Id))
        Assert.True(closed.OpenDocuments.IsEmpty)
        Assert.True(closed.ActiveDocument.IsNone)
