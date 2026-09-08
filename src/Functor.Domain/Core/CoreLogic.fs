namespace Functor.Domain.Core

open Functor.Domain.Editing
open Functor.Domain.Document
open Functor.Domain.Syntax
open Functor.Domain.Navigation
open Functor.Domain.Diagnostics

/// Pure state transition function for the editor.
/// Applies a CoreEvent to a CoreModel and returns a new CoreModel.
module CoreLogic =

    // ────────────────────────────────────────────────
    // Document
    // ────────────────────────────────────────────────

    let private applyDocumentEvent (model: CoreModel) (evt: DocumentEvent) =
        let updatedActive =
            model.ActiveDocument |> Option.map (fun doc -> DocumentLogic.update evt doc)

        let updatedOpen =
            model.OpenDocuments
            |> List.map (fun doc ->
                if Some doc = model.ActiveDocument then
                    updatedActive |> Option.defaultValue doc
                else
                    doc)

        { model with
            ActiveDocument = updatedActive
            OpenDocuments = updatedOpen }

    // ────────────────────────────────────────────────
    // Editing
    // ────────────────────────────────────────────────

    let private applyEditingEvent (model: CoreModel) (evt: Functor.Domain.Editing.EditingEvent) =
        { model with
            Editing = EditingLogic.update evt model.Editing }

    // ────────────────────────────────────────────────
    // Syntax
    // ────────────────────────────────────────────────

    let private applySyntaxEvent (model: CoreModel) (evt: SyntaxEvent) =
        { model with
            Syntax = SyntaxLogic.update evt model.Syntax }

    // ────────────────────────────────────────────────
    // Navigation
    // ────────────────────────────────────────────────

    let private applyNavigationEvent (model: CoreModel) (evt: NavigationEvent) =
        { model with
            Navigation = NavigationLogic.update evt model.Navigation }

    // ────────────────────────────────────────────────
    // Diagnostics
    // ────────────────────────────────────────────────

    let private applyDiagnosticEvent (model: CoreModel) (evt: DiagnosticsEvent) =
        { model with
            Diagnostics = DiagnosticsLogic.update evt model.Diagnostics }

    // ────────────────────────────────────────────────
    // Core Update
    // ────────────────────────────────────────────────

    let update (evt: CoreEvent) (model: CoreModel) : CoreModel =
        match evt with

        // Document Lifecycle
        | OpenDocument path ->
            let doc = DocumentModel.createFromFile path ""

            { model with
                ActiveDocument = Some doc
                OpenDocuments = doc :: model.OpenDocuments }

        | CloseDocument id ->
            let remaining = model.OpenDocuments |> List.filter (fun d -> d.Id <> id)

            let newActive =
                match model.ActiveDocument with
                | Some d when d.Id = id -> remaining |> List.tryHead
                | other -> other

            { model with
                ActiveDocument = newActive
                OpenDocuments = remaining }

        | SwitchDocument id ->
            let newActive = model.OpenDocuments |> List.tryFind (fun d -> d.Id = id)

            { model with
                ActiveDocument = newActive }

        | ApplyDocumentEvent evt -> applyDocumentEvent model evt

        // Editing
        | ApplyEditingEvent evt -> applyEditingEvent model evt

        // Syntax
        | ApplySyntaxEvent evt -> applySyntaxEvent model evt

        // Navigation
        | ApplyNavigationEvent evt -> applyNavigationEvent model evt

        // Diagnostics
        | ApplyDiagnosticsEvent evt -> applyDiagnosticEvent model evt

        // Mode
        | ChangeMode mode -> { model with Mode = mode }

        // Viewport
        | ResizeViewport(width, height) ->
            { model with
                Viewport = { Width = width; Height = height } }

        // Vertical Scrolling
        | ScrollTo offset ->
            { model with
                VerticalOffset = max 0 offset }

        | ScrollBy delta ->
            { model with
                VerticalOffset = max 0 (model.VerticalOffset + delta) }

        | WorkspaceEvent _
        | AgentEvent _
        | NoOp -> model
