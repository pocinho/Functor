namespace Functor.Avalonia.Views

open Avalonia
open Avalonia.Controls
open Avalonia.Input
open Avalonia.Layout
open Avalonia.Media
open Functor.Application
open Functor.Avalonia

module ThemedDialogWindow =
    let create (settings: AppSettings) title width height =
        let shape = ThemeShapeDensity.fromUiTheme settings.Theme.Ui

        let dialog =
            Window(
                Title = title,
                Width = width,
                Height = height,
                CanResize = false,
                WindowStartupLocation = WindowStartupLocation.CenterOwner,
                WindowDecorations = WindowDecorations.BorderOnly,
                ExtendClientAreaToDecorationsHint = true,
                ExtendClientAreaTitleBarHeightHint = shape.TitleBarHeight
            )

        ThemeManager.apply Application.Current dialog settings

        let titleBar = Border()
        titleBar.Height <- shape.TitleBarHeight
        titleBar.Padding <- Thickness(shape.TitleBarHorizontalPadding, 0.0)
        titleBar.BorderThickness <- Thickness(0.0, 0.0, 0.0, shape.SeparatorWidth)

        let titleText = TextBlock()
        titleText.Text <- title
        titleText.VerticalAlignment <- VerticalAlignment.Center

        titleBar.Child <- titleText

        titleBar.PointerPressed.Add(fun args ->
            if args.GetCurrentPoint(titleBar).Properties.PointerUpdateKind = PointerUpdateKind.LeftButtonPressed then
                dialog.BeginMoveDrag(args))

        let contentHost = Border()
        contentHost.BorderThickness <- Thickness(shape.BorderWidth)

        let applyChromeResources () =
            titleBar.Background <- Application.Current.Resources["Theme.PanelBackground"] :?> IBrush
            titleBar.BorderBrush <- Application.Current.Resources["Theme.Border"] :?> IBrush
            titleText.Foreground <- Application.Current.Resources["Theme.TextPrimary"] :?> IBrush
            contentHost.Background <- Application.Current.Resources["Theme.SurfaceBackground"] :?> IBrush
            contentHost.BorderBrush <- Application.Current.Resources["Theme.Border"] :?> IBrush

        applyChromeResources ()
        dialog.ResourcesChanged.Add(fun _ -> applyChromeResources ())

        let root = Grid()
        root.RowDefinitions <- RowDefinitions("Auto,*")
        root.Children.Add(titleBar) |> ignore
        Grid.SetRow(titleBar, 0)
        root.Children.Add(contentHost) |> ignore
        Grid.SetRow(contentHost, 1)
        dialog.Content <- root

        let setContent content = contentHost.Child <- content

        dialog, setContent
