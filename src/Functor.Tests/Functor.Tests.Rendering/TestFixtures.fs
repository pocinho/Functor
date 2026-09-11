namespace Functor.Tests.Rendering

open Functor.Rendering

module TestFixtures =
    let createMeasurer () =
        TextMeasurer.create
            (TextMetrics.create 16.0f 8.0f 4)
