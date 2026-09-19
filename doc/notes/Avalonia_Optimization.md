# Avalonia Optimization Findings

These findings were checked against the local Avalonia 12.1.2 source at `D:\dev\downloads\Avalonia-12.1.2`.

## Resource Updates

`ResourceDictionary` raises `ResourcesChanged` for each indexer assignment. Its public `SetItems` method updates a collection of keys and raises one notification after the batch.

Functor's theme application writes many semantic and compatibility keys in `ThemeResources`. Batched writes reduce repeated resource lookup, style invalidation, and layout propagation during one theme change.

Guidance: build a complete resource batch for each theme layer and use `SetItems` when the application dictionary is the concrete Avalonia `ResourceDictionary`. Keep an indexer-based fallback for alternate resource-dictionary implementations.

## Resource Observables

Avalonia exposes `ResourceNodeExtensions.GetResourceObservable`. It follows resource changes and actual theme-variant changes through the public resource-host API.

Guidance: use resource observables for future code-behind controls that need one specific resource to stay current. The current themed dialog uses `ResourcesChanged` because it refreshes a small, fixed chrome surface; it can be narrowed to per-resource observables if the helper grows.

## Dialog And Window Chrome

`DynamicResourceExtension` is a XAML markup extension. Its binding expression is created through internal Avalonia APIs, so direct F# code-behind cannot apply it through the public `AvaloniaObject` API in the same way as XAML.

The public `ResourcesChanged` event is a suitable code-behind refresh point for application-owned dialog chrome. `ExtendClientAreaTitleBarHeightHint` is also available for custom client-area chrome. Functor now sets it to match the shared 32-pixel title-bar row on the main window and themed dialogs.

Headless tests can verify window properties, resource propagation, and layout structure. Native border rendering, title-bar hit testing, and platform drag behavior require desktop smoke tests.

Fluent template state setters have the same boundary: a bare headless button did not expose the application-level pointer, pressed, focus, or disabled setter values after pseudo-class activation. Keep state verification as a desktop smoke-test task unless a fully templated headless fixture is introduced.

## Renderer And Testing

Avalonia resource replacement does not automatically invalidate Functor's renderer-local font measurement caches. Clearing those caches when font family, fallback, size, or tab size changes remains necessary.

Headless font metrics may not differ for all installed font names, so cache tests should assert invalidation behavior or use a controlled fake measurement seam rather than relying only on visible metric differences.
