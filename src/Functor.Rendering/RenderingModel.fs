namespace Functor.Rendering

open Functor.Domain.Core
open Functor.Domain.Editing
open Functor.Domain.Syntax
open Functor.Domain.Diagnostics
open Functor.Domain.Navigation

/// A source range used by rendering geometry.
type Range = { Start: Position; End: Position }

module Range =

    let normalize range =
        if
            compare (range.Start.Line, range.Start.Column) (range.End.Line, range.End.Column)
            <= 0
        then
            range
        else
            { Start = range.End; End = range.Start }

/// A rectangle in viewport pixel coordinates.
type Rect =
    { X: float32
      Y: float32
      Width: float32
      Height: float32 }

type TextColorRole =
    | EditorForeground
    | SyntaxForeground of string

type TextWeight =
    | Normal
    | Bold

type TextSlant =
    | Upright
    | Italic

type TextStyle =
    { Foreground: TextColorRole
      Weight: TextWeight
      Slant: TextSlant }

module TextStyle =
    let defaultStyle =
        { Foreground = EditorForeground
          Weight = Normal
          Slant = Upright }

    let forTokenKind (kind: string) =
        let normalized = kind.Trim().ToLowerInvariant()

        { Foreground = SyntaxForeground normalized
          Weight = if normalized = "keyword" then Bold else Normal
          Slant = if normalized = "comment" then Italic else Upright }

/// Represents a single visible line in the viewport.
type VisibleLine =
    { LineIndex: int
      Text: string
      X: float32
      Y: float32
      Height: float32 }

/// A complete text segment ready for a backend to materialize.
type VisibleTextRun =
    { LineIndex: int; Text: string; Range: Range; X: float32; Y: float32; Height: float32; StartVisualColumn: int; Style: TextStyle }

/// Represents a styled token ready for rendering.
type VisibleToken =
    { LineIndex: int; Range: Range; Style: TextStyle; XStart: float32; XEnd: float32; Y: float32 }

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
        TextRuns: list<VisibleTextRun>
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
          TextRuns = []
          VisibleTokens = []
          Selections = []
          Cursors = []
          Diagnostics = []
          LineNumbers = []
          ViewportWidth = width
          ViewportHeight = height }
