namespace Functor.Workspace

open System
open System.IO
open System.Text
open System.Text.Json

type WorkspaceLayout = { SidePanelWidth: double }

type WorkspaceLayoutDocument =
    { Version: int
      Layout: WorkspaceLayout }

module WorkspaceLayout =
    [<Literal>]
    let CurrentVersion = 1

    let defaults = { SidePanelWidth = 320.0 }

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
        writer.WriteNumber("sidePanelWidth", layoutDocument.Layout.SidePanelWidth)

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
            let mutable sidePanelWidth = Unchecked.defaultof<JsonElement>

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
                not (layout.TryGetProperty("sidePanelWidth", &sidePanelWidth))
                || sidePanelWidth.ValueKind <> JsonValueKind.Number
            then
                Error "Layout is missing a numeric sidePanelWidth value."
            elif not (Double.IsFinite(sidePanelWidth.GetDouble())) then
                Error "Layout sidePanelWidth must be finite."
            else
                Ok
                    { Version = CurrentVersion
                      Layout = { SidePanelWidth = sidePanelWidth.GetDouble() } }
        with
        | :? JsonException as error -> Error(sprintf "Invalid layout JSON: %s" error.Message)
        | :? FormatException as error -> Error(sprintf "Invalid layout value: %s" error.Message)
