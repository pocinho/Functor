# Renderer Measurement Cache

## Current Finding

`Functor.Avalonia.Rendering.RenderingSurface.setUiTheme` rebuilds the editor typeface and clears both measurement caches:

- default-character advances
- grapheme advances

The cache keys are based on line height and, for graphemes, the grapheme text. Font family and fallback changes therefore require the explicit cache clear already present in `setUiTheme`.

## Test Constraint

A black-box Avalonia headless test was attempted by switching from the default editor font to another installed Windows font and measuring the same glyph before and after the theme change. The headless text platform resolved both fonts to identical metrics, so an `Assert.NotEqual` check was environment-dependent and was removed.

The production behavior remains implemented. A deterministic cache-generation diagnostic now allows tests to verify invalidation without relying on distinguishable installed-font metrics.

## Recommended Follow-Up

The focused Avalonia test uses the cache-generation diagnostic rather than relying on installed-font differences. If cache internals become instance-scoped later, retain an equivalent narrow test seam. Suitable options are:

1. Inject a small text-measurement function or typeface factory into `RenderingSurface` and use a deterministic fake whose measured width changes when the theme changes.
2. Keep an internal-only cache diagnostic or generation counter available to the test assembly, then assert that `setUiTheme` invalidates both caches.

Keep the production cache keys unchanged unless the renderer becomes instance-scoped. Restore the default theme after any test because the current `RenderingSurface` state is module-level.
