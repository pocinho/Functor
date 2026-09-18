namespace Functor.Avalonia

open Functor.Application

type ToolKind =
    | WorkspaceTool
    | SearchTool

type ToolDescriptor =
    { Kind: ToolKind
      Id: string
      Title: string
      Glyph: string
      AccessibilityName: string }

type ToolPanelState =
    | ToolPanelUnavailable of message: string
    | ToolPanelLoading of message: string
    | ToolPanelEmpty of message: string
    | ToolPanelReady

module ToolPanelState =
    let forTool tool =
        match tool with
        | SearchTool -> ToolPanelEmpty "Search is not available yet."
        | WorkspaceTool -> ToolPanelReady

module ToolDescriptor =
    let all =
        [ { Kind = WorkspaceTool
            Id = "workspace"
            Title = "Workspace"
            Glyph = "W"
            AccessibilityName = "Workspace" }
          { Kind = SearchTool
            Id = "search"
            Title = "Search"
            Glyph = "S"
            AccessibilityName = "Search" } ]

    let get tool =
        all |> List.find (fun descriptor -> descriptor.Kind = tool)

type ShellLayoutState =
    { SidePanelWidth: double
      ActiveTool: ToolKind option
      IsToolPanelOpen: bool
      IsPanelAnimating: bool }

type ShellModel = { Layout: ShellLayoutState }

type ShellMsg =
    | SetSidePanelWidth of double
    | ToggleTool of ToolKind
    | RestoreTool of ToolKind option * bool
    | CloseToolPanel
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
          ActiveTool = None
          IsToolPanelOpen = false
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
        | ToggleTool tool ->
            let isSameTool = model.Layout.ActiveTool = Some tool

            { model with
                Layout =
                    { model.Layout with
                        ActiveTool = Some tool
                        IsToolPanelOpen =
                            if isSameTool then
                                not model.Layout.IsToolPanelOpen
                            else
                                true } },
            []
        | RestoreTool(tool, isPanelOpen) ->
            { model with
                Layout =
                    { model.Layout with
                        ActiveTool = tool
                        IsToolPanelOpen = isPanelOpen } },
            []
        | CloseToolPanel ->
            { model with
                Layout =
                    { model.Layout with
                        IsToolPanelOpen = false } },
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
