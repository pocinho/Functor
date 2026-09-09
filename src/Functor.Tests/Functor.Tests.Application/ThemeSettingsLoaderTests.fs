namespace Functor.Tests.Application

open Functor.Application
open Functor.Rendering
open Xunit


type ThemeSettingsLoaderTests() =
    [<Fact>]
    member _.``loads nested theme colors and border settings``() =
        let json =
            """
            {
              "theme": {
                "background": "#202020",
                "foreground": "#F0F0F0",
                "editorBorder": "#555555",
                "editorBorderWidth": 2.0
              }
            }
            """

        let result = ThemeSettingsLoader.loadText json
        let palette =
          match result with
          | Ok settings -> settings.ThemeSource.Resolve()
          | Error error -> failwith error

        Assert.Equal(0xFF202020u, palette.Background)
        Assert.Equal(0xFFF0F0F0u, palette.Foreground)
        Assert.Equal(Some 0xFF555555u, palette.EditorBorder)
        Assert.Equal(2.0f, palette.EditorBorderWidth)

    [<Fact>]
    member _.``loads colors without a leading hash``() =
        let result = ThemeSettingsLoader.loadText "{ \"background\": \"FF010203\" }"
        let palette =
          match result with
          | Ok settings -> settings.ThemeSource.Resolve()
          | Error error -> failwith error

        Assert.Equal(0xFF010203u, palette.Background)

    [<Fact>]
    member _.``invalid json returns an error``() =
        let result = ThemeSettingsLoader.loadText "{ invalid"

        Assert.True(result |> Result.isError)

    [<Fact>]
    member _.``application settings serialize and load theme values``() =
        let settings =
            { Theme.defaultPalette with
                Background = 0xFF101112u
                Foreground = 0xFFFAFBFCu
                EditorBorder = Some 0xFF333435u
                EditorBorderWidth = 3.0f }
            |> ThemeSettings.fromPalette
            |> AppSettings.fromTheme

        let result = settings |> AppSettingsLoader.toJson |> AppSettingsLoader.loadText
        let palette =
          match result with
          | Ok settings -> settings.Theme.ThemeSource.Resolve()
          | Error error -> failwith error

        Assert.Equal(0xFF101112u, palette.Background)
        Assert.Equal(0xFFFAFBFCu, palette.Foreground)
        Assert.Equal(Some 0xFF333435u, palette.EditorBorder)
        Assert.Equal(3.0f, palette.EditorBorderWidth)
