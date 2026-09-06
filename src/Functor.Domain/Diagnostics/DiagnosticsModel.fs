namespace Functor.Domain.Diagnostics

open Functor.Domain.Editing
open System

/// Severity of a diagnostic message.
type DiagnosticSeverity =
    | Error
    | Warning
    | Information
    | Hint

/// Represents a diagnostic message associated with a document.
/// This mirrors LSP-style diagnostics but stays pure and platform-agnostic.
type Diagnostic =
    {
        Severity : DiagnosticSeverity
        Message : string
        RangeStart : Position
        RangeEnd : Position
        Code : string option
        Source : string option
    }

/// Diagnostics grouped by line for fast rendering.
type LineDiagnostics =
    {
        Line : int
        Diagnostics : Diagnostic list
    }

/// Represents the diagnostics state of a document:
/// - full diagnostic list
/// - per-line buckets
/// - last update timestamp
/// - dirty flag (needs refresh)
type DiagnosticsModel =
    {
        /// All diagnostics for the document.
        All : Diagnostic list

        /// Diagnostics grouped by line.
        ByLine : LineDiagnostics list

        /// Timestamp of last diagnostics update.
        LastUpdated : DateTime option

        /// Whether diagnostics are outdated (e.g., after edits).
        IsDirty : bool
    }

module DiagnosticsModel =

    /// Empty diagnostics state.
    let create () =
        {
            All = []
            ByLine = []
            LastUpdated = None
            IsDirty = false
        }

    /// Marks diagnostics as needing refresh.
    let markDirty (model: DiagnosticsModel) =
        { model with IsDirty = true }

    /// Sets the full diagnostics list and regenerates line buckets.
    let setDiagnostics (model: DiagnosticsModel) (diags: Diagnostic list) =
        let grouped =
            diags
            |> List.groupBy (fun d -> d.RangeStart.Line)
            |> List.map (fun (line, ds) -> { Line = line; Diagnostics = ds })

        {
            model with
                All = diags
                ByLine = grouped
                LastUpdated = Some DateTime.UtcNow
                IsDirty = false
        }
