namespace Functor.Avalonia

open Functor.Application

type ShellLayoutState =
    { SidePanelWidth: double
      IsPanelAnimating: bool }

type ShellModel = { Layout: ShellLayoutState }

type ShellMsg =
    | SetSidePanelWidth of double
    | BeginPanelAnimation
    | EndPanelAnimation
    | OpenFileRequested
    | CloseDocumentRequested
    | ReopenClosedTabRequested

type ShellEffect = DispatchAppCommand of AppCommand

module ShellLayoutState =
    [<Literal>]
    let MinimumSidePanelWidth = 240.0

    [<Literal>]
    let MaximumSidePanelWidth = 640.0

    [<Literal>]
    let DefaultSidePanelWidth = 320.0

    let initial =
        { SidePanelWidth = DefaultSidePanelWidth
          IsPanelAnimating = false }

    let clampSidePanelWidth width =
        width |> max MinimumSidePanelWidth |> min MaximumSidePanelWidth

module ShellModel =
    let initial = { Layout = ShellLayoutState.initial }

module ShellUpdate =
    let update message model =
        match message with
        | SetSidePanelWidth width ->
            { model with
                Layout =
                    { model.Layout with
                        SidePanelWidth = ShellLayoutState.clampSidePanelWidth width } },
            []
        | BeginPanelAnimation ->
            { model with
                Layout =
                    { model.Layout with
                        IsPanelAnimating = true } },
            []
        | EndPanelAnimation ->
            { model with
                Layout =
                    { model.Layout with
                        IsPanelAnimating = false } },
            []
        | OpenFileRequested -> model, [ DispatchAppCommand AppCommand.openFile ]
        | CloseDocumentRequested -> model, [ DispatchAppCommand AppCommand.closeDocument ]
        | ReopenClosedTabRequested -> model, [ DispatchAppCommand AppCommand.reopenClosedTab ]
