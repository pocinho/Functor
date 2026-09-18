namespace Functor.Tests.Workspace

open Xunit
open Functor.Workspace

module WorkspaceLayoutTests =
    let private persistedLayout =
        { SidePanelWidth = 480.0
          ActiveToolId = Some "search"
          IsToolPanelOpen = true }

    [<Fact>]
    let ``layout document round trips through versioned json`` () =
        let result =
            persistedLayout
            |> WorkspaceLayout.document
            |> WorkspaceLayout.toJson
            |> WorkspaceLayout.ofJson

        Assert.Equal(Ok(WorkspaceLayout.document persistedLayout), result)

    [<Fact>]
    let ``layout parser rejects unsupported versions`` () =
        let result = WorkspaceLayout.ofJson "{\"version\":2,\"layout\":{}}"

        Assert.Equal(Error "Unsupported layout document version: 2.", result)

    [<Fact>]
    let ``layout parser rejects non finite widths`` () =
        let result =
            WorkspaceLayout.ofJson "{\"version\":1,\"layout\":{\"sidePanelWidth\":1e400}}"

        Assert.Equal(Error "Layout sidePanelWidth must be finite.", result)

    [<Fact>]
    let ``layout parser defaults tool state for older documents`` () =
        let result =
            WorkspaceLayout.ofJson "{\"version\":1,\"layout\":{\"sidePanelWidth\":480}}"

        let expected =
            WorkspaceLayout.document
                { SidePanelWidth = 480.0
                  ActiveToolId = None
                  IsToolPanelOpen = false }

        Assert.Equal(Ok expected, result)
