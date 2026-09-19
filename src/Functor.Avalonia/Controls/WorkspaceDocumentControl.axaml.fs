namespace Functor.Avalonia.Controls

open System
open Avalonia
open Avalonia.Controls
open Avalonia.Layout
open Avalonia.Markup.Xaml
open Avalonia.Media
open Functor.Domain.Document
open Functor.Workspace

type WorkspaceDocumentControl() as this =
    inherit UserControl()

    let documentActivated = Event<DocumentId>()
    let fileOpenRequested = Event<string>()
    let content = lazy (this.FindControl<StackPanel>("ContentPanel"))
    let mutable expandedFolders: Set<string> = Set.empty

    let mutable renderedRoot: WorkspaceFileTreeNode option = None
    let mutable renderedTabs: WorkspaceTabProjection list option = None
    let mutable openEditorsPanel: StackPanel option = None
    let mutable treePanel: StackPanel option = None

    let itemFontSize = 12.0

    let indentation depth = Thickness(float (depth * 14), 0, 0, 0)

    let fileButton depth name path isDirty documentId =
        let file =
            Button(
                Content = TextBlock(Text = (if isDirty then name + " *" else name), FontSize = itemFontSize),
                Padding = Thickness(0),
                HorizontalContentAlignment = HorizontalAlignment.Left,
                HorizontalAlignment = HorizontalAlignment.Stretch,
                Margin = indentation depth
            )

        documentId
        |> Option.iter (fun id -> file.Click.Add(fun _ -> documentActivated.Trigger(id)))

        if documentId.IsNone then
            path
            |> Option.iter (fun value -> file.Click.Add(fun _ -> fileOpenRequested.Trigger(value)))

        file

    let collapsibleControl depth name isExpanded onExpanded onCollapsed (children: Control) =
        let arrow = TextBlock(Text = ">", FontSize = itemFontSize)

        let header =
            Border(
                BorderBrush = SolidColorBrush(Color.FromArgb(110uy, 160uy, 160uy, 160uy)),
                BorderThickness = Thickness(0, 0, 0, 1),
                Padding = Thickness(0),
                Child =
                    TextBlock(
                        Text = name,
                        FontWeight = FontWeight.Bold,
                        FontSize = itemFontSize,
                        HorizontalAlignment = HorizontalAlignment.Stretch
                    )
            )

        let collapseButton =
            Button(
                Content = arrow,
                Width = 18.0,
                MinWidth = 18.0,
                Padding = Thickness(0),
                HorizontalContentAlignment = HorizontalAlignment.Center,
                VerticalContentAlignment = VerticalAlignment.Center,
                HorizontalAlignment = HorizontalAlignment.Left
            )

        let row =
            DockPanel(
                LastChildFill = true,
                Margin = indentation depth,
                HorizontalAlignment = HorizontalAlignment.Stretch
            )

        DockPanel.SetDock(collapseButton, Dock.Left)
        row.Children.Add(collapseButton) |> ignore
        row.Children.Add(header) |> ignore

        let mutable expanded = isExpanded

        let updateVisualState () =
            children.IsVisible <- expanded
            arrow.RenderTransform <- RotateTransform(if expanded then 90.0 else 0.0)

        collapseButton.Click.Add(fun _ ->
            expanded <- not expanded

            if expanded then onExpanded () else onCollapsed ()

            updateVisualState ())

        updateVisualState ()

        let container = StackPanel(Spacing = 0.0, Margin = Thickness(0))
        container.Children.Add(row) |> ignore
        container.Children.Add(children) |> ignore
        container :> Control

    let rec nodeControl depth (node: WorkspaceFileTreeNode) : Control =
        if node.IsDirectory then
            let children = StackPanel(Spacing = 0.0, Margin = Thickness(0))

            node.Children
            |> List.iter (fun child -> children.Children.Add(nodeControl (depth + 1) child) |> ignore)

            collapsibleControl
                depth
                node.Name
                (expandedFolders.Contains node.Key)
                (fun () -> expandedFolders <- expandedFolders.Add(node.Key))
                (fun () -> expandedFolders <- expandedFolders.Remove(node.Key))
                (children :> Control)
        else
            fileButton depth node.Name (Some node.Path) node.IsDirty None :> Control

    let rec treeRenderKey (node: WorkspaceFileTreeNode) =
        String.concat
            "|"
            [ node.Key
              node.Name
              node.Path
              string node.IsDirectory
              string node.IsDirty
              node.Children |> List.map treeRenderKey |> String.concat "|" ]

    do this.InitializeComponent()

    member _.DocumentActivated = documentActivated.Publish

    member _.FileOpenRequested = fileOpenRequested.Publish

    member _.ApplyWorkspace(root: WorkspaceFileTreeNode, tabs: WorkspaceTabProjection list) =
        let rootChanged =
            renderedRoot
            |> Option.map treeRenderKey
            |> fun previous -> previous <> Some(treeRenderKey root)

        let tabsChanged = renderedTabs <> Some tabs

        if rootChanged || tabsChanged then
            let editors, tree =
                match openEditorsPanel, treePanel with
                | Some editors, Some tree -> editors, tree
                | _ ->
                    let content = content.Value
                    content.Children.Clear()

                    content.Children.Add(
                        TextBlock(
                            Text = "EXPLORER",
                            FontSize = itemFontSize,
                            FontWeight = FontWeight.Bold,
                            Margin = Thickness(0)
                        )
                    )
                    |> ignore

                    let editors = StackPanel(Spacing = 0.0, Margin = Thickness(0))

                    content.Children.Add(collapsibleControl 0 "OPEN EDITORS" true ignore ignore (editors :> Control))
                    |> ignore

                    let tree = StackPanel(Spacing = 0.0, Margin = Thickness(0))
                    content.Children.Add(tree) |> ignore
                    openEditorsPanel <- Some editors
                    treePanel <- Some tree
                    editors, tree

            if tabsChanged then
                editors.Children.Clear()

                tabs
                |> List.iter (fun tab ->
                    editors.Children.Add(fileButton 1 tab.Name tab.Path tab.IsDirty (Some tab.DocumentId))
                    |> ignore)

            if rootChanged then
                tree.Children.Clear()

                if String.IsNullOrEmpty root.Path then
                    root.Children
                    |> List.iter (fun node -> tree.Children.Add(nodeControl 0 node) |> ignore)
                else
                    let rootNode =
                        nodeControl
                            0
                            { root with
                                Name = root.Name.ToUpperInvariant() }

                    tree.Children.Add(rootNode) |> ignore

            renderedRoot <- Some root
            renderedTabs <- Some tabs

    member private this.InitializeComponent() = AvaloniaXamlLoader.Load(this)
