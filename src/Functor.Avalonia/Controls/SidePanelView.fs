namespace Functor.Avalonia.Controls

open Avalonia
open Avalonia.Controls
open Avalonia.Layout
open Avalonia.Media

type SidePanelView() as this =
    inherit UserControl()

    let closeRequested = Event<unit>()
    let title = TextBlock(FontWeight = FontWeight.Bold)
    let content = ContentControl()
    let closeButton = Button(Content = "Close")
    let host = Border(BorderThickness = Thickness(1, 0, 0, 0), Padding = Thickness(12))

    do
        let header = Grid(ColumnDefinitions = ColumnDefinitions.Parse("*,Auto"))
        Grid.SetColumn(title, 0)
        Grid.SetColumn(closeButton, 1)
        header.Children.Add(title) |> ignore
        header.Children.Add(closeButton) |> ignore

        let panel = StackPanel(Spacing = 12.0)
        panel.Children.Add(header) |> ignore
        panel.Children.Add(content) |> ignore
        host.Child <- panel
        this.Content <- host

        closeButton.Click.Add(fun _ -> closeRequested.Trigger())

    member _.CloseRequested = closeRequested.Publish

    member _.Title
        with get () = title.Text
        and set value = title.Text <- value

    member _.PanelContent
        with get () = content.Content
        and set value = content.Content <- value

    member _.IsOpen
        with get () = this.IsVisible
        and set value = this.IsVisible <- value

    member _.PanelWidth
        with get () = this.Width
        and set value = this.Width <- value
