//! The event wire format: one JSON object per line (`docs/design/event-model-mvp.md`, "Envelope").
//!
//! The wire DTO lives here, not in the model (ADR-0005). Keys are written in a fixed order.

use std::collections::BTreeMap;
use std::str::FromStr;

use functor_model::{
    Event, EventData, EventId, EventKind, EventSubject, MetadataValue, RepositoryId, Timestamp,
};
use serde::Serialize;
use serde_json::Value;

/// The envelope version this crate writes and reads.
pub const ENVELOPE_VERSION: u64 = 1;

#[derive(Serialize)]
struct Envelope {
    v: u64,
    id: String,
    seq: u64,
    at: String,
    r#type: &'static str,
    origin: String,
    subject: String,
    data: BTreeMap<String, Value>,
    metadata: BTreeMap<String, Value>,
}

fn json_value(value: &MetadataValue) -> Value {
    match value {
        MetadataValue::String(text) => Value::from(text.as_str()),
        MetadataValue::Integer(number) => Value::from(*number),
        MetadataValue::Bool(flag) => Value::from(*flag),
        MetadataValue::StringList(items) => Value::from(items.clone()),
        // `MetadataValue` is non-exhaustive; keep an unknown kind readable rather than lose it.
        other => Value::from(format!("{other:?}")),
    }
}

/// Encodes an event as one line of JSON, without the trailing newline.
///
/// The output is deterministic: fixed envelope key order and `data` keys in ascending order.
#[must_use]
pub fn encode_event(event: &Event) -> String {
    let envelope = Envelope {
        v: ENVELOPE_VERSION,
        id: event.id.to_string(),
        seq: event.seq,
        at: event.at.to_string(),
        r#type: event.kind.as_str(),
        origin: event.origin.canonical(),
        subject: event.subject.canonical(),
        data: event
            .data
            .iter()
            .map(|(key, value)| (key.to_owned(), json_value(value)))
            .collect(),
        metadata: BTreeMap::new(),
    };
    // Serializing plain strings, numbers and maps cannot fail.
    serde_json::to_string(&envelope).unwrap_or_default()
}

/// Why a line is not a usable event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineProblem {
    /// Not JSON, or a required field is missing or invalid.
    Malformed(String),
    /// A valid envelope of a version this reader does not know; ignored.
    UnknownVersion(u64),
    /// A valid envelope with an event type this reader does not know; ignored.
    UnknownType(String),
}

fn field<'a>(
    object: &'a serde_json::Map<String, Value>,
    name: &str,
) -> Result<&'a Value, LineProblem> {
    object
        .get(name)
        .ok_or_else(|| LineProblem::Malformed(format!("missing field `{name}`")))
}

fn text<'a>(
    object: &'a serde_json::Map<String, Value>,
    name: &str,
) -> Result<&'a str, LineProblem> {
    field(object, name)?
        .as_str()
        .ok_or_else(|| LineProblem::Malformed(format!("field `{name}` must be a string")))
}

fn parse_data(value: Option<&Value>) -> EventData {
    let mut data = EventData::new();
    if let Some(Value::Object(map)) = value {
        for (key, value) in map {
            let converted = match value {
                Value::String(text) => Some(MetadataValue::String(text.clone())),
                Value::Number(number) => number.as_i64().map(MetadataValue::Integer),
                Value::Bool(flag) => Some(MetadataValue::Bool(*flag)),
                Value::Array(items) => items
                    .iter()
                    .map(|item| item.as_str().map(str::to_owned))
                    .collect::<Option<Vec<_>>>()
                    .map(MetadataValue::StringList),
                // Unknown value shapes are ignored, like unknown keys.
                _ => None,
            };
            if let Some(converted) = converted {
                data = data.with(key, converted);
            }
        }
    }
    data
}

/// Decodes one line (without its newline).
///
/// # Errors
/// [`LineProblem`] explains why the line is not a usable event.
pub fn decode_event(line: &str) -> Result<Event, LineProblem> {
    let value: Value =
        serde_json::from_str(line).map_err(|error| LineProblem::Malformed(error.to_string()))?;
    let Value::Object(object) = value else {
        return Err(LineProblem::Malformed("not a JSON object".to_owned()));
    };

    let version = field(&object, "v")?
        .as_u64()
        .ok_or_else(|| LineProblem::Malformed("field `v` must be a number".to_owned()))?;
    if version != ENVELOPE_VERSION {
        return Err(LineProblem::UnknownVersion(version));
    }

    let malformed = |what: &str, error: &dyn std::fmt::Display| {
        LineProblem::Malformed(format!("invalid {what}: {error}"))
    };
    let seq = field(&object, "seq")?
        .as_u64()
        .ok_or_else(|| LineProblem::Malformed("field `seq` must be a number".to_owned()))?;
    let id = EventId::from_str(text(&object, "id")?).map_err(|e| malformed("id", &e))?;
    let at = Timestamp::parse(text(&object, "at")?).map_err(|e| malformed("timestamp", &e))?;
    let kind_name = text(&object, "type")?;
    let origin = RepositoryId::from_canonical(text(&object, "origin")?)
        .map_err(|e| malformed("origin", &e))?;
    let subject = EventSubject::from_canonical(text(&object, "subject")?)
        .map_err(|e| malformed("subject", &e))?;
    let kind =
        EventKind::parse(kind_name).map_err(|_| LineProblem::UnknownType(kind_name.to_owned()))?;

    let new = functor_model::NewEvent::new(kind, subject, parse_data(object.get("data")));
    Event::new(id, seq, at, origin, new).map_err(|e| malformed("sequence number", &e))
}

/// The sequence number of a line, if it has one, without checking the rest of the envelope.
///
/// Writers use it to continue the numbering even past events a reader would ignore.
#[must_use]
pub fn peek_seq(line: &str) -> Option<u64> {
    let value: Value = serde_json::from_str(line).ok()?;
    value.get("seq")?.as_u64().filter(|seq| *seq >= 1)
}

/// The sequence number and id of a line. Followers use the pair to tell whether a log file was
/// replaced: a reset log can start at the same sequence number but never with the same event id.
#[must_use]
pub fn peek_key(line: &str) -> Option<(u64, String)> {
    let value: Value = serde_json::from_str(line).ok()?;
    let seq = value.get("seq")?.as_u64().filter(|seq| *seq >= 1)?;
    let id = value.get("id").and_then(Value::as_str).unwrap_or_default();
    Some((seq, id.to_owned()))
}

#[cfg(test)]
mod tests {
    use functor_model::{ArtifactId, NewEvent, RelationshipId};

    use super::*;

    fn at() -> Timestamp {
        Timestamp::parse("2026-10-12T18:05:00Z").unwrap()
    }

    fn event(seq: u64) -> Event {
        Event::new(
            EventId::from_bits(7),
            seq,
            at(),
            RepositoryId::from_bits(3),
            NewEvent::new(
                EventKind::RelationshipCreated,
                EventSubject::Relationship(RelationshipId::from_bits(9)),
                EventData::new()
                    .with("relationship_type", "depends_on")
                    .with("source", ArtifactId::from_bits(1).to_string().as_str())
                    .with("revision", 2_i64)
                    .with("strict", true)
                    .with("changed", vec!["title".to_owned(), "tags".to_owned()]),
            ),
        )
        .unwrap()
    }

    #[test]
    fn encoding_uses_the_documented_key_order_and_shape() {
        let line = encode_event(&event(3));
        let keys: Vec<&str> = [
            "\"v\"",
            "\"id\"",
            "\"seq\"",
            "\"at\"",
            "\"type\"",
            "\"origin\"",
            "\"subject\"",
            "\"data\"",
            "\"metadata\"",
        ]
        .to_vec();
        let positions: Vec<usize> = keys.iter().map(|k| line.find(k).unwrap()).collect();
        let mut sorted = positions.clone();
        sorted.sort_unstable();
        assert_eq!(positions, sorted, "{line}");
        assert!(line.starts_with("{\"v\":1,\"id\":\"evt_"));
        assert!(line.contains("\"at\":\"2026-10-12T18:05:00Z\""));
        assert!(line.contains("\"type\":\"relationship.created\""));
        assert!(line.contains("\"origin\":\"functor:repository:repo_"));
        assert!(line.contains("\"subject\":\"functor:relationship:rel_"));
        assert!(line.ends_with("\"metadata\":{}}"));
        assert!(!line.contains('\n'));
    }

    #[test]
    fn data_keys_are_written_in_ascending_order() {
        let line = encode_event(&event(3));
        let data = &line[line.find("\"data\"").unwrap()..];
        let order: Vec<usize> = [
            "changed",
            "relationship_type",
            "revision",
            "source",
            "strict",
        ]
        .iter()
        .map(|k| data.find(&format!("\"{k}\"")).unwrap())
        .collect();
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(order, sorted);
    }

    #[test]
    fn events_round_trip() {
        let original = event(42);
        assert_eq!(decode_event(&encode_event(&original)).unwrap(), original);
    }

    #[test]
    fn unknown_keys_are_tolerated_everywhere() {
        let line = encode_event(&event(1));
        let extended = line.replacen(
            "\"data\":{",
            "\"future\":[1,2],\"data\":{\"extra\":{\"x\":1},",
            1,
        );
        let decoded = decode_event(&extended).unwrap();
        assert_eq!(decoded, event(1), "unknown keys and shapes are ignored");
    }

    #[test]
    fn missing_data_and_metadata_are_fine() {
        let explicit = "{\"v\":1,\"id\":\"evt_00000000000000000000000007\",\"seq\":1,\"at\":\"2026-10-12T18:05:00Z\",\"type\":\"artifact.created\",\"origin\":\"functor:repository:repo_00000000000000000000000003\",\"subject\":\"functor:artifact:art_00000000000000000000000001\"}";
        let decoded = decode_event(explicit).unwrap();
        assert_eq!(decoded.kind, EventKind::ArtifactCreated);
        assert!(decoded.data.is_empty());
    }

    #[test]
    fn unknown_versions_and_types_are_distinguished_from_garbage() {
        let line = encode_event(&event(1));
        assert_eq!(
            decode_event(&line.replacen("\"v\":1", "\"v\":2", 1)),
            Err(LineProblem::UnknownVersion(2))
        );
        assert_eq!(
            decode_event(&line.replacen("relationship.created", "artifact.deleted", 1)),
            Err(LineProblem::UnknownType("artifact.deleted".to_owned()))
        );
        for garbage in [
            "",
            "not json",
            "[1]",
            "{}",
            "{\"v\":1}",
            "{\"v\":\"1\"}",
            "{\"v\":1,\"seq\":0}",
        ] {
            assert!(
                matches!(decode_event(garbage), Err(LineProblem::Malformed(_))),
                "{garbage:?}"
            );
        }
        let bad_seq = line.replacen("\"seq\":1", "\"seq\":0", 1);
        assert!(matches!(
            decode_event(&bad_seq),
            Err(LineProblem::Malformed(_))
        ));
    }

    #[test]
    fn peek_seq_reads_only_the_sequence_number() {
        assert_eq!(peek_seq(&encode_event(&event(17))), Some(17));
        assert_eq!(
            peek_seq("{\"v\":9,\"seq\":5,\"type\":\"whatever\"}"),
            Some(5)
        );
        assert_eq!(peek_seq("{\"seq\":0}"), None);
        assert_eq!(peek_seq("{\"seq\":\"3\"}"), None);
        assert_eq!(peek_seq("garbage"), None);
        assert_eq!(peek_seq("{\"seq\":3,\"tr"), None);
    }
}
