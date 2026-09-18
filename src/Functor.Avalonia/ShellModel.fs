namespace Functor.Avalonia

open Functor.Application

type ShellPanel =
    | Notebook
    | Agent
    | PluginPanel of string

type ShellLayoutState =
    { IsSidePanelOpen: bool
      SidePanelWidth: double
      ActivePanel: ShellPanel option
      IsPanelAnimating: bool }

type ShellModel = { Layout: ShellLayoutState }

type ShellMsg =
    | SetSidePanelOpen of bool
    | SetSidePanelWidth of double
    | SelectPanel of ShellPanel option
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
        { IsSidePanelOpen = false
          SidePanelWidth = DefaultSidePanelWidth
          ActivePanel = None
          IsPanelAnimating = false }

    let clampSidePanelWidth width =
        width |> max MinimumSidePanelWidth |> min MaximumSidePanelWidth

module ShellModel =
    let initial = { Layout = ShellLayoutState.initial }

module ShellUpdate =
    let update message model =
        match message with
        | SetSidePanelOpen isOpen ->
            { model with
                Layout =
                    { model.Layout with
                        IsSidePanelOpen = isOpen } },
            []
        | SetSidePanelWidth width ->
            { model with
                Layout =
                    { model.Layout with
                        SidePanelWidth = ShellLayoutState.clampSidePanelWidth width } },
            []
        | SelectPanel panel ->
            { model with
                Layout =
                    { model.Layout with
                        ActivePanel = panel
                        IsSidePanelOpen = panel.IsSome } },
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
