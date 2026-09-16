namespace Functor.Tests.Avalonia

open Avalonia.Controls
open Avalonia.Headless.XUnit
open Avalonia.Threading
open Functor.Application
open Functor.Domain.Core
open Functor.Avalonia.Views
open Xunit

module CommandPaletteViewTests =
    let private state = AppSessionState.empty CoreModel.empty

    [<AvaloniaFact>]
    let ``configure renders enabled commands and selects the first`` () =
        let view = CommandPaletteView()
        view.Configure(AppCommandCatalog.all, state, ignore)

        let commandList = view.FindControl<ListBox>("CommandList")

        let enabledCommandCount =
            AppCommandCatalog.all
            |> List.filter (fun command -> command.IsEnabled state)
            |> List.length

        Assert.Equal(enabledCommandCount, commandList.ItemCount)
        Assert.Equal(Some "file.new", view.SelectedDescriptor |> Option.map (fun descriptor -> descriptor.Id))

    [<AvaloniaFact>]
    let ``search filters commands and preserves the selected descriptor`` () =
        let view = CommandPaletteView()
        view.Configure(AppCommandCatalog.all, state, ignore)
        let searchBox = view.FindControl<TextBox>("SearchBox")
        searchBox.Text <- "settings"
        Dispatcher.UIThread.RunJobs()

        let commandList = view.FindControl<ListBox>("CommandList")

        Assert.Equal(1, commandList.ItemCount)
        Assert.Equal(Some "workbench.settings", view.SelectedDescriptor |> Option.map (fun descriptor -> descriptor.Id))

    [<AvaloniaFact>]
    let ``reconfiguring clears a previous search query`` () =
        let view = CommandPaletteView()
        view.Configure(AppCommandCatalog.all, state, ignore)
        let searchBox = view.FindControl<TextBox>("SearchBox")
        searchBox.Text <- "settings"
        Dispatcher.UIThread.RunJobs()

        view.Configure(AppCommandCatalog.all, state, ignore)

        Assert.Equal("", searchBox.Text)

        let enabledCommandCount =
            AppCommandCatalog.all
            |> List.filter (fun command -> command.IsEnabled state)
            |> List.length

        Assert.Equal(enabledCommandCount, view.FindControl<ListBox>("CommandList").ItemCount)

    [<AvaloniaFact>]
    let ``close and reopen resets an empty filtered palette`` () =
        let view = CommandPaletteView()
        let mutable closeCount = 0
        view.CloseRequested.Add(fun _ -> closeCount <- closeCount + 1)
        view.Configure(AppCommandCatalog.all, state, ignore)

        let searchBox = view.FindControl<TextBox>("SearchBox")
        searchBox.Text <- "no matching command"
        Dispatcher.UIThread.RunJobs()

        Assert.Equal(0, view.FindControl<ListBox>("CommandList").ItemCount)
        Assert.Null(view.SelectedDescriptor)

        view.RaiseEvent(
            Avalonia.Input.KeyEventArgs(
                RoutedEvent = Avalonia.Input.InputElement.KeyDownEvent,
                Key = Avalonia.Input.Key.Escape
            )
        )

        Assert.Equal(1, closeCount)

        view.Configure(AppCommandCatalog.all, state, ignore)

        Assert.Equal("", searchBox.Text)
        Assert.Equal(Some "file.new", view.SelectedDescriptor |> Option.map (fun descriptor -> descriptor.Id))
