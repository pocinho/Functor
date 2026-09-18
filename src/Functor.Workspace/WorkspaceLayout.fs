namespace Functor.Workspace

open System
open System.IO
open System.Text
open System.Text.Json

type WorkspaceLayout =
    { IsSidePanelOpen: bool
      SidePanelWidth: double
      ActivePanel: string option }

type WorkspaceLayoutDocument =
    { Version: int
      Layout: WorkspaceLayout }

module WorkspaceLayout =
    [<Literal>]
    let CurrentVersion = 1

    let defaults =
        { IsSidePanelOpen = false
          SidePanelWidth = 320.0
          ActivePanel = None }

    let document layout =
        { Version = CurrentVersion
          Layout = layout }

    let toJson (layoutDocument: WorkspaceLayoutDocument) =
        use stream = new MemoryStream()
        use writer = new Utf8JsonWriter(stream)

        writer.WriteStartObject()
        writer.WriteNumber("version", layoutDocument.Version)
        writer.WritePropertyName("layout")
        writer.WriteStartObject()
        writer.WriteBoolean("isSidePanelOpen", layoutDocument.Layout.IsSidePanelOpen)
        writer.WriteNumber("sidePanelWidth", layoutDocument.Layout.SidePanelWidth)

        match layoutDocument.Layout.ActivePanel with
        | Some panel -> writer.WriteString("activePanel", panel)
        | None -> writer.WriteNull("activePanel")

        writer.WriteEndObject()
        writer.WriteEndObject()
        writer.Flush()
        Encoding.UTF8.GetString(stream.ToArray())

    let ofJson (json: string) =
        try
            use document = JsonDocument.Parse(json)
            let root = document.RootElement
            let mutable version = Unchecked.defaultof<JsonElement>
            let mutable layout = Unchecked.defaultof<JsonElement>
            let mutable isSidePanelOpen = Unchecked.defaultof<JsonElement>
            let mutable sidePanelWidth = Unchecked.defaultof<JsonElement>
            let mutable activePanel = Unchecked.defaultof<JsonElement>

            if
                not (root.TryGetProperty("version", &version))
                || version.ValueKind <> JsonValueKind.Number
            then
                Error "Layout document is missing a numeric version."
            elif version.GetInt32() <> CurrentVersion then
                Error(sprintf "Unsupported layout document version: %d." (version.GetInt32()))
            elif
                not (root.TryGetProperty("layout", &layout))
                || layout.ValueKind <> JsonValueKind.Object
            then
                Error "Layout document is missing an object layout."
            elif
                not (layout.TryGetProperty("isSidePanelOpen", &isSidePanelOpen))
                || (isSidePanelOpen.ValueKind <> JsonValueKind.True
                    && isSidePanelOpen.ValueKind <> JsonValueKind.False)
            then
                Error "Layout is missing a boolean isSidePanelOpen value."
            elif
                not (layout.TryGetProperty("sidePanelWidth", &sidePanelWidth))
                || sidePanelWidth.ValueKind <> JsonValueKind.Number
            then
                Error "Layout is missing a numeric sidePanelWidth value."
            elif not (Double.IsFinite(sidePanelWidth.GetDouble())) then
                Error "Layout sidePanelWidth must be finite."
            elif
                not (layout.TryGetProperty("activePanel", &activePanel))
                || (activePanel.ValueKind <> JsonValueKind.Null
                    && activePanel.ValueKind <> JsonValueKind.String)
            then
                Error "Layout activePanel must be a string or null."
            else
                let panel =
                    if activePanel.ValueKind = JsonValueKind.Null then
                        None
                    else
                        Some(activePanel.GetString())

                Ok
                    { Version = CurrentVersion
                      Layout =
                        { IsSidePanelOpen = isSidePanelOpen.GetBoolean()
                          SidePanelWidth = sidePanelWidth.GetDouble()
                          ActivePanel = panel } }
        with
        | :? JsonException as error -> Error(sprintf "Invalid layout JSON: %s" error.Message)
        | :? FormatException as error -> Error(sprintf "Invalid layout value: %s" error.Message)
