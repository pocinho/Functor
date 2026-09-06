namespace Functor.Domain.Diagnostics

open System

/// Pure diagnostics logic:
/// Applies a DiagnosticsEvent to a DiagnosticsModel and returns a new DiagnosticsModel.
module DiagnosticsLogic =

    // ────────────────────────────────────────────────
    // Full Diagnostics Update
    // ────────────────────────────────────────────────

    let private setDiagnostics (model: DiagnosticsModel) (diags: Diagnostic list) =
        DiagnosticsModel.setDiagnostics model diags

    let private clearDiagnostics (model: DiagnosticsModel) =
        DiagnosticsModel.create ()

    // ────────────────────────────────────────────────
    // Incremental Diagnostics
    // ────────────────────────────────────────────────

    let private setLineDiagnostics (model: DiagnosticsModel) (line: int) (diags: Diagnostic list) =
        // Replace diagnostics for a single line
        let filtered =
            model.All
            |> List.filter (fun d -> d.RangeStart.Line <> line)

        let newAll = diags @ filtered

        DiagnosticsModel.setDiagnostics model newAll

    let private setRangeDiagnostics (model: DiagnosticsModel) (startLine: int) (endLine: int) (diags: Diagnostic list) =
        // Remove diagnostics in the range
        let filtered =
            model.All
            |> List.filter (fun d ->
                let ln = d.RangeStart.Line
                ln < startLine || ln > endLine)

        let newAll = diags @ filtered

        DiagnosticsModel.setDiagnostics model newAll

    // ────────────────────────────────────────────────
    // Metadata
    // ────────────────────────────────────────────────

    let private updateDiagnosticsTimestamp (model: DiagnosticsModel) (ts: DateTime) =
        { model with LastUpdated = Some ts }

    // ────────────────────────────────────────────────
    // Invalidation
    // ────────────────────────────────────────────────

    let private markDiagnosticsDirty (model: DiagnosticsModel) =
        DiagnosticsModel.markDirty model

    // ────────────────────────────────────────────────
    // Main update function
    // ────────────────────────────────────────────────

    let update (evt: DiagnosticsEvent) (model: DiagnosticsModel) : DiagnosticsModel =
        match evt with
        | SetDiagnostics diags ->
            setDiagnostics model diags

        | ClearDiagnostics ->
            clearDiagnostics model

        | SetLineDiagnostics (line, diags) ->
            setLineDiagnostics model line diags

        | SetRangeDiagnostics (startLine, endLine, diags) ->
            setRangeDiagnostics model startLine endLine diags

        | UpdateDiagnosticsTimestamp ts ->
            updateDiagnosticsTimestamp model ts

        | MarkDiagnosticsDirty ->
            markDiagnosticsDirty model
