namespace Functor.Avalonia.Controls

open System
open Avalonia
open Avalonia.Controls
open Avalonia.Layout
open Avalonia.Media
open Functor.Domain.Document
open Functor.Workspace

type WorkspaceDocumentView() as this =
    inherit UserControl()

    let documentActivated = Event<DocumentId>()
    let fileOpenRequested = Event<string>()
    let content = StackPanel(Spacing = 2.0)

    let indentation depth = Thickness(float (depth * 14), 2, 4, 2)

    let fileButton depth name path isDirty documentId =
        let file =
            Button(
                Content = (if isDirty then name + " *" else name),
                Padding = Thickness(4, 2),
                HorizontalContentAlignment = HorizontalAlignment.Left,
                HorizontalAlignment = HorizontalAlignment.Stretch,
                Margin = indentation depth
            )

        documentId
        |> Option.iter (fun id -> file.Click.Add(fun _ -> documentActivated.Trigger(id)))

        if documentId.IsNone then
            path |> Option.iter (fun value -> file.Click.Add(fun _ -> fileOpenRequested.Trigger(value)))

        file

    let rec nodeControl depth (node: WorkspaceFileTreeNode) : Control =
        if node.IsDirectory then
            let children = StackPanel(Spacing = 2.0)

            node.Children
            |> List.iter (fun child -> children.Children.Add(nodeControl (depth + 1) child) |> ignore)

            Expander(
                Header = TextBlock(Text = node.Name, FontWeight = FontWeight.Bold),
                Content = children,
                IsExpanded = true,
                Margin = indentation depth
            )
            :> Control
        else
            match node.DocumentId with
            | documentId -> fileButton depth node.Name (Some node.Path) node.IsDirty documentId :> Control

    do this.Content <- content

    member _.DocumentActivated = documentActivated.Publish

    member _.FileOpenRequested = fileOpenRequested.Publish

    member _.ApplyWorkspace(root: WorkspaceFileTreeNode, tabs: WorkspaceTabProjection list) =
        content.Children.Clear()

        let openEditors = StackPanel(Spacing = 2.0)

        tabs
        |> List.iter (fun tab ->
            openEditors.Children.Add(fileButton 0 tab.Name tab.Path tab.IsDirty (Some tab.DocumentId))
            |> ignore)

        content.Children.Add(
            Expander(
                Header = TextBlock(Text = "OPEN EDITORS", FontWeight = FontWeight.Bold),
                Content = openEditors,
                IsExpanded = true,
                Margin = Thickness(4, 4, 4, 6)
            )
        )
        |> ignore

        if String.IsNullOrEmpty root.Path then
            root.Children
            |> List.iter (fun node -> content.Children.Add(nodeControl 0 node) |> ignore)
        else
            content.Children.Add(nodeControl 0 root) |> ignore
