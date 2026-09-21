namespace Functor.Domain.Diagnostics

open Functor.Domain.Editing
open System

/// Events related to diagnostics:
/// - setting diagnostics
/// - clearing diagnostics
/// - marking diagnostics dirty
/// - updating timestamps
/// - incremental updates
type DiagnosticsUpdateEvent =
    // ────────────────────────────────────────────────
    // Full Diagnostics Update
    // ────────────────────────────────────────────────
    | SetDiagnostics of diagnostics: Diagnostic list
    | ClearDiagnostics

type DiagnosticsIncrementalEvent =
    | SetLineDiagnostics of line: int * diagnostics: Diagnostic list
    | SetRangeDiagnostics of startLine: int * endLine: int * diagnostics: Diagnostic list

type DiagnosticsMetadataEvent = UpdateDiagnosticsTimestamp of timestamp: DateTime

type DiagnosticsInvalidationEvent = | MarkDiagnosticsDirty

type DiagnosticsEvent =
    | Update of DiagnosticsUpdateEvent
    | Incremental of DiagnosticsIncrementalEvent
    | Metadata of DiagnosticsMetadataEvent
    | Invalidation of DiagnosticsInvalidationEvent

    static member SetDiagnostics diagnostics =
        Update(DiagnosticsUpdateEvent.SetDiagnostics diagnostics)

    static member ClearDiagnostics = Update DiagnosticsUpdateEvent.ClearDiagnostics

    static member SetLineDiagnostics(line, diagnostics) =
        Incremental(DiagnosticsIncrementalEvent.SetLineDiagnostics(line, diagnostics))

    static member SetRangeDiagnostics(startLine, endLine, diagnostics) =
        Incremental(DiagnosticsIncrementalEvent.SetRangeDiagnostics(startLine, endLine, diagnostics))

    static member UpdateDiagnosticsTimestamp timestamp =
        Metadata(DiagnosticsMetadataEvent.UpdateDiagnosticsTimestamp timestamp)

    static member MarkDiagnosticsDirty =
        Invalidation DiagnosticsInvalidationEvent.MarkDiagnosticsDirty

[<AutoOpen>]
module DiagnosticsEventConstructors =
    let SetDiagnostics diagnostics =
        DiagnosticsEvent.SetDiagnostics diagnostics

    let ClearDiagnostics = DiagnosticsEvent.ClearDiagnostics

    let SetLineDiagnostics (line, diagnostics) =
        DiagnosticsEvent.SetLineDiagnostics(line, diagnostics)

    let SetRangeDiagnostics (startLine, endLine, diagnostics) =
        DiagnosticsEvent.SetRangeDiagnostics(startLine, endLine, diagnostics)

    let UpdateDiagnosticsTimestamp timestamp =
        DiagnosticsEvent.UpdateDiagnosticsTimestamp timestamp

    let MarkDiagnosticsDirty = DiagnosticsEvent.MarkDiagnosticsDirty
