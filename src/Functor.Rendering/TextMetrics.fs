namespace Functor.Rendering

type TextMetrics =
    { LineHeight: float32
      DefaultAdvance: float32
      TabWidth: int }

module TextMetrics =
    let create lineHeight defaultAdvance tabWidth =
        { LineHeight = lineHeight
          DefaultAdvance = defaultAdvance
          TabWidth = max 1 tabWidth }
