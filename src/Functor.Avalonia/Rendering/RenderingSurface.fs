namespace Functor.Avalonia.Rendering

open Avalonia
open Avalonia.Controls
open Avalonia.Media
open System.Globalization
open System.Runtime.InteropServices
open System.Text
open Functor.Application
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

    let private expandTabs (tabWidth: int) (startColumn: int) (text: string) =
        let builder = StringBuilder()
        let mutable column = startColumn
        let elements = StringInfo.GetTextElementEnumerator(text)

        while elements.MoveNext() do
            let element = elements.GetTextElement()

            if element = "\t" then
                let spaces = tabWidth - (column % tabWidth)
                builder.Append(' ', spaces) |> ignore
                column <- column + spaces
            else
                builder.Append(element) |> ignore
                column <- column + 1

        builder.ToString()

    let mutable private uiTheme = UiThemeDefaults.defaultTheme

    let private editorFontFamily () =
        if RuntimeInformation.IsOSPlatform(OSPlatform.Windows) then
            sprintf "%s, %s" uiTheme.EditorFontFamily uiTheme.EditorFallbackFontFamily
        else
            uiTheme.EditorFontFamily

    let mutable private editorTypeface = Typeface(editorFontFamily ())

    let private defaultAdvanceCache =
        System.Collections.Concurrent.ConcurrentDictionary<float32, float32>()

    let private graphemeAdvanceCache =
        System.Collections.Concurrent.ConcurrentDictionary<struct (float32 * string), float32>()

    let mutable private measurementCacheGeneration = 0L

    let setUiTheme (value: UiThemeDefaults) =
        uiTheme <- value
        editorTypeface <- Typeface(editorFontFamily ())
        defaultAdvanceCache.Clear()
        graphemeAdvanceCache.Clear()
        measurementCacheGeneration <- measurementCacheGeneration + 1L

    let measurementCacheGenerationValue () = measurementCacheGeneration

    let private colorFromArgb (argb: uint32) =
        let a = byte ((argb >>> 24) &&& 0xFFu)
        let r = byte ((argb >>> 16) &&& 0xFFu)
        let g = byte ((argb >>> 8) &&& 0xFFu)
        let b = byte (argb &&& 0xFFu)
        Color.FromArgb(a, r, g, b)

    let private editorFontSize (lineHeight: float32) =
        max 1.0 (float lineHeight * (5.0 / 6.0))

    let measureDefaultAdvance (lineHeight: float32) =
        defaultAdvanceCache.GetOrAdd(
            lineHeight,
            fun lineHeight ->
                let text =
                    FormattedText(
                        "M",
                        CultureInfo.InvariantCulture,
                        FlowDirection.LeftToRight,
                        editorTypeface,
                        editorFontSize lineHeight,
                        SolidColorBrush(colorFromArgb uiTheme.MeasurementColor)
                    )

                float32 text.Width
        )

    let measureGraphemeAdvance (lineHeight: float32) (column: int) (grapheme: string) =
        if grapheme = "\t" then
            float32 (uiTheme.EditorTabSize - (column % uiTheme.EditorTabSize))
            * measureDefaultAdvance lineHeight
        else
            graphemeAdvanceCache.GetOrAdd(
                struct (lineHeight, grapheme),
                fun struct (lineHeight, grapheme) ->
                    // Anchor the grapheme between non-whitespace characters: Avalonia trims the
                    // advance of a standalone whitespace-only FormattedText string to zero.
                    let formatted (value: string) =
                        FormattedText(
                            value,
                            CultureInfo.InvariantCulture,
                            FlowDirection.LeftToRight,
                            editorTypeface,
                            editorFontSize lineHeight,
                            SolidColorBrush(colorFromArgb uiTheme.MeasurementColor)
                        )
                            .Width

                    float32 (formatted ("x" + grapheme + "x") - formatted "xx")
            )

    let private resolvedGutterWidth (model: RenderingModel) =
        let textRightEdge (lineNumber: LineNumber) =
            float lineNumber.X
            + float (measureDefaultAdvance (float32 uiTheme.EditorLineHeight))
              * float lineNumber.Text.Length
            + uiTheme.GutterPadding

        model.LineNumbers
        |> List.map textRightEdge
        |> List.fold max 0.0
        |> max uiTheme.GutterMinimumWidth

    /// Draws a single frame using the provided RenderingModel.
    /// This is called by EditorSurface during OnRender.
    let draw (context: DrawingContext) (bounds: Avalonia.Rect) (model: RenderingModel) (theme: ThemePalette) =
        use clip = context.PushClip(bounds)

        let borderWidth =
            match theme.EditorBorder with
            | Some _ -> max 0.0f theme.EditorBorderWidth
            | None -> 0.0f

        let contentBounds = bounds.Deflate(float borderWidth)

        // ------------------------------------------------------------
        // 1. Draw background
        // ------------------------------------------------------------
        let background = SolidColorBrush(colorFromArgb theme.Background)
        context.FillRectangle(background, bounds)

        match theme.EditorBorder with
        | Some color when borderWidth > 0.0f ->
            let borderPen = Pen(SolidColorBrush(colorFromArgb color), float borderWidth)
            let borderRect = bounds.Deflate(float borderWidth / 2.0)
            context.DrawRectangle(borderPen, borderRect)
        | _ -> ()

        let contentTransform = Matrix.CreateTranslation(contentBounds.X, contentBounds.Y)
        use transform = context.PushTransform(contentTransform)

        use contentClip =
            context.PushClip(Rect(0.0, 0.0, contentBounds.Width, contentBounds.Height))

        let gutterWidth = resolvedGutterWidth model

        let gutterBrush = SolidColorBrush(colorFromArgb theme.GutterBackground)
        context.FillRectangle(gutterBrush, Rect(0.0, 0.0, gutterWidth, contentBounds.Height))

        match theme.GutterSeparator with
        | Some color ->
            let separatorPen =
                Pen(SolidColorBrush(colorFromArgb color), uiTheme.GutterSeparatorWidth)

            context.DrawLine(separatorPen, Point(gutterWidth, 0.0), Point(gutterWidth, contentBounds.Height))
        | None -> ()

        let lineNumberBrush = SolidColorBrush(colorFromArgb theme.LineNumber)

        for lineNumber in model.LineNumbers do
            let text =
                FormattedText(
                    lineNumber.Text,
                    CultureInfo.InvariantCulture,
                    FlowDirection.LeftToRight,
                    editorTypeface,
                    editorFontSize (
                        model.VisibleLines
                        |> List.tryHead
                        |> Option.map (fun line -> line.Height)
                        |> Option.defaultValue 16.0f
                    ),
                    lineNumberBrush
                )

            context.DrawText(text, Point(float lineNumber.X, float lineNumber.Y))

        use editorContentClip =
            context.PushClip(Rect(gutterWidth, 0.0, max 0.0 (contentBounds.Width - gutterWidth), contentBounds.Height))

        // ------------------------------------------------------------
        // 2. Selection(s)
        // ------------------------------------------------------------
        let selectionBrush = SolidColorBrush(colorFromArgb theme.Selection)

        for selection in model.Selections do
            for rect in selection.Rects do
                context.FillRectangle(
                    selectionBrush,
                    Rect(float rect.X, float rect.Y, float rect.Width, float rect.Height)
                )

        let drawTextRun (run: VisibleTextRun) =
            let fontSize = editorFontSize run.Height

            let foreground =
                SolidColorBrush(colorFromArgb (Theme.resolveTextColor theme run.Style.Foreground))

            let fontWeight =
                match run.Style.Weight with
                | TextWeight.Normal -> FontWeight.Normal
                | TextWeight.Bold -> FontWeight.Bold

            let fontStyle =
                match run.Style.Slant with
                | TextSlant.Upright -> FontStyle.Normal
                | TextSlant.Italic -> FontStyle.Italic

            let typeface = Typeface(editorTypeface.FontFamily, fontStyle, fontWeight)

            let renderedText = expandTabs uiTheme.EditorTabSize run.StartVisualColumn run.Text

            let text =
                FormattedText(
                    renderedText,
                    CultureInfo.InvariantCulture,
                    FlowDirection.LeftToRight,
                    typeface,
                    fontSize,
                    foreground
                )

            context.DrawText(text, Point(float run.X, float run.Y))

        for run in model.TextRuns do
            drawTextRun run

        // ------------------------------------------------------------
        // 3. Cursor(s)
        // ------------------------------------------------------------
        let cursorPen =
            Pen(SolidColorBrush(colorFromArgb theme.Cursor), uiTheme.CursorWidth)

        for cursor in model.Cursors do
            let x = float cursor.X
            let y = float cursor.Y
            let h = float cursor.Height

            context.DrawLine(cursorPen, Point(x, y), Point(x, y + h))
