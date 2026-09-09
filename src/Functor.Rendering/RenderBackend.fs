namespace Functor.Rendering

/// Minimal backend contract for platform-specific drawing.
/// The render pipeline produces a pure RenderingModel; the backend is responsible
/// for materializing it on a concrete target like Avalonia or Skia.
type IRenderBackend<'Context, 'Bounds, 'Theme> =
    abstract DrawFrame: 'Context * 'Bounds * RenderingModel * 'Theme -> unit

module RenderBackend =
    let drawFrame (backend: IRenderBackend<'Context, 'Bounds, 'Theme>) (context: 'Context) (bounds: 'Bounds) (model: RenderingModel) (theme: 'Theme) =
        backend.DrawFrame(context, bounds, model, theme)
