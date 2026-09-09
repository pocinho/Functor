namespace Functor.Avalonia.Rendering

open Avalonia
open Avalonia.Media
open Functor.Rendering

type AvaloniaRenderBackend() =
    interface IRenderBackend<DrawingContext, Avalonia.Rect, ThemePalette> with
        member _.DrawFrame(context, bounds, model, theme) =
            RenderingSurface.draw context bounds model theme
