namespace Functor.Tests.Domain

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
