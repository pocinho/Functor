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
        let editing = EditingLogic.update evt model.Editing
        let bufferChanged = editing.Revision <> model.Editing.Revision

        let activeDocument =
            if editing.IsDirty then
                model.ActiveDocument |> Option.map DocumentModel.markDirty
            else
                model.ActiveDocument

        { model with
            Editing = editing
            Syntax =
                match bufferChanged, editing.LastChange with
                | true, Some change ->
                    SyntaxModel.markDirtyRange editing.Revision change.StartLine System.Int32.MaxValue model.Syntax
                | _ -> model.Syntax
            ActiveDocument = activeDocument
            OpenDocuments =
                model.OpenDocuments
                |> List.map (fun document ->
                    if Some document = model.ActiveDocument then
                        activeDocument |> Option.defaultValue document
                    else
                        document) }

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
        | NewDocument name ->
            let doc = DocumentModel.createUntitled name
            let editing = EditingModel.create ()

            { model with
                ActiveDocument = Some doc
                OpenDocuments = doc :: model.OpenDocuments
                Editing = editing
                Syntax = SyntaxModel.createForDocument doc.Id editing.Revision
                Diagnostics = DiagnosticsModel.create () }

        | OpenDocument path ->
            let doc = DocumentModel.createFromFile path ""
            let editing = EditingModel.create ()

            { model with
                ActiveDocument = Some doc
                OpenDocuments = doc :: model.OpenDocuments
                Editing = editing
                Syntax = SyntaxModel.createForDocument doc.Id editing.Revision
                Diagnostics = DiagnosticsModel.create () }

        | LoadDocument(path, text) ->
            let doc = DocumentModel.createFromFile path text
            let editing = EditingModel.createFromText text model.Editing

            { model with
                ActiveDocument = Some doc
                OpenDocuments = doc :: model.OpenDocuments
                Editing = editing
                Syntax = SyntaxModel.createForDocument doc.Id editing.Revision
                Diagnostics = DiagnosticsModel.create () }

        | CloseDocument id ->
            let remaining = model.OpenDocuments |> List.filter (fun d -> d.Id <> id)

            let closesActiveDocument =
                model.ActiveDocument |> Option.exists (fun document -> document.Id = id)

            let newActive =
                match model.ActiveDocument with
                | Some d when d.Id = id -> remaining |> List.tryHead
                | other -> other

            { model with
                ActiveDocument = newActive
                OpenDocuments = remaining
                Editing =
                    if closesActiveDocument then
                        EditingModel.create ()
                    else
                        model.Editing
                Syntax =
                    if closesActiveDocument then
                        match newActive with
                        | Some document -> SyntaxModel.createForDocument document.Id 0L
                        | None -> SyntaxModel.create ()
                    else
                        model.Syntax
                Diagnostics =
                    if closesActiveDocument then
                        DiagnosticsModel.create ()
                    else
                        model.Diagnostics
                View =
                    if closesActiveDocument then
                        { model.View with VerticalOffset = 0 }
                    else
                        model.View }

        | SwitchDocument id ->
            let newActive = model.OpenDocuments |> List.tryFind (fun d -> d.Id = id)

            match newActive with
            | Some document when model.ActiveDocument |> Option.exists (fun active -> active.Id = document.Id) ->
                model
            | Some document ->
                let editing = EditingModel.createFromText document.InitialText model.Editing

                { model with
                    ActiveDocument = Some document
                    Editing = editing
                    Syntax = SyntaxModel.createForDocument document.Id editing.Revision
                    Diagnostics = DiagnosticsModel.create () }
            | None ->
                model

        | ApplyDocumentEvent evt ->
            let updated = applyDocumentEvent model evt

            match evt with
            | MarkDocumentClean ->
                { updated with
                    Editing =
                        { updated.Editing with
                            SavedBuffer = updated.Editing.Buffer
                            IsDirty = false } }
            | _ ->
                updated

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
                View =
                    { model.View with
                        Viewport = { Width = width; Height = height } } }

        // Vertical Scrolling
        | ScrollVerticalTo offset ->
            { model with
                View =
                    { model.View with
                        VerticalOffset = max 0 offset } }

        | ScrollVerticalBy delta ->
            { model with
                View =
                    { model.View with
                        VerticalOffset = max 0 (model.View.VerticalOffset + delta) } }

        | ScrollHorizontalTo offset ->
            { model with
                View =
                    { model.View with
                        HorizontalOffset = max 0 offset } }

        | ScrollHorizontalBy delta ->
            { model with
                View =
                    { model.View with
                        HorizontalOffset = max 0 (model.View.HorizontalOffset + delta) } }

        | WorkspaceEvent _
        | AgentEvent _
        | NoOp -> model
