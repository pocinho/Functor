namespace Functor.Platform

open System
open System.Text.Json

module ThemeSchemaMigration =
    let currentVersion = 1

    let migrate (root: JsonElement) : Result<JsonElement, string> =
        let mutable themeElement = Unchecked.defaultof<JsonElement>

        let theme =
            if
                root.TryGetProperty("theme", &themeElement)
                && themeElement.ValueKind = JsonValueKind.Object
            then
                themeElement
            else
                root

        let mutable schemaVersionElement = Unchecked.defaultof<JsonElement>

        if not (theme.TryGetProperty("schemaVersion", &schemaVersionElement)) then
            Error "Theme schema warning: missing schemaVersion; theme was not loaded"
        elif schemaVersionElement.ValueKind <> JsonValueKind.Number then
            Error "Invalid theme schema version: expected an integer"
        else
            match schemaVersionElement.TryGetInt32() with
            | false, _ -> Error "Invalid theme schema version: expected an integer"
            | true, version when version > currentVersion ->
                Error(sprintf "Unsupported theme schema version: %d" version)
            | true, _ -> Ok(theme.Clone())
