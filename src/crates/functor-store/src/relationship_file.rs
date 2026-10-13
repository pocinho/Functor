//! The TOML representation of a relationship: parsing and canonical rendering.
//!
//! See `docs/repository-layout.md`. Relationships are immutable in the MVP, so there is no
//! in-place edit.

use std::fmt::Display;

use functor_core::LoadIssueKind;
use functor_model::{
    ArtifactId, Metadata, MetadataKey, MetadataValue, Relationship, RelationshipId,
    RelationshipType,
};
use functor_repository::toml_support::{
    FileProblem, FileResult, optional_str, required_str, required_timestamp, string_array_value,
    timestamp_value, unknown_keys,
};
use toml_edit::{DocumentMut, Item, Table, Value};

/// The keys this release understands.
pub const RELATIONSHIP_KEYS: [&str; 6] = ["id", "type", "source", "target", "created", "metadata"];

/// A parsed relationship file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRelationship {
    /// The relationship.
    pub relationship: Relationship,
    /// Keys not understood by this release (`V0018`).
    pub unknown_fields: Vec<String>,
}

fn malformed(error: impl Display) -> FileProblem {
    FileProblem::malformed(error.to_string())
}

fn invalid_identifier(error: impl Display) -> FileProblem {
    FileProblem::new(LoadIssueKind::InvalidIdentifier, error.to_string())
}

/// Parses the text of a relationship file.
///
/// # Errors
/// A missing `id` is `MissingIdentifier`; a malformed relationship or artifact id is
/// `InvalidIdentifier`; an acyclic relationship pointing at its own source is `SelfReference`
/// (the model constructor rejects it, so it is reported rather than built); anything else wrong
/// is `MalformedFile`.
pub fn parse_relationship_file(text: &str) -> FileResult<ParsedRelationship> {
    let document: DocumentMut = text
        .parse()
        .map_err(|error| FileProblem::malformed(format!("invalid TOML: {error}")))?;
    let table = document.as_table();

    let id = match optional_str(table, "id")? {
        None => {
            return Err(FileProblem::new(
                LoadIssueKind::MissingIdentifier,
                "missing field `id`",
            ));
        }
        Some(text) => text.parse::<RelationshipId>().map_err(invalid_identifier)?,
    };
    let relationship_type =
        RelationshipType::parse(required_str(table, "type")?).map_err(malformed)?;
    let source: ArtifactId = required_str(table, "source")?
        .parse()
        .map_err(invalid_identifier)?;
    let target: ArtifactId = required_str(table, "target")?
        .parse()
        .map_err(invalid_identifier)?;
    let created = required_timestamp(table, "created")?;
    let metadata = match table.get("metadata") {
        None => Metadata::new(),
        Some(item) => parse_metadata(item)?,
    };

    if source == target && relationship_type.is_acyclic() {
        return Err(FileProblem::new(
            LoadIssueKind::SelfReference,
            format!("a `{relationship_type}` relationship cannot point at its own source {source}"),
        ));
    }
    let relationship =
        Relationship::from_parts(id, relationship_type, source, target, created, metadata)
            .map_err(malformed)?;
    Ok(ParsedRelationship {
        relationship,
        unknown_fields: unknown_keys(table, &RELATIONSHIP_KEYS),
    })
}

fn parse_metadata(item: &Item) -> FileResult<Metadata> {
    let table = item
        .as_table_like()
        .ok_or_else(|| FileProblem::malformed("`metadata` must be a table"))?;
    let mut metadata = Metadata::new();
    for (key, entry) in table.iter() {
        let name = MetadataKey::new(key).map_err(malformed)?;
        let value = match entry.as_value() {
            Some(Value::String(text)) => Some(MetadataValue::String(text.value().clone())),
            Some(Value::Integer(number)) => Some(MetadataValue::Integer(*number.value())),
            Some(Value::Boolean(flag)) => Some(MetadataValue::Bool(*flag.value())),
            Some(Value::Array(array)) => array
                .iter()
                .map(|item| item.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
                .map(MetadataValue::StringList),
            _ => None,
        }
        .ok_or_else(|| {
            FileProblem::malformed(format!(
                "metadata `{key}` must be a string, integer, boolean or array of strings"
            ))
        })?;
        metadata.insert(name, value);
    }
    Ok(metadata)
}

fn to_value(value: &MetadataValue) -> Value {
    match value {
        MetadataValue::String(text) => Value::from(text.as_str()),
        MetadataValue::Integer(number) => Value::from(*number),
        MetadataValue::Bool(flag) => Value::from(*flag),
        MetadataValue::StringList(items) => string_array_value(items.iter().map(String::as_str)),
        // `MetadataValue` is non-exhaustive; keep unknown kinds visible rather than dropping them.
        other => Value::from(format!("{other:?}")),
    }
}

/// Renders a relationship in the canonical form.
#[must_use]
pub fn render_relationship_file(relationship: &Relationship) -> String {
    let mut document = DocumentMut::new();
    let table = document.as_table_mut();
    table.insert(
        "id",
        Item::Value(Value::from(relationship.id().to_string())),
    );
    table.insert(
        "type",
        Item::Value(Value::from(relationship.relationship_type().as_str())),
    );
    table.insert(
        "source",
        Item::Value(Value::from(relationship.source().to_string())),
    );
    table.insert(
        "target",
        Item::Value(Value::from(relationship.target().to_string())),
    );
    table.insert(
        "created",
        Item::Value(timestamp_value(relationship.created())),
    );
    if !relationship.metadata().is_empty() {
        let mut metadata = Table::new();
        metadata.decor_mut().set_prefix("\n");
        for (key, value) in relationship.metadata().iter() {
            metadata.insert(key.as_str(), Item::Value(to_value(value)));
        }
        table.insert("metadata", Item::Table(metadata));
    }
    document.to_string()
}

#[cfg(test)]
mod tests {
    use functor_model::Timestamp;

    use super::*;

    const REL: &str = "rel_01J9Z3KQ0A1B2C3D4E5F6G7H8J";
    const SRC: &str = "art_01J9Z3K8M2Q7X4V6B5N1R0T8WD";
    const DST: &str = "art_01J9Z3K0P4Q5R6S7T8V9W0X1Y2";

    fn sample(kind: &str, source: &str, target: &str) -> Relationship {
        let mut metadata = Metadata::new();
        metadata.insert(
            MetadataKey::new("note").unwrap(),
            "RFC-0001 builds on the vision".into(),
        );
        Relationship::from_parts(
            REL.parse().unwrap(),
            RelationshipType::parse(kind).unwrap(),
            source.parse().unwrap(),
            target.parse().unwrap(),
            Timestamp::parse("2026-10-12T18:05:00Z").unwrap(),
            metadata,
        )
        .unwrap()
    }

    #[test]
    fn renders_the_documented_example() {
        assert_eq!(
            render_relationship_file(&sample("depends_on", SRC, DST)),
            format!(
                "id = \"{REL}\"\ntype = \"depends_on\"\nsource = \"{SRC}\"\ntarget = \"{DST}\"\ncreated = 2026-10-12T18:05:00Z\n\n[metadata]\nnote = \"RFC-0001 builds on the vision\"\n"
            )
        );
    }

    #[test]
    fn rendered_relationships_parse_back() {
        for relationship in [
            sample("depends_on", SRC, DST),
            sample("supports", DST, SRC),
            sample("references", SRC, SRC),
        ] {
            let text = render_relationship_file(&relationship);
            let parsed = parse_relationship_file(&text).unwrap();
            assert_eq!(parsed.relationship, relationship);
            assert_eq!(parsed.unknown_fields, Vec::<String>::new());
            assert_eq!(render_relationship_file(&parsed.relationship), text);
        }
    }

    #[test]
    fn the_kebab_case_alias_is_read_and_normalized() {
        let text = render_relationship_file(&sample("depends_on", SRC, DST))
            .replace("depends_on", "depends-on");
        let parsed = parse_relationship_file(&text).unwrap();
        assert_eq!(
            parsed.relationship.relationship_type(),
            &RelationshipType::DependsOn
        );
    }

    fn problem(text: &str) -> FileProblem {
        parse_relationship_file(text).unwrap_err()
    }

    #[test]
    fn problems_are_classified() {
        let good = render_relationship_file(&sample("depends_on", SRC, DST));
        assert_eq!(problem("= = =").kind, LoadIssueKind::MalformedFile);
        let without_id = good.replace(&format!("id = \"{REL}\"\n"), "");
        assert_eq!(problem(&without_id).kind, LoadIssueKind::MissingIdentifier);
        assert_eq!(
            problem(&good.replace(REL, "rel_nope")).kind,
            LoadIssueKind::InvalidIdentifier
        );
        assert_eq!(
            problem(&good.replace(&format!("source = \"{SRC}\""), "source = \"rfc-0001\"")).kind,
            LoadIssueKind::InvalidIdentifier
        );
        assert_eq!(
            problem(&good.replace(&format!("target = \"{DST}\""), "target = \"x\"")).kind,
            LoadIssueKind::InvalidIdentifier
        );
        assert_eq!(
            problem(&good.replace("depends_on", "Not Valid")).kind,
            LoadIssueKind::MalformedFile
        );
        assert_eq!(
            problem(&good.replace("created = 2026-10-12T18:05:00Z\n", "")).kind,
            LoadIssueKind::MalformedFile
        );
        assert_eq!(
            problem(&format!("{good}\n[metadata]\n")).kind,
            LoadIssueKind::MalformedFile,
            "duplicate table"
        );
    }

    #[test]
    fn self_reference_of_acyclic_types_is_reported_not_built() {
        for kind in ["depends_on", "derived_from", "extends"] {
            let text = render_relationship_file(&sample("references", SRC, SRC))
                .replace("references", kind);
            assert_eq!(problem(&text).kind, LoadIssueKind::SelfReference, "{kind}");
        }
        let allowed = render_relationship_file(&sample("references", SRC, SRC));
        assert!(parse_relationship_file(&allowed).is_ok());
    }

    #[test]
    fn unknown_fields_are_reported_but_tolerated() {
        let top = format!(
            "id = \"{REL}\"\ntype = \"depends_on\"\nsource = \"{SRC}\"\ntarget = \"{DST}\"\ncreated = 2026-10-12T18:05:00Z\nconfidence = 1\n"
        );
        assert_eq!(
            parse_relationship_file(&top).unwrap().unknown_fields,
            ["confidence"]
        );
    }

    #[test]
    fn crlf_files_parse() {
        let text = render_relationship_file(&sample("depends_on", SRC, DST)).replace('\n', "\r\n");
        assert!(parse_relationship_file(&text).is_ok());
    }
}
