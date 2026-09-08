namespace Functor.Rendering

open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Domain.Diagnostics
open Functor.Domain.Navigation

/// A source range used by rendering geometry.
type Range = { Start: Position; End: Position }

/// A rectangle in viewport pixel coordinates.
type Rect =
    { X: float32
      Y: float32
      Width: float32
      Height: float32 }

/// The style assigned to a syntax token.
type SyntaxStyle = { Kind: string }

/// Represents a single visible line in the viewport.
type VisibleLine =
    { LineIndex: int
      Text: string
      Y: float32
      Height: float32 }

/// Represents a styled token ready for rendering.
type VisibleToken =
    { LineIndex: int
      Range: Range
      Style: SyntaxStyle
      XStart: float32
      XEnd: float32
      Y: float32 }

/// Represents the geometry of a selection range.
type SelectionGeometry = { Range: Range; Rects: list<Rect> }

/// Represents the geometry of a cursor.
type CursorGeometry =
    { Position: Position
      X: float32
      Y: float32
      Height: float32
      Width: float32 }

/// Represents diagnostics rendered in the viewport.
type VisibleDiagnostic =
    { Range: Range
      Severity: DiagnosticSeverity
      Glyph: Rect option
      Underline: list<Rect> }

/// Represents line numbers rendered in the gutter.
type LineNumber =
    { LineIndex: int
      Text: string
      X: float32
      Y: float32 }

/// The complete rendering description for a single frame.
/// This is consumed by RenderingSurface (Skia + Avalonia).
type RenderingModel =
    {
        VisibleLines: list<VisibleLine>
        VisibleTokens: list<VisibleToken>
        Selections: list<SelectionGeometry>
        Cursors: list<CursorGeometry>
        Diagnostics: list<VisibleDiagnostic>
        LineNumbers: list<LineNumber>

        /// Pixel width/height of the viewport.
        ViewportWidth: float32
        ViewportHeight: float32
    }

module RenderingModel =

    /// Creates an empty rendering model.
    let empty width height =
        { VisibleLines = []
          VisibleTokens = []
          Selections = []
          Cursors = []
          Diagnostics = []
          LineNumbers = []
          ViewportWidth = width
          ViewportHeight = height }
