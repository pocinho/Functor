namespace Functor.Tests.Avalonia

open Avalonia.Controls
open Avalonia.Headless.XUnit
open Functor.Avalonia
open Functor.Avalonia.Controls
open Xunit

type ShellProjectionTests() =
    [<AvaloniaFact>]
    member _.``projection derives active document visibility from the editor state``() =
        let editor = EditorControl()
        editor.NewDocument()
        let initial = ShellProjection.fromEditor editor

        Assert.True(initial.HasActiveDocument)
        editor.CloseDocument()

        let updated = ShellProjection.fromEditor editor
        Assert.False(updated.HasActiveDocument)

    [<AvaloniaFact>]
    member _.``projection derives tabs from workspace state``() =
        let editor = EditorControl()
        let window = Window(Content = editor)
        window.Show()
        let projection = ShellProjection.fromEditor editor

        Assert.Single(projection.Tabs) |> ignore
        Assert.True(projection.Tabs.Head.IsActive)
