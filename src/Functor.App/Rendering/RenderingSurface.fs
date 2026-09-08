namespace Functor.App.Rendering

open Avalonia
open Avalonia.Controls
open Avalonia.Media
open Avalonia.Skia
open SkiaSharp
open Functor.Rendering

/// RenderingSurface is responsible for drawing a complete frame
/// described by RenderingModel onto a Skia canvas.
///
/// IMPORTANT:
/// - This module does NOT compute geometry.
/// - It does NOT slice tokens.
/// - It does NOT layout text.
/// - It ONLY draws what RenderingModel provides.
module RenderingSurface =

    /// Draws a single frame using the provided RenderingModel.
    /// This is called by EditorSurface during OnRender.
    let draw (context: DrawingContext) (model: RenderingModel) =
        // Implementation will be added later.
        ()

    /// Draw visible lines (background + text).
    let private drawLines (canvas: SKCanvas) (model: RenderingModel) =
        // Implementation will be added later.
        ()

    /// Draw syntax-highlighted tokens.
    let private drawTokens (canvas: SKCanvas) (model: RenderingModel) =
        // Implementation will be added later.
        ()

    /// Draw selection rectangles.
    let private drawSelections (canvas: SKCanvas) (model: RenderingModel) =
        // Implementation will be added later.
        ()

    /// Draw cursor(s).
    let private drawCursors (canvas: SKCanvas) (model: RenderingModel) =
        // Implementation will be added later.
        ()

    /// Draw diagnostics (glyphs + underlines).
    let private drawDiagnostics (canvas: SKCanvas) (model: RenderingModel) =
        // Implementation will be added later.
        ()

    /// Draw line numbers in the gutter.
    let private drawLineNumbers (canvas: SKCanvas) (model: RenderingModel) =
        // Implementation will be added later.
        ()
