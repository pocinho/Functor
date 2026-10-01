namespace Functor.Application

type ShellState =
    { AppSettings: AppSettings
      IsCommandPaletteOpen: bool }

module ShellState =
    let initial =
        { AppSettings = AppSettings.defaults
          IsCommandPaletteOpen = false }

    let withSettings settings state = { state with AppSettings = settings }

    let openCommandPalette state =
        { state with
            IsCommandPaletteOpen = true }

    let closeCommandPalette state =
        { state with
            IsCommandPaletteOpen = false }
