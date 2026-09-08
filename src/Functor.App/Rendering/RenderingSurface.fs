namespace Functor.App.Rendering

open Avalonia
open Avalonia.Controls
open Avalonia.Media
open System.Globalization
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
    let draw (context: DrawingContext) (bounds: Avalonia.Rect) (model: RenderingModel) =
        use clip = context.PushClip(bounds)

        // ------------------------------------------------------------
        // 1. Draw background
        // ------------------------------------------------------------
        let background = Brushes.Black
        context.FillRectangle(background, bounds)

        /// Debug border
        let borderPen = Pen(Brushes.Gray, 10.0)
        context.DrawRectangle(borderPen, bounds)

        // ------------------------------------------------------------
        // 2. Selection(s)
        // ------------------------------------------------------------
        let selectionBrush = SolidColorBrush(Color.FromArgb(96uy, 70uy, 130uy, 220uy))

        for selection in model.Selections do
            for rect in selection.Rects do
                context.FillRectangle(
                    selectionBrush,
                    Rect(float rect.X, float rect.Y, float rect.Width, float rect.Height)
                )

        let drawText (line: VisibleLine) =
            let fontSize = max 1.0 (float line.Height * 0.8)

            let text =
                FormattedText(
                    line.Text,
                    CultureInfo.InvariantCulture,
                    FlowDirection.LeftToRight,
                    Typeface("Consolas"),
                    fontSize,
                    Brushes.White
                )

            context.DrawText(text, Point(float line.X, float line.Y))

        for line in model.VisibleLines do
            drawText line

        // ------------------------------------------------------------
        // 3. Cursor(s)
        // ------------------------------------------------------------
        let cursorPen = Pen(Brushes.White, 1.5)

        for cursor in model.Cursors do
            let x = float cursor.X
            let y = float cursor.Y
            let h = float cursor.Height

            context.DrawLine(cursorPen, Point(x, y), Point(x, y + h))
