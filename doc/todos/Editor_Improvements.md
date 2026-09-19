# Editor Improvements

## Unicode Text Baseline

### Finding

The original U+2011 rendering issue was caused by drawing each styled token as an independent Avalonia `FormattedText` instance at the same top-left Y coordinate. Fallback fonts and font styles can expose different internal baseline metrics, so equal geometry does not guarantee equal visual alignment.

The previous workaround replaced U+2011 with ASCII `-` during measurement and drawing. It kept the source document unchanged, but it was character-specific and could not protect other Unicode fallback glyphs.

### Generalized Fix

- Preserve the original Unicode text in the document, ranges, editing model, and measurements.
- Use Avalonia `FormattedText.Baseline` to calculate a shared line baseline.
- Draw every styled run with a baseline correction relative to the normal editor typeface.
- Keep line geometry, cursor positions, hit testing, and UTF-16 offsets in the existing rendering layer.
- Do not normalize or substitute individual Unicode characters.

### Implementation Slices

- [x] Replace the U+2011-specific render substitution with shared baseline alignment.
- [x] Keep grapheme measurement based on the original Unicode text.
- [x] Add regression coverage for the exact non-breaking hyphen input.
- [ ] Add pixel-level headless coverage for fallback-font glyphs when a stable cross-platform raster assertion is available.
- [x] Manually verify the README line and representative emoji, CJK, combining-mark, and symbol text on desktop platforms; live application smoke test passed.

### Validation

The focused Avalonia and rendering tests pass after baseline alignment. Live desktop smoke testing also passed for the README line and representative Unicode glyphs; fallback font availability and rasterization remain platform-specific considerations for future platforms.
