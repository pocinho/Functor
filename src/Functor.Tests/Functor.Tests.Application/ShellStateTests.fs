namespace Functor.Tests.Application

open Functor.Application
open Functor.Rendering
open Xunit

type ShellStateTests() =
    [<Fact>]
    member _.``starts with default settings and closed palette``() =
        let state = ShellState.initial

        Assert.Equal(AppSettings.defaults, state.AppSettings)
        Assert.False(state.IsCommandPaletteOpen)

    [<Fact>]
    member _.``updates settings without changing palette visibility``() =
        let state = ShellState.initial |> ShellState.openCommandPalette

        let updated =
            state
            |> ShellState.withSettings
                { AppSettings.defaults with
                    Theme = ThemeSettings.fromSource Theme.defaultSource }

        Assert.True(updated.IsCommandPaletteOpen)
        Assert.Equal("Custom", updated.AppSettings.Theme.Preset)

    [<Fact>]
    member _.``closes command palette``() =
        let state =
            ShellState.initial
            |> ShellState.openCommandPalette
            |> ShellState.closeCommandPalette

        Assert.False(state.IsCommandPaletteOpen)
