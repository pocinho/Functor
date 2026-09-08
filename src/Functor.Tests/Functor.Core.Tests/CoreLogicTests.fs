namespace Functor.Core.Tests

open Functor.Domain.Core
open Functor.Domain.Editing
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

    [<Fact>]
    member _.``editing marks the active document dirty``() =
        let opened = CoreLogic.update (OpenDocument "C:\\work\\file.fs") CoreModel.empty
        let edited = CoreLogic.update (ApplyEditingEvent(InsertString "text")) opened

        Assert.True(edited.ActiveDocument.Value.Metadata.IsDirty)
        Assert.True(edited.OpenDocuments.Head.Metadata.IsDirty)
