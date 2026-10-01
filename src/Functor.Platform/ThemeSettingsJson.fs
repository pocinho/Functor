namespace Functor.Platform

open System
open System.Globalization
open System.Text.Json

module ThemeSettingsJson =
    let parseColor (value: string) =
        let normalized = value.Trim().TrimStart('#')

        let normalized =
            if normalized.Length = 6 then
                "FF" + normalized
            else
                normalized

        let mutable parsed = 0u

        if UInt32.TryParse(normalized, NumberStyles.HexNumber, CultureInfo.InvariantCulture, &parsed) then
            Ok parsed
        else
            Error(sprintf "Invalid theme color: %s" value)

    let tryString (element: JsonElement) (name: string) =
        let mutable property = Unchecked.defaultof<JsonElement>

        if
            element.TryGetProperty(name, &property)
            && property.ValueKind = JsonValueKind.String
        then
            property.GetString() |> Option.ofObj
        else
            None

    let tryFloat32 (element: JsonElement) (name: string) =
        let mutable property = Unchecked.defaultof<JsonElement>

        if
            element.TryGetProperty(name, &property)
            && property.ValueKind = JsonValueKind.Number
        then
            match property.TryGetSingle() with
            | true, value -> Some value
            | _ -> None
        else
            None

    let tryInt32 (element: JsonElement) (name: string) =
        let mutable property = Unchecked.defaultof<JsonElement>

        if
            element.TryGetProperty(name, &property)
            && property.ValueKind = JsonValueKind.Number
        then
            match property.TryGetInt32() with
            | true, value -> Some value
            | _ -> None
        else
            None
