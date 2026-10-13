//! Helpers for reading and editing TOML entity files with `toml_edit`.
//!
//! Shared by `functor-repository` (`repository.toml`) and `functor-store` (artifacts and
//! relationships). New files are rendered in the canonical form; updates edit the existing
//! document in place so comments, ordering, formatting and unknown keys survive
//! (ADR-0011, `docs/artifact-format.md`).

use functor_core::LoadIssueKind;
use functor_model::Timestamp;
use toml_edit::{Array, Datetime, Item, Key, Table, Value};

/// A problem found in a file, classified for validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileProblem {
    /// The classification (see `LoadIssueKind`).
    pub kind: LoadIssueKind,
    /// A human-readable explanation.
    pub detail: String,
}

impl FileProblem {
    /// A malformed-file problem.
    #[must_use]
    pub fn malformed(detail: impl Into<String>) -> Self {
        Self {
            kind: LoadIssueKind::MalformedFile,
            detail: detail.into(),
        }
    }

    /// A problem of the given kind.
    #[must_use]
    pub fn new(kind: LoadIssueKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

/// The result of reading a file.
pub type FileResult<T> = std::result::Result<T, FileProblem>;

/// Maximum width of an inline array before it is written one element per line.
pub const ARRAY_WIDTH: usize = 80;

/// Reads an optional string value.
///
/// # Errors
/// Fails when the key exists with another type.
pub fn optional_str<'a>(table: &'a Table, key: &str) -> FileResult<Option<&'a str>> {
    match table.get(key) {
        None => Ok(None),
        Some(item) => item
            .as_str()
            .map(Some)
            .ok_or_else(|| FileProblem::malformed(format!("`{key}` must be a string"))),
    }
}

/// Reads a required string value.
///
/// # Errors
/// Fails when the key is missing or has another type.
pub fn required_str<'a>(table: &'a Table, key: &str) -> FileResult<&'a str> {
    optional_str(table, key)?
        .ok_or_else(|| FileProblem::malformed(format!("missing field `{key}`")))
}

/// Reads an optional integer value.
///
/// # Errors
/// Fails when the key exists with another type.
pub fn optional_integer(table: &Table, key: &str) -> FileResult<Option<i64>> {
    match table.get(key) {
        None => Ok(None),
        Some(item) => item
            .as_integer()
            .map(Some)
            .ok_or_else(|| FileProblem::malformed(format!("`{key}` must be an integer"))),
    }
}

/// Reads a required UTC timestamp written as a TOML date-time with second precision.
///
/// # Errors
/// Fails when the key is missing, is not a date-time, or is not UTC at second precision.
pub fn required_timestamp(table: &Table, key: &str) -> FileResult<Timestamp> {
    let item = table
        .get(key)
        .ok_or_else(|| FileProblem::malformed(format!("missing field `{key}`")))?;
    let invalid = || {
        FileProblem::malformed(format!(
            "`{key}` must be a UTC date-time with second precision, like 2026-10-12T18:00:00Z"
        ))
    };
    let datetime = item.as_datetime().ok_or_else(invalid)?;
    Timestamp::parse(&datetime.to_string()).map_err(|_| invalid())
}

/// Reads a list of strings.
///
/// # Errors
/// Fails when the item is not an array of strings.
pub fn string_list(item: &Item, field: &str) -> FileResult<Vec<String>> {
    let array = item
        .as_array()
        .ok_or_else(|| FileProblem::malformed(format!("`{field}` must be an array of strings")))?;
    array
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                FileProblem::malformed(format!("`{field}` must contain only strings"))
            })
        })
        .collect()
}

/// Top-level keys that are not in `known`, in document order.
#[must_use]
pub fn unknown_keys(table: &Table, known: &[&str]) -> Vec<String> {
    table
        .iter()
        .map(|(key, _)| key)
        .filter(|key| !known.contains(key))
        .map(str::to_owned)
        .collect()
}

/// A canonical TOML date-time value, for example `2026-10-12T18:00:00Z`.
///
/// # Panics
/// Never in practice: the canonical text of a [`Timestamp`] is always a valid TOML date-time.
#[must_use]
pub fn timestamp_value(timestamp: Timestamp) -> Value {
    let datetime: Datetime = timestamp
        .to_string()
        .parse()
        .unwrap_or_else(|_| unreachable!("canonical timestamps are valid TOML date-times"));
    Value::from(datetime)
}

/// A string-array value: one line when it fits [`ARRAY_WIDTH`] columns (`["a", "b"]`), otherwise
/// one element per line.
#[must_use]
pub fn string_array_value<'a>(items: impl IntoIterator<Item = &'a str>) -> Value {
    let items: Vec<&str> = items.into_iter().collect();
    let mut array: Array = items.iter().map(|s| Value::from(*s)).collect();
    let inline = array.to_string();
    if inline.len() + "tags = ".len() > ARRAY_WIDTH {
        array.set_trailing_comma(true);
        array.set_trailing("\n");
        for value in array.iter_mut() {
            value.decor_mut().set_prefix("\n    ");
        }
    }
    Value::from(array)
}

/// Replaces the value under `key`, keeping its decoration (spacing and trailing comment), or
/// inserts it at the end of the table's key/value pairs when absent.
pub fn set_value(table: &mut Table, key: &str, value: Value) {
    if let Some(Item::Value(existing)) = table.get_mut(key) {
        let decor = existing.decor().clone();
        *existing = value;
        *existing.decor_mut() = decor;
    } else {
        table.insert(key, Item::Value(value));
    }
}

/// Like [`set_value`], but a missing key is inserted at its place in `order`: right after the last
/// earlier key of `order` that is present as a value (or first, if none is), so new keys land in
/// the canonical position and unknown keys that sit further down stay after it.
pub fn set_value_in_order(table: &mut Table, key: &str, value: Value, order: &[&str]) {
    if table.contains_key(key) {
        set_value(table, key, value);
        return;
    }
    let earlier: &[&str] = order
        .iter()
        .position(|candidate| *candidate == key)
        .map_or(&[], |index| &order[..index]);
    let value_keys: Vec<String> = table
        .iter()
        .filter(|(_, item)| item.is_value())
        .map(|(existing, _)| existing.to_owned())
        .collect();
    let moved_from = value_keys
        .iter()
        .rposition(|existing| earlier.contains(&existing.as_str()))
        .map_or(0, |index| index + 1);
    let moved: Vec<(Key, Item)> = value_keys[moved_from..]
        .iter()
        .filter_map(|existing| table.remove_entry(existing))
        .collect();
    table.insert(key, Item::Value(value));
    for (moved_key, item) in moved {
        table.insert_formatted(&moved_key, item);
    }
}

/// Removes a key together with the comment lines attached directly above it.
pub fn remove_key(table: &mut Table, key: &str) {
    table.remove(key);
}

/// Gives text produced by an in-place edit the line endings of the original file.
///
/// `toml_edit` keeps existing line endings but writes `\n` for new lines, so a CRLF file would
/// otherwise end up mixed.
#[must_use]
pub fn match_line_endings(original: &str, edited: String) -> String {
    if original.contains("\r\n") {
        edited.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        edited
    }
}

#[cfg(test)]
mod tests {
    use toml_edit::DocumentMut;

    use super::*;

    fn doc(text: &str) -> DocumentMut {
        text.parse().unwrap()
    }

    #[test]
    fn readers_report_missing_and_mistyped_fields() {
        let d = doc("a = \"x\"\nn = 3\nwhen = 2026-10-12T18:00:00Z\nlist = [\"a\", 1]\n");
        let t = d.as_table();
        assert_eq!(required_str(t, "a").unwrap(), "x");
        assert_eq!(optional_str(t, "zzz").unwrap(), None);
        assert!(
            required_str(t, "zzz")
                .unwrap_err()
                .detail
                .contains("missing field `zzz`")
        );
        assert!(
            optional_str(t, "n")
                .unwrap_err()
                .detail
                .contains("must be a string")
        );
        assert_eq!(optional_integer(t, "n").unwrap(), Some(3));
        assert!(optional_integer(t, "a").is_err());
        assert_eq!(
            required_timestamp(t, "when").unwrap(),
            Timestamp::parse("2026-10-12T18:00:00Z").unwrap()
        );
        assert!(required_timestamp(t, "a").is_err());
        assert!(string_list(t.get("list").unwrap(), "list").is_err());
    }

    #[test]
    fn timestamps_must_be_utc_at_second_precision() {
        for bad in [
            "when = 2026-10-12T18:00:00+01:00",
            "when = 2026-10-12T18:00:00.5Z",
            "when = 2026-10-12",
            "when = 18:00:00",
            "when = 2026-10-12T18:00:00",
        ] {
            let d = doc(bad);
            assert!(required_timestamp(d.as_table(), "when").is_err(), "{bad}");
        }
    }

    #[test]
    fn timestamp_values_render_unquoted() {
        let mut d = DocumentMut::new();
        let ts = Timestamp::parse("2026-10-12T18:00:00Z").unwrap();
        d.insert("created", Item::Value(timestamp_value(ts)));
        assert_eq!(d.to_string(), "created = 2026-10-12T18:00:00Z\n");
    }

    #[test]
    fn unknown_keys_are_listed_in_document_order() {
        let d = doc("a = 1\nz = 2\nb = 3\n[t]\nx = 1\n");
        assert_eq!(unknown_keys(d.as_table(), &["a"]), ["z", "b", "t"]);
    }

    #[test]
    fn short_arrays_stay_on_one_line_and_long_ones_split() {
        let mut d = DocumentMut::new();
        d.insert("tags", Item::Value(string_array_value(["a", "b", "c"])));
        assert_eq!(d.to_string(), "tags = [\"a\", \"b\", \"c\"]\n");

        let long: Vec<String> = (0..12).map(|i| format!("tag-number-{i}")).collect();
        let mut d = DocumentMut::new();
        d.insert(
            "tags",
            Item::Value(string_array_value(long.iter().map(String::as_str))),
        );
        let text = d.to_string();
        assert!(
            text.starts_with("tags = [\n    \"tag-number-0\",\n"),
            "{text}"
        );
        assert!(text.ends_with("\"tag-number-11\",\n]\n"), "{text}");
        let reparsed = doc(&text);
        assert_eq!(
            string_list(reparsed.get("tags").unwrap(), "tags").unwrap(),
            long
        );
    }

    #[test]
    fn set_value_keeps_spacing_and_trailing_comment() {
        let mut d = doc("title   =   \"old\"   # keep me\nother = 1\n");
        set_value(d.as_table_mut(), "title", Value::from("new"));
        assert_eq!(
            d.to_string(),
            "title   =   \"new\"   # keep me\nother = 1\n"
        );
    }

    #[test]
    fn set_value_in_order_inserts_at_the_canonical_position() {
        let order = ["id", "slug", "title", "description", "updated"];
        let mut d =
            doc("# header\nid = \"x\"\nslug = \"s\"\ntitle = \"t\"\nupdated = 1\nextra = true\n");
        set_value_in_order(d.as_table_mut(), "description", Value::from("d"), &order);
        assert_eq!(
            d.to_string(),
            "# header\nid = \"x\"\nslug = \"s\"\ntitle = \"t\"\ndescription = \"d\"\nupdated = 1\nextra = true\n"
        );
    }

    #[test]
    fn set_value_in_order_appends_when_nothing_follows() {
        let order = ["id", "slug", "description"];
        let mut d = doc("id = \"x\"\nslug = \"s\"\n");
        set_value_in_order(d.as_table_mut(), "description", Value::from("d"), &order);
        assert_eq!(
            d.to_string(),
            "id = \"x\"\nslug = \"s\"\ndescription = \"d\"\n"
        );
    }

    #[test]
    fn remove_key_takes_its_attached_comment_only() {
        let mut d = doc("# about a\na = 1\n# about b\nb = 2\n");
        remove_key(d.as_table_mut(), "a");
        assert_eq!(d.to_string(), "# about b\nb = 2\n");
    }

    #[test]
    fn crlf_files_keep_crlf_after_edits() {
        let original = "a = 1\r\nb = 2\r\n";
        let mut d = doc(original);
        set_value_in_order(d.as_table_mut(), "c", Value::from(3), &["a", "b", "c"]);
        let edited = match_line_endings(original, d.to_string());
        assert_eq!(edited, "a = 1\r\nb = 2\r\nc = 3\r\n");
        assert_eq!(
            match_line_endings("a = 1\n", "a = 2\n".to_owned()),
            "a = 2\n"
        );
    }
}
