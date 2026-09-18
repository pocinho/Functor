namespace Functor.Tests.Workspace

open Xunit
open Functor.Workspace

module WorkspaceLayoutTests =
    [<Fact>]
    let ``layout document round trips through versioned json`` () =
        let layout = { SidePanelWidth = 480.0 }

        let result =
            layout
            |> WorkspaceLayout.document
            |> WorkspaceLayout.toJson
            |> WorkspaceLayout.ofJson

        Assert.Equal(Ok(WorkspaceLayout.document layout), result)

    [<Fact>]
    let ``layout parser rejects unsupported versions`` () =
        let result = WorkspaceLayout.ofJson "{\"version\":2,\"layout\":{}}"

        Assert.Equal(Error "Unsupported layout document version: 2.", result)

    [<Fact>]
    let ``layout parser rejects non finite widths`` () =
        let result =
            WorkspaceLayout.ofJson "{\"version\":1,\"layout\":{\"sidePanelWidth\":1e400}}"

        Assert.Equal(Error "Layout sidePanelWidth must be finite.", result)
