namespace Functor.Tests.Domain

open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Xunit


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
            |> CoreLogic.update (ScrollVerticalTo -10)
            |> CoreLogic.update (ScrollVerticalBy 25)

        Assert.Equal(EditorMode.Insert, model.Mode)
        Assert.Equal({ Width = 800; Height = 600 }, model.View.Viewport)
        Assert.Equal(25, model.View.VerticalOffset)

    [<Fact>]
    member _.``core routes horizontal scrolling events``() =
        let model =
            CoreModel.empty
            |> CoreLogic.update (ScrollHorizontalTo -10)
            |> CoreLogic.update (ScrollHorizontalBy 12)

        Assert.Equal(12, model.View.HorizontalOffset)

    [<Fact>]
    member _.``core opens and closes documents``() =
        let opened = CoreLogic.update (OpenDocument "C:\\work\\file.fs") CoreModel.empty
        let document = opened.ActiveDocument |> Option.get
        let closed = CoreLogic.update (CloseDocument document.Id) opened

        Assert.Equal(Some document.Id, opened.OpenDocuments |> List.tryHead |> Option.map (fun item -> item.Id))
        Assert.True(closed.OpenDocuments.IsEmpty)
        Assert.True(closed.ActiveDocument.IsNone)
        Assert.True([ "" ] = closed.Editing.Buffer)
        Assert.Equal(0, closed.View.VerticalOffset)

    [<Fact>]
    member _.``editing marks the active document dirty``() =
        let opened = CoreLogic.update (OpenDocument "C:\\work\\file.fs") CoreModel.empty
        let edited = CoreLogic.update (ApplyEditingEvent(InsertString "text")) opened

        Assert.True(edited.ActiveDocument.Value.Metadata.IsDirty)
        Assert.True(edited.OpenDocuments.Head.Metadata.IsDirty)

    [<Fact>]
    member _.``buffer edits advance revision and invalidate syntax``() =
        let opened = CoreLogic.update (OpenDocument "C:\\work\\file.fs") CoreModel.empty
        let moved = CoreLogic.update (ApplyEditingEvent MoveRight) opened
        let edited = CoreLogic.update (ApplyEditingEvent(InsertString "text")) moved

        Assert.Equal(0L, moved.Editing.Revision)
        Assert.False(moved.Syntax.IsDirty)
        Assert.Equal(1L, edited.Editing.Revision)
        Assert.True(edited.Syntax.IsDirty)
        Assert.Equal(1L, edited.Syntax.DocumentRevision)
        Assert.Equal(Some { StartLine = 0; EndLine = 0; OldEndLine = 0; LineDelta = 0 }, edited.Editing.LastChange)

    [<Fact>]
    member _.``loading a document resets syntax ownership``() =
        let first = CoreLogic.update (OpenDocument "C:\\work\\first.fs") CoreModel.empty
        let dirtySyntax = first |> CoreLogic.update (ApplySyntaxEvent(SetLanguage "fsharp"))
        let loaded = dirtySyntax |> CoreLogic.update (LoadDocument("C:\\work\\second.json", "{}"))

        Assert.Equal(loaded.ActiveDocument |> Option.map (fun document -> document.Id), loaded.Syntax.DocumentId)
        Assert.NotEqual(first.Syntax.DocumentId, loaded.Syntax.DocumentId)
        Assert.True(loaded.Syntax.Language.IsNone)
        Assert.False(loaded.Syntax.IsDirty)
