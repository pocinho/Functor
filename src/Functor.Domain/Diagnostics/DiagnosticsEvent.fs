namespace Functor.Domain.Diagnostics

open Functor.Domain.Editing
open System

/// Events related to diagnostics:
/// - setting diagnostics
/// - clearing diagnostics
/// - marking diagnostics dirty
/// - updating timestamps
/// - incremental updates
type DiagnosticsEvent =
    // ────────────────────────────────────────────────
    // Full Diagnostics Update
    // ────────────────────────────────────────────────
    | SetDiagnostics of diagnostics:Diagnostic list
    | ClearDiagnostics

    // ────────────────────────────────────────────────
    // Incremental Diagnostics
    // ────────────────────────────────────────────────
    | SetLineDiagnostics of line:int * diagnostics:Diagnostic list
    | SetRangeDiagnostics of startLine:int * endLine:int * diagnostics:Diagnostic list

    // ────────────────────────────────────────────────
    // Metadata
    // ────────────────────────────────────────────────
    | UpdateDiagnosticsTimestamp of timestamp:DateTime

    // ────────────────────────────────────────────────
    // Invalidation
    // ────────────────────────────────────────────────
    | MarkDiagnosticsDirty
