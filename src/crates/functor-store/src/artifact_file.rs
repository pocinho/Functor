//! The TOML representation of an artifact: parsing, canonical rendering and in-place edits.
//!
//! See `docs/artifact-format.md` and ADR-0011. New files use the canonical form; updates edit the
//! existing document so comments, key order, whitespace and unknown keys are preserved.

use std::fmt::Display;

use functor_core::LoadIssueKind;
use functor_model::{
    Artifact, ArtifactField, ArtifactId, ArtifactType, Content, ContentFormat, ContentPath,
    Metadata, MetadataKey, MetadataValue, Revision, Slug, Tag, Tags, Title,
};
use functor_repository::toml_support::{
    FileProblem, FileResult, match_line_endings, optional_integer, optional_str, remove_key,
    required_str, required_timestamp, set_value, set_value_in_order, string_array_value,
    string_list, timestamp_value, unknown_keys,
};
use toml_edit::{DocumentMut, Item, Table, TableLike, Value};

/// The keys this release understands.
pub const ARTIFACT_KEYS: [&str; 11] = [
    "id",
    "slug",
    "type",
    "title",
    "revision",
    "description",
    "created",
    "updated",
    "tags",
    "metadata",
    "content",
];

/// Canonical order of the plain (non-table) keys.
const VALUE_ORDER: [&str; 9] = [
    "id",
    "slug",
    "type",
    "title",
    "revision",
    "description",
    "created",
    "updated",
    "tags",
];

/// Inline content larger than this is rejected as malformed.
pub const MAX_INLINE_CONTENT: usize = 1024 * 1024;

/// A parsed artifact file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedArtifact {
    /// The artifact.
    pub artifact: Artifact,
    /// Keys not understood by this release, kept for forward compatibility (`V0018`).
    pub unknown_fields: Vec<String>,
}

fn malformed(error: impl Display) -> FileProblem {
    FileProblem::malformed(error.to_string())
}

fn invalid_identifier(error: impl Display) -> FileProblem {
    FileProblem::new(LoadIssueKind::InvalidIdentifier, error.to_string())
}

/// Parses the text of an artifact file.
///
/// # Errors
/// Returns a classified [`FileProblem`]: a missing `id` is `MissingIdentifier`, a bad id or slug
/// is `InvalidIdentifier`, an unsafe content path is `UnsafeContentPath`, anything else wrong
/// with the file is `MalformedFile`.
pub fn parse_artifact_file(text: &str) -> FileResult<ParsedArtifact> {
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
        Some(text) => text.parse::<ArtifactId>().map_err(invalid_identifier)?,
    };
    let slug = Slug::parse(required_str(table, "slug")?).map_err(invalid_identifier)?;
    let artifact_type = ArtifactType::new(required_str(table, "type")?).map_err(malformed)?;
    let title = Title::new(required_str(table, "title")?).map_err(malformed)?;
    let revision = optional_integer(table, "revision")?
        .ok_or_else(|| FileProblem::malformed("missing field `revision`"))?;
    let revision = u64::try_from(revision)
        .ok()
        .and_then(|number| Revision::new(number).ok())
        .ok_or_else(|| FileProblem::malformed("`revision` must be an integer of at least 1"))?;
    let description = optional_str(table, "description")?.map(str::to_owned);
    let created = required_timestamp(table, "created")?;
    let updated = required_timestamp(table, "updated")?;
    let tags = match table.get("tags") {
        None => Tags::default(),
        Some(item) => string_list(item, "tags")?
            .iter()
            .map(|tag| Tag::new(tag).map_err(malformed))
            .collect::<FileResult<Tags>>()?,
    };
    let metadata = match table.get("metadata") {
        None => Metadata::new(),
        Some(item) => parse_metadata(item)?,
    };
    let content = match table.get("content") {
        None => Content::None,
        Some(item) => parse_content(item)?,
    };

    let artifact = Artifact::from_parts(
        id,
        slug,
        artifact_type,
        title,
        revision,
        description,
        tags,
        metadata,
        content,
        created,
        updated,
    )
    .map_err(malformed)?;
    Ok(ParsedArtifact {
        artifact,
        unknown_fields: unknown_keys(table, &ARTIFACT_KEYS),
    })
}

fn parse_metadata(item: &Item) -> FileResult<Metadata> {
    let table = item
        .as_table_like()
        .ok_or_else(|| FileProblem::malformed("`metadata` must be a table"))?;
    let mut metadata = Metadata::new();
    for (key, entry) in table.iter() {
        let name = MetadataKey::new(key).map_err(malformed)?;
        let value = entry.as_value().and_then(metadata_value).ok_or_else(|| {
            FileProblem::malformed(format!(
                "metadata `{key}` must be a string, integer, boolean or array of strings"
            ))
        })?;
        metadata.insert(name, value);
    }
    Ok(metadata)
}

fn metadata_value(value: &Value) -> Option<MetadataValue> {
    match value {
        Value::String(text) => Some(MetadataValue::String(text.value().clone())),
        Value::Integer(number) => Some(MetadataValue::Integer(*number.value())),
        Value::Boolean(flag) => Some(MetadataValue::Bool(*flag.value())),
        Value::Array(array) => array
            .iter()
            .map(|item| item.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .map(MetadataValue::StringList),
        _ => None,
    }
}

fn parse_content(item: &Item) -> FileResult<Content> {
    let table: &dyn TableLike = item
        .as_table_like()
        .ok_or_else(|| FileProblem::malformed("`content` must be a table"))?;
    let string = |key: &str| -> FileResult<Option<&str>> {
        match table.get(key) {
            None => Ok(None),
            Some(entry) => entry
                .as_str()
                .map(Some)
                .ok_or_else(|| FileProblem::malformed(format!("`content.{key}` must be a string"))),
        }
    };
    let format = string("format")?
        .ok_or_else(|| FileProblem::malformed("missing field `content.format`"))?;
    let format = ContentFormat::new(format).map_err(malformed)?;
    if let Some(unknown) = table
        .iter()
        .map(|(key, _)| key)
        .find(|key| !["format", "text", "path"].contains(key))
    {
        return Err(FileProblem::malformed(format!(
            "unknown field `content.{unknown}`"
        )));
    }
    match (string("text")?, string("path")?) {
        (Some(text), None) => {
            if text.len() > MAX_INLINE_CONTENT {
                return Err(FileProblem::malformed(
                    "inline content is larger than 1 MiB",
                ));
            }
            Ok(Content::Inline {
                format,
                text: text.to_owned(),
            })
        }
        (None, Some(path)) => {
            let path = ContentPath::new(path).map_err(|error| {
                FileProblem::new(LoadIssueKind::UnsafeContentPath, error.to_string())
            })?;
            Ok(Content::Path { format, path })
        }
        (Some(_), Some(_)) => Err(FileProblem::malformed(
            "`content` must have exactly one of `text` and `path`, not both",
        )),
        (None, None) => Err(FileProblem::malformed(
            "`content` must have one of `text` or `path`",
        )),
    }
}

fn to_value(value: &MetadataValue) -> Value {
    match value {
        MetadataValue::String(text) => Value::from(text.as_str()),
        MetadataValue::Integer(number) => Value::from(*number),
        MetadataValue::Bool(flag) => Value::from(*flag),
        MetadataValue::StringList(items) => string_array_value(items.iter().map(String::as_str)),
        // `MetadataValue` is non-exhaustive; an unknown kind is rendered as its debug text rather
        // than dropped.
        other => Value::from(format!("{other:?}")),
    }
}

fn revision_value(revision: Revision) -> Value {
    Value::from(i64::try_from(revision.get()).unwrap_or(i64::MAX))
}

fn spaced_table() -> Table {
    let mut table = Table::new();
    table.decor_mut().set_prefix("\n");
    table
}

fn metadata_table(metadata: &Metadata) -> Table {
    let mut table = spaced_table();
    for (key, value) in metadata.iter() {
        table.insert(key.as_str(), Item::Value(to_value(value)));
    }
    table
}

fn content_table(content: &Content) -> Option<Table> {
    let mut table = spaced_table();
    match content {
        Content::Inline { format, text } => {
            table.insert("format", Item::Value(Value::from(format.as_str())));
            table.insert("text", Item::Value(Value::from(text.as_str())));
        }
        Content::Path { format, path } => {
            table.insert("format", Item::Value(Value::from(format.as_str())));
            table.insert("path", Item::Value(Value::from(path.as_str())));
        }
        // No content has no table (and `Content` is non-exhaustive: future variants too).
        _ => return None,
    }
    Some(table)
}

/// Renders an artifact in the canonical form.
#[must_use]
pub fn render_artifact_file(artifact: &Artifact) -> String {
    let mut document = DocumentMut::new();
    let table = document.as_table_mut();
    table.insert("id", Item::Value(Value::from(artifact.id().to_string())));
    table.insert("slug", Item::Value(Value::from(artifact.slug().as_str())));
    table.insert(
        "type",
        Item::Value(Value::from(artifact.artifact_type().as_str())),
    );
    table.insert("title", Item::Value(Value::from(artifact.title().as_str())));
    table.insert("revision", Item::Value(revision_value(artifact.revision())));
    if let Some(description) = artifact.description() {
        table.insert("description", Item::Value(Value::from(description)));
    }
    table.insert("created", Item::Value(timestamp_value(artifact.created())));
    table.insert("updated", Item::Value(timestamp_value(artifact.updated())));
    if !artifact.tags().is_empty() {
        table.insert(
            "tags",
            Item::Value(string_array_value(artifact.tags().iter().map(Tag::as_str))),
        );
    }
    if !artifact.metadata().is_empty() {
        table.insert("metadata", Item::Table(metadata_table(artifact.metadata())));
    }
    if let Some(content) = content_table(artifact.content()) {
        table.insert("content", Item::Table(content));
    }
    document.to_string()
}

/// Edits the existing file text so that it reflects `artifact`, changing only what a patch
/// changed: the fields in `changed`, plus `revision` and `updated`.
///
/// Comments, whitespace, line endings, key order and unknown keys and tables are preserved. A
/// key that does not exist yet is inserted at its canonical position.
///
/// # Errors
/// Fails if `original` is not valid TOML (it was valid when it was loaded, so this signals the
/// file changed underneath the caller).
pub fn edit_artifact_file(
    original: &str,
    artifact: &Artifact,
    changed: &[ArtifactField],
) -> FileResult<String> {
    let mut document: DocumentMut = original
        .parse()
        .map_err(|error| FileProblem::malformed(format!("invalid TOML: {error}")))?;
    let table = document.as_table_mut();

    set_value_in_order(
        table,
        "revision",
        revision_value(artifact.revision()),
        &VALUE_ORDER,
    );
    set_value_in_order(
        table,
        "updated",
        timestamp_value(artifact.updated()),
        &VALUE_ORDER,
    );

    for field in changed {
        match field {
            ArtifactField::Slug => {
                set_value_in_order(
                    table,
                    "slug",
                    Value::from(artifact.slug().as_str()),
                    &VALUE_ORDER,
                );
            }
            ArtifactField::Title => {
                set_value_in_order(
                    table,
                    "title",
                    Value::from(artifact.title().as_str()),
                    &VALUE_ORDER,
                );
            }
            ArtifactField::Description => match artifact.description() {
                Some(text) => {
                    set_value_in_order(table, "description", Value::from(text), &VALUE_ORDER);
                }
                None => remove_key(table, "description"),
            },
            ArtifactField::Tags => {
                if artifact.tags().is_empty() {
                    remove_key(table, "tags");
                } else {
                    let tags = string_array_value(artifact.tags().iter().map(Tag::as_str));
                    set_value_in_order(table, "tags", tags, &VALUE_ORDER);
                }
            }
            ArtifactField::Metadata => sync_metadata(table, artifact.metadata()),
            ArtifactField::Content => {
                remove_key(table, "content");
                if let Some(content) = content_table(artifact.content()) {
                    table.insert("content", Item::Table(content));
                }
            }
            // `ArtifactField` is non-exhaustive; an unknown field has nothing to write here.
            _ => {}
        }
    }
    Ok(match_line_endings(original, document.to_string()))
}

// Updates the `[metadata]` table key by key so comments on untouched keys survive.
fn sync_metadata(root: &mut Table, metadata: &Metadata) {
    if metadata.is_empty() {
        remove_key(root, "metadata");
        return;
    }
    if !root.get("metadata").is_some_and(Item::is_table) {
        remove_key(root, "metadata");
        root.insert("metadata", Item::Table(spaced_table()));
    }
    let Some(table) = root.get_mut("metadata").and_then(Item::as_table_mut) else {
        return;
    };
    let stale: Vec<String> = table
        .iter()
        .map(|(key, _)| key.to_owned())
        .filter(|key| !metadata.iter().any(|(wanted, _)| wanted.as_str() == key))
        .collect();
    for key in stale {
        remove_key(table, &key);
    }
    for (key, value) in metadata.iter() {
        set_value(table, key.as_str(), to_value(value));
    }
}

#[cfg(test)]
mod tests {
    use functor_model::{NewArtifact, Timestamp};

    use super::*;

    const ID: &str = "art_01J9Z3K8M2Q7X4V6B5N1R0T8WD";

    fn ts(text: &str) -> Timestamp {
        Timestamp::parse(text).unwrap()
    }

    fn minimal() -> Artifact {
        let new = NewArtifact::new(
            Slug::parse("rfc-0000").unwrap(),
            ArtifactType::new("rfc").unwrap(),
            Title::new("Functor Vision").unwrap(),
        );
        Artifact::create(ID.parse().unwrap(), new, ts("2026-10-12T18:00:00Z"))
    }

    fn complete() -> Artifact {
        let mut metadata = Metadata::new();
        metadata.insert(MetadataKey::new("author").unwrap(), "Paulo".into());
        metadata.insert(MetadataKey::new("status").unwrap(), "draft".into());
        let new = NewArtifact::new(
            Slug::parse("rfc-0000").unwrap(),
            ArtifactType::new("rfc").unwrap(),
            Title::new("Functor Vision").unwrap(),
        )
        .with_description("Platform vision and goals")
        .with_tags(Tags::new(
            ["rfc", "core", "architecture"].map(|t| Tag::new(t).unwrap()),
        ))
        .with_metadata(metadata)
        .with_content(Content::Path {
            format: ContentFormat::new("markdown").unwrap(),
            path: ContentPath::new("content/rfc-0000.md").unwrap(),
        });
        let created = Artifact::create(ID.parse().unwrap(), new, ts("2026-10-12T18:00:00Z"));
        // Two updates, so the documented example shows revision 3.
        let mut patched = created;
        for when in ["2026-10-13T00:00:00Z", "2026-10-14T09:30:00Z"] {
            patched = patched
                .apply_patch(
                    functor_model::ArtifactPatch::new()
                        .with_description(Some(format!("Platform vision and goals {when}"))),
                    ts(when),
                )
                .unwrap()
                .artifact;
        }
        patched
    }

    #[test]
    fn minimal_artifact_renders_canonically() {
        assert_eq!(
            render_artifact_file(&minimal()),
            format!(
                "id = \"{ID}\"\nslug = \"rfc-0000\"\ntype = \"rfc\"\ntitle = \"Functor Vision\"\nrevision = 1\ncreated = 2026-10-12T18:00:00Z\nupdated = 2026-10-12T18:00:00Z\n"
            )
        );
    }

    #[test]
    fn complete_artifact_renders_the_documented_example() {
        let rendered = render_artifact_file(&complete());
        assert_eq!(
            rendered,
            format!(
                "id = \"{ID}\"\nslug = \"rfc-0000\"\ntype = \"rfc\"\ntitle = \"Functor Vision\"\nrevision = 3\ndescription = \"Platform vision and goals 2026-10-14T09:30:00Z\"\ncreated = 2026-10-12T18:00:00Z\nupdated = 2026-10-14T09:30:00Z\ntags = [\"architecture\", \"core\", \"rfc\"]\n\n[metadata]\nauthor = \"Paulo\"\nstatus = \"draft\"\n\n[content]\nformat = \"markdown\"\npath = \"content/rfc-0000.md\"\n"
            )
        );
    }

    #[test]
    fn rendered_artifacts_parse_back_unchanged() {
        for artifact in [minimal(), complete()] {
            let text = render_artifact_file(&artifact);
            let parsed = parse_artifact_file(&text).unwrap();
            assert_eq!(parsed.artifact, artifact);
            assert_eq!(parsed.unknown_fields, Vec::<String>::new());
            assert_eq!(
                render_artifact_file(&parsed.artifact),
                text,
                "rendering is stable"
            );
        }
    }

    #[test]
    fn inline_content_with_newlines_round_trips() {
        let text = "# Notes\n\nline \"two\" with a \\ backslash\n\ttabbed\n";
        let new = NewArtifact::new(
            Slug::parse("n").unwrap(),
            ArtifactType::new("note").unwrap(),
            Title::new("n").unwrap(),
        )
        .with_content(Content::Inline {
            format: ContentFormat::new("markdown").unwrap(),
            text: text.to_owned(),
        });
        let artifact = Artifact::create(ID.parse().unwrap(), new, ts("2026-10-12T18:00:00Z"));
        let rendered = render_artifact_file(&artifact);
        assert!(rendered.contains("[content]"));
        let parsed = parse_artifact_file(&rendered).unwrap();
        assert_eq!(parsed.artifact.content(), artifact.content());
    }

    #[test]
    fn long_tag_lists_split_across_lines_and_round_trip() {
        let tags: Vec<Tag> = (0..15)
            .map(|i| Tag::new(&format!("tag-{i:02}")).unwrap())
            .collect();
        let new = NewArtifact::new(
            Slug::parse("t").unwrap(),
            ArtifactType::document(),
            Title::new("t").unwrap(),
        )
        .with_tags(Tags::new(tags));
        let artifact = Artifact::create(ID.parse().unwrap(), new, ts("2026-10-12T18:00:00Z"));
        let rendered = render_artifact_file(&artifact);
        assert!(
            rendered.contains("tags = [\n    \"tag-00\",\n"),
            "{rendered}"
        );
        assert_eq!(parse_artifact_file(&rendered).unwrap().artifact, artifact);
    }

    #[test]
    fn metadata_of_every_type_round_trips() {
        let mut metadata = Metadata::new();
        metadata.insert(MetadataKey::new("s").unwrap(), "text".into());
        metadata.insert(MetadataKey::new("i").unwrap(), 42_i64.into());
        metadata.insert(MetadataKey::new("b").unwrap(), true.into());
        metadata.insert(
            MetadataKey::new("l").unwrap(),
            vec!["x".to_owned(), "y".to_owned()].into(),
        );
        let new = NewArtifact::new(
            Slug::parse("m").unwrap(),
            ArtifactType::document(),
            Title::new("m").unwrap(),
        )
        .with_metadata(metadata);
        let artifact = Artifact::create(ID.parse().unwrap(), new, ts("2026-10-12T18:00:00Z"));
        let parsed = parse_artifact_file(&render_artifact_file(&artifact)).unwrap();
        assert_eq!(parsed.artifact, artifact);
    }

    fn problem(text: &str) -> FileProblem {
        parse_artifact_file(text).unwrap_err()
    }

    fn with(replace: &str, with: &str) -> String {
        render_artifact_file(&minimal()).replace(replace, with)
    }

    #[test]
    fn parse_classifies_problems() {
        assert_eq!(problem("not = = toml").kind, LoadIssueKind::MalformedFile);
        let without_id = render_artifact_file(&minimal()).replace(&format!("id = \"{ID}\"\n"), "");
        assert_eq!(problem(&without_id).kind, LoadIssueKind::MissingIdentifier);
        assert_eq!(
            problem(&with(ID, "art_nope")).kind,
            LoadIssueKind::InvalidIdentifier
        );
        assert_eq!(
            problem(&with("rfc-0000", "Bad Slug")).kind,
            LoadIssueKind::InvalidIdentifier
        );
        assert_eq!(
            problem(&with("title = \"Functor Vision\"\n", "")).kind,
            LoadIssueKind::MalformedFile
        );
        assert_eq!(
            problem(&with("title = \"Functor Vision\"", "title = \"  \"")).kind,
            LoadIssueKind::MalformedFile
        );
        assert_eq!(
            problem(&with("revision = 1", "revision = 0")).kind,
            LoadIssueKind::MalformedFile
        );
        assert_eq!(
            problem(&with("revision = 1", "revision = \"1\"")).kind,
            LoadIssueKind::MalformedFile
        );
        assert_eq!(
            problem(&with("type = \"rfc\"", "type = \"Not Valid\"")).kind,
            LoadIssueKind::MalformedFile
        );
        assert!(
            problem(&with(
                "created = 2026-10-12T18:00:00Z",
                "created = \"yesterday\""
            ))
            .detail
            .contains("created")
        );
        assert!(
            problem(&with(
                "updated = 2026-10-12T18:00:00Z",
                "updated = 2020-01-01T00:00:00Z"
            ))
            .detail
            .contains("updated")
        );
    }

    #[test]
    fn kind_is_not_accepted_in_place_of_type() {
        let text = with("type = \"rfc\"", "kind = \"rfc\"");
        assert!(problem(&text).detail.contains("type"));
    }

    #[test]
    fn content_problems_are_classified() {
        let base = render_artifact_file(&minimal());
        let both = format!("{base}\n[content]\nformat = \"text\"\ntext = \"a\"\npath = \"b\"\n");
        assert_eq!(problem(&both).kind, LoadIssueKind::MalformedFile);
        let neither = format!("{base}\n[content]\nformat = \"text\"\n");
        assert_eq!(problem(&neither).kind, LoadIssueKind::MalformedFile);
        let no_format = format!("{base}\n[content]\ntext = \"a\"\n");
        assert!(problem(&no_format).detail.contains("format"));
        for unsafe_path in ["../outside.md", "/etc/passwd", "a/../../b", "C:/x"] {
            let text = format!("{base}\n[content]\nformat = \"text\"\npath = \"{unsafe_path}\"\n");
            assert_eq!(
                problem(&text).kind,
                LoadIssueKind::UnsafeContentPath,
                "{unsafe_path}"
            );
        }
        let huge = format!(
            "{base}\n[content]\nformat = \"text\"\ntext = \"{}\"\n",
            "x".repeat(MAX_INLINE_CONTENT + 1)
        );
        assert!(problem(&huge).detail.contains("1 MiB"));
    }

    #[test]
    fn metadata_and_tag_problems_are_malformed() {
        let base = render_artifact_file(&minimal());
        for extra in [
            "\n[metadata]\nBad = \"x\"\n",
            "\n[metadata]\nnested = { a = 1 }\n",
            "\n[metadata]\nlist = [1, 2]\n",
            "\n[metadata]\nwhen = 2026-10-12T18:00:00Z\n",
        ] {
            assert_eq!(
                problem(&format!("{base}{extra}")).kind,
                LoadIssueKind::MalformedFile,
                "{extra}"
            );
        }
        let bad_tags = with("revision = 1", "revision = 1\ntags = [\"OK\"]");
        assert_eq!(problem(&bad_tags).kind, LoadIssueKind::MalformedFile);
    }

    #[test]
    fn unknown_fields_are_reported_but_tolerated() {
        let text = format!(
            "{}future = 1\n\n[extra]\nx = 1\n",
            render_artifact_file(&minimal())
        );
        let parsed = parse_artifact_file(&text).unwrap();
        assert_eq!(parsed.unknown_fields, ["future", "extra"]);
    }

    #[test]
    fn crlf_files_parse() {
        let text = render_artifact_file(&minimal()).replace('\n', "\r\n");
        assert_eq!(parse_artifact_file(&text).unwrap().artifact, minimal());
    }

    // --- in-place edits ---

    fn patch(
        artifact: &Artifact,
        patch: functor_model::ArtifactPatch,
        when: &str,
    ) -> functor_model::Patched {
        artifact.apply_patch(patch, ts(when)).unwrap()
    }

    #[test]
    fn editing_a_canonical_file_matches_a_fresh_render() {
        let original = minimal();
        let text = render_artifact_file(&original);
        let patched = patch(
            &original,
            functor_model::ArtifactPatch::new()
                .with_title(Title::new("Vision").unwrap())
                .with_description(Some("d".into())),
            "2026-10-13T00:00:00Z",
        );
        let edited = edit_artifact_file(&text, &patched.artifact, &patched.changed).unwrap();
        assert_eq!(edited, render_artifact_file(&patched.artifact));
    }

    #[test]
    fn editing_preserves_comments_order_whitespace_and_unknown_keys() {
        let text = format!(
            "# my notes about this artifact\nid = \"{ID}\"\nslug   = \"rfc-0000\"   # the alias\ntype = \"rfc\"\nowner = \"someone\"\ntitle = \"Functor Vision\"\nrevision = 1\ncreated = 2026-10-12T18:00:00Z\nupdated = 2026-10-12T18:00:00Z\n\n[extra]\nkeep = \"me\"\n"
        );
        let original = parse_artifact_file(&text).unwrap().artifact;
        let patched = patch(
            &original,
            functor_model::ArtifactPatch::new().with_slug(Slug::parse("vision").unwrap()),
            "2026-10-13T00:00:00Z",
        );
        let edited = edit_artifact_file(&text, &patched.artifact, &patched.changed).unwrap();
        assert_eq!(
            edited,
            format!(
                "# my notes about this artifact\nid = \"{ID}\"\nslug   = \"vision\"   # the alias\ntype = \"rfc\"\nowner = \"someone\"\ntitle = \"Functor Vision\"\nrevision = 2\ncreated = 2026-10-12T18:00:00Z\nupdated = 2026-10-13T00:00:00Z\n\n[extra]\nkeep = \"me\"\n"
            )
        );
        let reparsed = parse_artifact_file(&edited).unwrap();
        assert_eq!(reparsed.artifact, patched.artifact);
        assert_eq!(reparsed.unknown_fields, ["owner", "extra"]);
    }

    #[test]
    fn new_optional_keys_are_inserted_at_their_canonical_position() {
        let text = format!(
            "id = \"{ID}\"\nslug = \"rfc-0000\"\ntype = \"rfc\"\ntitle = \"Functor Vision\"\nrevision = 1\ncreated = 2026-10-12T18:00:00Z\nupdated = 2026-10-12T18:00:00Z\nowner = \"someone\"\n"
        );
        let original = parse_artifact_file(&text).unwrap().artifact;
        let patched = patch(
            &original,
            functor_model::ArtifactPatch::new()
                .with_description(Some("d".into()))
                .with_tags(Tags::new([Tag::new("a").unwrap()])),
            "2026-10-13T00:00:00Z",
        );
        let edited = edit_artifact_file(&text, &patched.artifact, &patched.changed).unwrap();
        assert_eq!(
            edited,
            format!(
                "id = \"{ID}\"\nslug = \"rfc-0000\"\ntype = \"rfc\"\ntitle = \"Functor Vision\"\nrevision = 2\ndescription = \"d\"\ncreated = 2026-10-12T18:00:00Z\nupdated = 2026-10-13T00:00:00Z\ntags = [\"a\"]\nowner = \"someone\"\n"
            )
        );
    }

    #[test]
    fn clearing_a_description_removes_only_its_own_comment() {
        let text = format!(
            "id = \"{ID}\"\nslug = \"rfc-0000\"\ntype = \"rfc\"\ntitle = \"T\"\nrevision = 2\n# about the description\ndescription = \"d\"\n# about created\ncreated = 2026-10-12T18:00:00Z\nupdated = 2026-10-12T18:00:00Z\n"
        );
        let original = parse_artifact_file(&text).unwrap().artifact;
        let patched = patch(
            &original,
            functor_model::ArtifactPatch::new().with_description(None),
            "2026-10-13T00:00:00Z",
        );
        let edited = edit_artifact_file(&text, &patched.artifact, &patched.changed).unwrap();
        assert!(!edited.contains("description"));
        assert!(!edited.contains("about the description"));
        assert!(edited.contains("# about created\ncreated"));
    }

    #[test]
    fn metadata_is_synced_key_by_key() {
        let text = format!(
            "id = \"{ID}\"\nslug = \"s\"\ntype = \"rfc\"\ntitle = \"T\"\nrevision = 1\ncreated = 2026-10-12T18:00:00Z\nupdated = 2026-10-12T18:00:00Z\n\n[metadata]\n# who\nauthor = \"Paulo\"   # me\nstatus = \"draft\"\nold = 1\n"
        );
        let original = parse_artifact_file(&text).unwrap().artifact;
        let mut metadata = Metadata::new();
        metadata.insert(MetadataKey::new("author").unwrap(), "Paulo".into());
        metadata.insert(MetadataKey::new("status").unwrap(), "final".into());
        metadata.insert(MetadataKey::new("fresh").unwrap(), true.into());
        let patched = patch(
            &original,
            functor_model::ArtifactPatch::new().with_metadata(metadata),
            "2026-10-13T00:00:00Z",
        );
        let edited = edit_artifact_file(&text, &patched.artifact, &patched.changed).unwrap();
        assert!(
            edited.contains("# who\nauthor = \"Paulo\"   # me\n"),
            "{edited}"
        );
        assert!(edited.contains("status = \"final\""));
        assert!(edited.contains("fresh = true"));
        assert!(!edited.contains("old"));
        assert_eq!(
            parse_artifact_file(&edited).unwrap().artifact,
            patched.artifact
        );
    }

    #[test]
    fn metadata_can_be_added_and_removed_entirely() {
        let original = minimal();
        let text = render_artifact_file(&original);
        let mut metadata = Metadata::new();
        metadata.insert(MetadataKey::new("a").unwrap(), 1_i64.into());
        let added = patch(
            &original,
            functor_model::ArtifactPatch::new().with_metadata(metadata),
            "2026-10-13T00:00:00Z",
        );
        let with_meta = edit_artifact_file(&text, &added.artifact, &added.changed).unwrap();
        assert!(with_meta.contains("\n[metadata]\na = 1\n"));

        let removed = patch(
            &added.artifact,
            functor_model::ArtifactPatch::new().with_metadata(Metadata::new()),
            "2026-10-14T00:00:00Z",
        );
        let without = edit_artifact_file(&with_meta, &removed.artifact, &removed.changed).unwrap();
        assert!(!without.contains("metadata"));
        assert_eq!(
            parse_artifact_file(&without).unwrap().artifact,
            removed.artifact
        );
    }

    #[test]
    fn content_can_be_replaced_and_cleared() {
        let original = complete();
        let text = render_artifact_file(&original);
        let replaced = patch(
            &original,
            functor_model::ArtifactPatch::new().with_content(Content::Inline {
                format: ContentFormat::new("text").unwrap(),
                text: "hello".into(),
            }),
            "2026-10-15T00:00:00Z",
        );
        let edited = edit_artifact_file(&text, &replaced.artifact, &replaced.changed).unwrap();
        assert!(
            edited.contains("text = \"hello\"") && !edited.contains("path ="),
            "{edited}"
        );
        assert_eq!(
            parse_artifact_file(&edited).unwrap().artifact,
            replaced.artifact
        );

        let cleared = patch(
            &replaced.artifact,
            functor_model::ArtifactPatch::new().with_content(Content::None),
            "2026-10-16T00:00:00Z",
        );
        let none = edit_artifact_file(&edited, &cleared.artifact, &cleared.changed).unwrap();
        assert!(!none.contains("[content]"));
        assert_eq!(
            parse_artifact_file(&none).unwrap().artifact,
            cleared.artifact
        );
    }

    #[test]
    fn crlf_files_keep_crlf_after_an_edit() {
        let original = minimal();
        let text = render_artifact_file(&original).replace('\n', "\r\n");
        let patched = patch(
            &original,
            functor_model::ArtifactPatch::new().with_description(Some("d".into())),
            "2026-10-13T00:00:00Z",
        );
        let edited = edit_artifact_file(&text, &patched.artifact, &patched.changed).unwrap();
        assert!(
            !edited.replace("\r\n", "").contains('\n'),
            "no bare LF remains"
        );
        assert!(edited.contains("description = \"d\"\r\n"));
        assert_eq!(
            parse_artifact_file(&edited).unwrap().artifact,
            patched.artifact
        );
    }

    #[test]
    fn editing_invalid_text_is_reported() {
        let artifact = minimal();
        assert!(edit_artifact_file("= = =", &artifact, &[]).is_err());
    }
}
