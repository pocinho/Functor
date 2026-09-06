namespace Functor.Rendering

open Functor.Domain.Core
open Functor.Domain.Syntax
open Functor.Domain.Diagnostics
open Functor.Domain.Navigation

/// The SlicingEngine is responsible for converting CoreModel state
/// into *visible* spans (lines, tokens, selections, diagnostics, cursors)
/// based on viewport size and scroll offsets.
///
/// IMPORTANT:
/// - All slicing is done in Position/Range space (no pixels).
/// - LayoutEngine will convert these spans into pixel geometry.
/// - RenderingModel will combine geometry into a frame description.

module SlicingEngine =

    // ─────────────────────────────────────────────────────────────
    // Visible Lines
    // ─────────────────────────────────────────────────────────────

    /// Returns the list of visible line indices and their text.
    /// This is purely based on VerticalOffset, Viewport.Height,
    /// and the buffer length.
    let sliceLines
        (model: CoreModel)
        : list<int * string> =
        // Implementation will be added later.
        []

    // ─────────────────────────────────────────────────────────────
    // Visible Tokens
    // ─────────────────────────────────────────────────────────────

    /// Returns the list of syntax tokens that intersect the visible
    /// line range and horizontal viewport.
    let sliceTokens
        (model: CoreModel)
        (syntax: SyntaxModel)
        : list<SyntaxToken> =
        // Implementation will be added later.
        []

    // ─────────────────────────────────────────────────────────────
    // Visible Selections
    // ─────────────────────────────────────────────────────────────

    /// Returns the list of selection ranges that intersect the viewport.
    let sliceSelections
        (model: CoreModel)
        : list<Range> =
        // Implementation will be added later.
        []

    // ─────────────────────────────────────────────────────────────
    // Visible Cursors
    // ─────────────────────────────────────────────────────────────

    /// Returns the list of cursor positions that are visible.
    /// (Supports multi-cursor in the future.)
    let sliceCursors
        (model: CoreModel)
        : list<Position> =
        // Implementation will be added later.
        []

    // ─────────────────────────────────────────────────────────────
    // Visible Diagnostics
    // ─────────────────────────────────────────────────────────────

    /// Returns diagnostics whose ranges intersect the visible viewport.
    let sliceDiagnostics
        (model: CoreModel)
        (diagnostics: DiagnosticsModel)
        : list<Diagnostic> =
        // Implementation will be added later.
        []

    // ─────────────────────────────────────────────────────────────
    // Combined Slicing
    // ─────────────────────────────────────────────────────────────

    /// Performs all slicing steps and returns a structured record
    /// containing all visible spans (still in Position/Range space).
    type SlicedSpans =
        {
            Lines : list<int * string>
            Tokens : list<SyntaxToken>
            Selections : list<Range>
            Cursors : list<Position>
            Diagnostics : list<Diagnostic>
        }

    /// Runs the full slicing pipeline.
    let sliceAll (model: CoreModel) : SlicedSpans =
        {
            Lines = sliceLines model
            Tokens = sliceTokens model model.Syntax
            Selections = sliceSelections model
            Cursors = sliceCursors model
            Diagnostics = sliceDiagnostics model model.Diagnostics
        }
