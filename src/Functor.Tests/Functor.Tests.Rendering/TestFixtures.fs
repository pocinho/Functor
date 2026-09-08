namespace Functor.Tests.Rendering

open Functor.Rendering

module TestFixtures =
    let createMeasurer () =
        TextMeasurer.create
            { LineHeight = 16.0f
              DefaultAdvance = 8.0f
              TabWidth = 4 }
