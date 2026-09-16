namespace Functor.Tests.Application

open Functor.Application
open Xunit

type ShellMvuTests() =
    [<Fact>]
    member _.``shell visibility transitions are immutable``() =
        let opened = ShellState.initial |> ShellState.openCommandPalette
        let closed = opened |> ShellState.closeCommandPalette

        Assert.False(ShellState.initial.IsCommandPaletteOpen)
        Assert.True(opened.IsCommandPaletteOpen)
        Assert.False(closed.IsCommandPaletteOpen)

    [<Fact>]
    member _.``host-owned shell requests do not mutate the editor session``() =
        let session = EditorSession()
        let initialState = session.State
        let publishedStates = ResizeArray<AppSessionState>()
        let requestedEffects = ResizeArray<AppEffect list>()
        session.StateChanged.Add(fun state -> publishedStates.Add(state) |> ignore)
        session.EffectsRequested.Add(fun effects -> requestedEffects.Add(effects) |> ignore)

        session.DispatchCommand(AppCommand.openCommandPalette)
        session.DispatchCommand(AppCommand.openSettings)

        Assert.Equal(initialState, session.State)
        Assert.Empty(publishedStates)
        Assert.Empty(requestedEffects)

    [<Fact>]
    member _.``core dispatch remains independent from shell visibility``() =
        let shell = ShellState.initial |> ShellState.openCommandPalette
        let session = EditorSession()

        session.DispatchCommand(AppCommand.toCoreEvent (Functor.Domain.Core.ResizeViewport(640, 480)))

        Assert.True(shell.IsCommandPaletteOpen)
        let expectedViewport: Functor.Domain.Core.Viewport = { Width = 640; Height = 480 }
        Assert.Equal(expectedViewport, session.State.Model.View.Viewport)
