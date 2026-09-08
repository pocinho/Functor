namespace Functor.Core.Tests

open Functor.Domain.Diagnostics
open Xunit
open TestFixtures

type DiagnosticsTests() =
    [<Fact>]
    member _.``diagnostics are grouped by starting line``() =
        let diagnostics =
            [ diagnostic 2 "warning"; diagnostic 1 "error"; diagnostic 2 "hint" ]

        let model =
            DiagnosticsModel.create () |> DiagnosticsModel.setDiagnostics <| diagnostics

        Assert.True((diagnostics = model.All), "Diagnostics did not match")
        Assert.Equal(2, model.ByLine.Length)

        Assert.Equal(
            2,
            model.ByLine
            |> List.find (fun group -> group.Line = 2)
            |> fun group -> group.Diagnostics.Length
        )

        Assert.False(model.IsDirty)
