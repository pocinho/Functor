namespace Functor.App.Views

open System
open Avalonia.Controls
open Avalonia.Controls.Primitives
open Avalonia.Markup.Xaml
open Functor.App.Controls

type MainView() as this =
    inherit UserControl()

    let editor = lazy (this.FindControl<EditorControl>("EditorControl"))
    let verticalScrollBar = lazy (this.FindControl<ScrollBar>("VerticalScrollBar"))

    let updateScrollBar () =
        let editor = editor.Value
        let verticalScrollBar = verticalScrollBar.Value
        let maximum = float editor.VerticalScrollMaximum

        verticalScrollBar.Maximum <- maximum
        verticalScrollBar.ViewportSize <- float editor.VerticalScrollViewport
        verticalScrollBar.LargeChange <- max 1.0 (float editor.VerticalScrollViewport)
        verticalScrollBar.IsEnabled <- maximum > 0.0
        verticalScrollBar.Value <- float editor.VerticalOffset

    do
        this.InitializeComponent()

        let editor = editor.Value
        let verticalScrollBar = verticalScrollBar.Value

        editor.ScrollStateChanged.Add(fun () -> updateScrollBar ())

        verticalScrollBar.ValueChanged.Add(fun args ->
            let offset = int (Math.Round(args.NewValue))

            if offset <> editor.VerticalOffset then
                editor.ScrollTo(offset))

        updateScrollBar ()

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
