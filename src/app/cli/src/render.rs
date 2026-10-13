//! The shapes of artifact output: JSON documents (the stable contract) and text rendering.
//!
//! See `docs/cli-spec.md`, "JSON Schemas". Field order below is the documented key order.

use std::collections::BTreeMap;

use functor_core::FunctorError;
use functor_model::{
    Artifact, ArtifactField, ArtifactId, Content, Direction, MetadataValue, Relationship,
    RelationshipType, TraversalResult,
};
use serde::Serialize;
use serde_json::Value;

use crate::output::SCHEMA_VERSION;

/// The `content` object of an artifact: the format plus either inline `text` or a `path`.
/// With `show --content` a `path` artifact also carries the resolved `text`.
#[derive(Debug, Serialize)]
pub struct ContentDto {
    format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

/// An artifact as it appears in JSON output.
#[derive(Debug, Serialize)]
pub struct ArtifactDto {
    id: String,
    r#ref: String,
    slug: String,
    r#type: String,
    title: String,
    revision: u64,
    description: Option<String>,
    tags: Vec<String>,
    metadata: BTreeMap<String, Value>,
    content: Option<ContentDto>,
    created: String,
    updated: String,
}

fn metadata_json(value: &MetadataValue) -> Value {
    match value {
        MetadataValue::String(text) => Value::from(text.as_str()),
        MetadataValue::Integer(number) => Value::from(*number),
        MetadataValue::Bool(flag) => Value::from(*flag),
        MetadataValue::StringList(items) => Value::from(items.clone()),
        other => Value::from(format!("{other:?}")),
    }
}

impl ArtifactDto {
    /// Builds the DTO. `resolved_text` is the content of a `path` artifact read by
    /// `show --content`.
    #[must_use]
    pub fn new(artifact: &Artifact, resolved_text: Option<String>) -> Self {
        let content = match artifact.content() {
            Content::Inline { format, text } => Some(ContentDto {
                format: format.to_string(),
                text: Some(text.clone()),
                path: None,
            }),
            Content::Path { format, path } => Some(ContentDto {
                format: format.to_string(),
                text: resolved_text,
                path: Some(path.to_string()),
            }),
            _ => None,
        };
        Self {
            id: artifact.id().to_string(),
            r#ref: artifact.id().canonical(),
            slug: artifact.slug().to_string(),
            r#type: artifact.artifact_type().to_string(),
            title: artifact.title().to_string(),
            revision: artifact.revision().get(),
            description: artifact.description().map(str::to_owned),
            tags: artifact.tags().iter().map(ToString::to_string).collect(),
            metadata: artifact
                .metadata()
                .iter()
                .map(|(key, value)| (key.to_string(), metadata_json(value)))
                .collect(),
            content,
            created: artifact.created().to_string(),
            updated: artifact.updated().to_string(),
        }
    }
}

/// The document printed by `artifact create`, `show` and `update`.
#[derive(Debug, Serialize)]
pub struct ArtifactDocument {
    schema_version: u32,
    artifact: ArtifactDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    changed: Option<Vec<String>>,
}

impl ArtifactDocument {
    /// A document for `create` and `show`.
    #[must_use]
    pub fn new(artifact: ArtifactDto) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            artifact,
            changed: None,
        }
    }

    /// A document for `update`, which also lists the changed fields.
    #[must_use]
    pub fn updated(artifact: ArtifactDto, changed: &[ArtifactField]) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            artifact,
            changed: Some(changed.iter().map(ToString::to_string).collect()),
        }
    }
}

/// The document printed by `artifact list`.
#[derive(Debug, Serialize)]
pub struct ArtifactListDocument {
    schema_version: u32,
    artifacts: Vec<ArtifactDto>,
}

impl ArtifactListDocument {
    /// A list document.
    #[must_use]
    pub fn new(artifacts: Vec<ArtifactDto>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            artifacts,
        }
    }
}

// ---- text ----

/// Escapes control characters so repository content cannot inject terminal escape sequences.
/// Tabs are kept; use [`sanitize_block`] for text where line breaks are meaningful.
#[must_use]
pub fn sanitize(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() && c != '\t' {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}

/// Like [`sanitize`], but keeps line breaks (CRLF is normalized to LF first) so multi-line
/// content stays readable.
#[must_use]
pub fn sanitize_block(text: &str) -> String {
    text.replace("\r\n", "\n")
        .split('\n')
        .map(sanitize)
        .collect::<Vec<_>>()
        .join("\n")
}

fn metadata_text(value: &MetadataValue) -> String {
    match value {
        MetadataValue::String(text) => sanitize(text),
        MetadataValue::Integer(number) => number.to_string(),
        MetadataValue::Bool(flag) => flag.to_string(),
        MetadataValue::StringList(items) => items
            .iter()
            .map(|item| sanitize(item))
            .collect::<Vec<_>>()
            .join(", "),
        other => format!("{other:?}"),
    }
}

/// `created artifact <slug> (<id>)`
#[must_use]
pub fn created_text(artifact: &Artifact) -> String {
    format!("created artifact {} ({})", artifact.slug(), artifact.id())
}

/// `updated artifact <slug> (revision N)`, or `no changes`.
#[must_use]
pub fn updated_text(artifact: &Artifact, changed: &[ArtifactField]) -> String {
    if changed.is_empty() {
        "no changes".to_owned()
    } else {
        format!(
            "updated artifact {} (revision {})",
            artifact.slug(),
            artifact.revision()
        )
    }
}

/// The multi-line description printed by `artifact show`.
#[must_use]
pub fn show_text(artifact: &Artifact, resolved_text: Option<&str>) -> String {
    let mut lines = vec![
        format!("slug:        {}", artifact.slug()),
        format!("id:          {}", artifact.id()),
        format!("type:        {}", artifact.artifact_type()),
        format!("title:       {}", sanitize(artifact.title().as_str())),
        format!("revision:    {}", artifact.revision()),
        format!("created:     {}", artifact.created()),
        format!("updated:     {}", artifact.updated()),
    ];
    if let Some(description) = artifact.description() {
        lines.push(format!("description: {}", sanitize(description)));
    }
    if !artifact.tags().is_empty() {
        let tags: Vec<String> = artifact.tags().iter().map(ToString::to_string).collect();
        lines.push(format!("tags:        {}", tags.join(", ")));
    }
    if !artifact.metadata().is_empty() {
        lines.push("metadata:".to_owned());
        for (key, value) in artifact.metadata().iter() {
            lines.push(format!("  {key}: {}", metadata_text(value)));
        }
    }
    match artifact.content() {
        Content::Inline { format, text } => {
            lines.push(format!(
                "content:     {format}, inline ({} bytes)",
                text.len()
            ));
        }
        Content::Path { format, path } => lines.push(format!("content:     {format}, path {path}")),
        _ => {}
    }
    let mut text = lines.join("\n");
    if let Some(body) = resolved_text {
        text.push_str("\n\n");
        text.push_str(&sanitize_block(body.strip_suffix('\n').unwrap_or(body)));
    }
    text
}

// ---- relationships ----

/// Artifact slugs by id, used to label relationship endpoints.
#[derive(Debug, Default)]
pub struct SlugIndex(BTreeMap<ArtifactId, String>);

impl SlugIndex {
    /// Builds the index from a list of artifacts.
    #[must_use]
    pub fn new(artifacts: &[Artifact]) -> Self {
        Self(
            artifacts
                .iter()
                .map(|a| (*a.id(), a.slug().to_string()))
                .collect(),
        )
    }

    /// The slug of an artifact. The store refuses repositories with dangling relationship
    /// endpoints, so a missing slug means the two reads disagree and is an internal error.
    ///
    /// # Errors
    /// `Internal` when the artifact is unknown.
    pub fn slug(&self, id: &ArtifactId) -> Result<&str, FunctorError> {
        self.0.get(id).map(String::as_str).ok_or_else(|| {
            FunctorError::Internal(format!("relationship endpoint {id} has no artifact"))
        })
    }
}

/// One end of a relationship.
#[derive(Debug, Serialize)]
pub struct EndpointDto {
    id: String,
    slug: String,
}

impl EndpointDto {
    fn new(id: &ArtifactId, slugs: &SlugIndex) -> Result<Self, FunctorError> {
        Ok(Self {
            id: id.to_string(),
            slug: slugs.slug(id)?.to_owned(),
        })
    }

    /// The `{id, slug}` reference of an artifact.
    #[must_use]
    pub fn of(artifact: &Artifact) -> Self {
        Self {
            id: artifact.id().to_string(),
            slug: artifact.slug().to_string(),
        }
    }
}

/// A relationship as it appears in JSON output.
#[derive(Debug, Serialize)]
pub struct RelationshipDto {
    id: String,
    r#ref: String,
    r#type: String,
    source: EndpointDto,
    target: EndpointDto,
    created: String,
    metadata: BTreeMap<String, Value>,
}

impl RelationshipDto {
    /// Builds the DTO, labeling endpoints with the slugs in `slugs`.
    ///
    /// # Errors
    /// `Internal` when an endpoint has no slug in the index.
    pub fn new(relationship: &Relationship, slugs: &SlugIndex) -> Result<Self, FunctorError> {
        Ok(Self {
            id: relationship.id().to_string(),
            r#ref: relationship.id().canonical(),
            r#type: relationship.relationship_type().to_string(),
            source: EndpointDto::new(relationship.source(), slugs)?,
            target: EndpointDto::new(relationship.target(), slugs)?,
            created: relationship.created().to_string(),
            metadata: relationship
                .metadata()
                .iter()
                .map(|(key, value)| (key.to_string(), metadata_json(value)))
                .collect(),
        })
    }
}

/// The document printed by `relation add` and `relation show`.
#[derive(Debug, Serialize)]
pub struct RelationshipDocument {
    schema_version: u32,
    relationship: RelationshipDto,
}

impl RelationshipDocument {
    /// A relationship document.
    #[must_use]
    pub fn new(relationship: RelationshipDto) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            relationship,
        }
    }
}

/// The document printed by `relation list`.
#[derive(Debug, Serialize)]
pub struct RelationshipListDocument {
    schema_version: u32,
    relationships: Vec<RelationshipDto>,
}

impl RelationshipListDocument {
    /// A list document.
    #[must_use]
    pub fn new(relationships: Vec<RelationshipDto>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            relationships,
        }
    }
}

/// An artifact reached by a traversal, with the edge that reached it.
#[derive(Debug, Serialize)]
pub struct NodeDto {
    depth: u32,
    artifact: ArtifactDto,
    via: RelationshipDto,
}

/// The document printed by `query dependencies` and `query dependents`.
#[derive(Debug, Serialize)]
pub struct TraversalDocument {
    schema_version: u32,
    start: EndpointDto,
    direction: &'static str,
    relationship_types: Vec<String>,
    max_depth: Option<u32>,
    nodes: Vec<NodeDto>,
    edges: Vec<RelationshipDto>,
}

impl TraversalDocument {
    /// Builds the document from a traversal result.
    ///
    /// # Errors
    /// `Internal` when an edge endpoint is neither the start nor a reached node.
    pub fn new(
        result: &TraversalResult,
        direction: Direction,
        types: &[RelationshipType],
        max_depth: Option<u32>,
    ) -> Result<Self, FunctorError> {
        let slugs = traversal_slugs(result);
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            start: EndpointDto::of(&result.start),
            direction: match direction {
                Direction::Inbound => "inbound",
                _ => "outbound",
            },
            relationship_types: types.iter().map(ToString::to_string).collect(),
            max_depth,
            nodes: result
                .nodes
                .iter()
                .map(|node| {
                    Ok(NodeDto {
                        depth: node.depth,
                        artifact: ArtifactDto::new(&node.artifact, None),
                        via: RelationshipDto::new(&node.via, &slugs)?,
                    })
                })
                .collect::<Result<_, FunctorError>>()?,
            edges: result
                .edges
                .iter()
                .map(|edge| RelationshipDto::new(edge, &slugs))
                .collect::<Result<_, FunctorError>>()?,
        })
    }
}

/// The slugs of every artifact a traversal touched: the start and the reached nodes.
fn traversal_slugs(result: &TraversalResult) -> SlugIndex {
    let mut artifacts = vec![result.start.clone()];
    artifacts.extend(result.nodes.iter().map(|node| node.artifact.clone()));
    SlugIndex::new(&artifacts)
}

/// The document printed by `query relationships`.
#[derive(Debug, Serialize)]
pub struct RelationshipsDocument {
    schema_version: u32,
    artifact: EndpointDto,
    relationships: Vec<RelationshipDto>,
}

impl RelationshipsDocument {
    /// Builds the document.
    ///
    /// # Errors
    /// `Internal` when an endpoint has no slug in the index.
    pub fn new(
        artifact: &Artifact,
        relationships: &[Relationship],
        slugs: &SlugIndex,
    ) -> Result<Self, FunctorError> {
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            artifact: EndpointDto::of(artifact),
            relationships: relationships
                .iter()
                .map(|r| RelationshipDto::new(r, slugs))
                .collect::<Result<_, FunctorError>>()?,
        })
    }
}

/// The text printed by `query dependencies` and `query dependents`: a heading and a table, or
/// `no <things> found`.
#[must_use]
pub fn traversal_text(
    things: &str,
    result: &TraversalResult,
    types: &[RelationshipType],
    max_depth: Option<u32>,
) -> String {
    if result.nodes.is_empty() {
        return format!("no {things} found");
    }
    let types: Vec<String> = types.iter().map(ToString::to_string).collect();
    let depth = max_depth.map_or_else(|| "all".to_owned(), |depth| depth.to_string());
    let rows: Vec<Vec<String>> = result
        .nodes
        .iter()
        .map(|node| {
            vec![
                node.depth.to_string(),
                node.artifact.slug().to_string(),
                node.artifact.artifact_type().to_string(),
                sanitize(node.artifact.title().as_str()),
            ]
        })
        .collect();
    format!(
        "{things} of {} ({}, depth {depth})\n{}",
        result.start.slug(),
        types.join(", "),
        table(&["DEPTH", "SLUG", "TYPE", "TITLE"], &rows)
    )
}

/// `added <rel id>: <source> <type> <target>`
///
/// # Errors
/// `Internal` when an endpoint has no slug in the index.
pub fn added_text(relationship: &Relationship, slugs: &SlugIndex) -> Result<String, FunctorError> {
    Ok(format!(
        "added {}: {} {} {}",
        relationship.id(),
        slugs.slug(relationship.source())?,
        relationship.relationship_type(),
        slugs.slug(relationship.target())?
    ))
}

/// The multi-line description printed by `relation show`.
///
/// # Errors
/// `Internal` when an endpoint has no slug in the index.
pub fn relationship_show_text(
    relationship: &Relationship,
    slugs: &SlugIndex,
) -> Result<String, FunctorError> {
    let mut lines = vec![
        format!("id:       {}", relationship.id()),
        format!("type:     {}", relationship.relationship_type()),
        format!(
            "source:   {} ({})",
            slugs.slug(relationship.source())?,
            relationship.source()
        ),
        format!(
            "target:   {} ({})",
            slugs.slug(relationship.target())?,
            relationship.target()
        ),
        format!("created:  {}", relationship.created()),
    ];
    if !relationship.metadata().is_empty() {
        lines.push("metadata:".to_owned());
        for (key, value) in relationship.metadata().iter() {
            lines.push(format!("  {key}: {}", metadata_text(value)));
        }
    }
    Ok(lines.join("\n"))
}

/// The table printed by `relation list`, or `no relationships found`.
///
/// # Errors
/// `Internal` when an endpoint has no slug in the index.
pub fn relationship_list_text(
    relationships: &[Relationship],
    slugs: &SlugIndex,
) -> Result<String, FunctorError> {
    if relationships.is_empty() {
        return Ok("no relationships found".to_owned());
    }
    let rows = relationships
        .iter()
        .map(|r| {
            Ok(vec![
                r.id().to_string(),
                slugs.slug(r.source())?.to_owned(),
                r.relationship_type().to_string(),
                slugs.slug(r.target())?.to_owned(),
            ])
        })
        .collect::<Result<Vec<_>, FunctorError>>()?;
    Ok(table(&["ID", "SOURCE", "TYPE", "TARGET"], &rows))
}

/// A table with a header row, two spaces between columns, padded to the widest cell.
#[must_use]
pub fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = header.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }
    let render = |cells: Vec<&str>| {
        let last = cells.len() - 1;
        cells
            .iter()
            .enumerate()
            .map(|(index, cell)| {
                if index == last {
                    (*cell).to_owned()
                } else {
                    format!("{cell:<width$}", width = widths[index])
                }
            })
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_owned()
    };
    let mut lines = vec![render(header.to_vec())];
    lines.extend(
        rows.iter()
            .map(|row| render(row.iter().map(String::as_str).collect())),
    );
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use functor_model::{
        ArtifactId, ArtifactType, ContentFormat, ContentPath, Metadata, MetadataKey, NewArtifact,
        Slug, Tag, Tags, Timestamp, Title,
    };

    use super::*;

    fn artifact() -> Artifact {
        let mut metadata = Metadata::new();
        metadata.insert(MetadataKey::new("author").unwrap(), "Paulo".into());
        metadata.insert(MetadataKey::new("count").unwrap(), 3_i64.into());
        let new = NewArtifact::new(
            Slug::parse("rfc-0000").unwrap(),
            ArtifactType::new("rfc").unwrap(),
            Title::new("Functor Vision").unwrap(),
        )
        .with_description("Platform vision")
        .with_tags(Tags::new(["b", "a"].map(|t| Tag::new(t).unwrap())))
        .with_metadata(metadata)
        .with_content(Content::Path {
            format: ContentFormat::new("markdown").unwrap(),
            path: ContentPath::new("content/rfc-0000.md").unwrap(),
        });
        Artifact::create(
            ArtifactId::from_bits(1),
            new,
            Timestamp::parse("2026-10-12T18:00:00Z").unwrap(),
        )
    }

    #[test]
    fn json_artifacts_have_the_documented_shape_and_order() {
        let dto = ArtifactDto::new(&artifact(), None);
        let text = serde_json::to_string(&dto).unwrap();
        let order = [
            "\"id\"",
            "\"ref\"",
            "\"slug\"",
            "\"type\"",
            "\"title\"",
            "\"revision\"",
            "\"description\"",
            "\"tags\"",
            "\"metadata\"",
            "\"content\"",
            "\"created\"",
            "\"updated\"",
        ];
        let positions: Vec<usize> = order.iter().map(|k| text.find(k).unwrap()).collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]), "{text}");
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(value["tags"], serde_json::json!(["a", "b"]));
        assert_eq!(
            value["metadata"],
            serde_json::json!({"author": "Paulo", "count": 3})
        );
        assert_eq!(
            value["content"],
            serde_json::json!({"format": "markdown", "path": "content/rfc-0000.md"})
        );
        assert_eq!(
            value["ref"],
            format!("functor:artifact:{}", ArtifactId::from_bits(1))
        );
    }

    #[test]
    fn absent_values_are_null_and_collections_empty() {
        let bare = Artifact::create(
            ArtifactId::from_bits(2),
            NewArtifact::new(
                Slug::parse("a").unwrap(),
                ArtifactType::document(),
                Title::new("a").unwrap(),
            ),
            Timestamp::parse("2026-10-12T18:00:00Z").unwrap(),
        );
        let value = serde_json::to_value(ArtifactDto::new(&bare, None)).unwrap();
        assert_eq!(value["description"], Value::Null);
        assert_eq!(value["content"], Value::Null);
        assert_eq!(value["tags"], serde_json::json!([]));
        assert_eq!(value["metadata"], serde_json::json!({}));
    }

    #[test]
    fn resolved_text_is_added_to_path_content_only_when_given() {
        let value =
            serde_json::to_value(ArtifactDto::new(&artifact(), Some("# hi".into()))).unwrap();
        assert_eq!(value["content"]["text"], "# hi");
        assert_eq!(value["content"]["path"], "content/rfc-0000.md");
    }

    #[test]
    fn update_documents_list_changed_fields_and_others_do_not() {
        let dto = || ArtifactDto::new(&artifact(), None);
        let created = serde_json::to_value(ArtifactDocument::new(dto())).unwrap();
        assert!(created.get("changed").is_none());
        let updated =
            serde_json::to_value(ArtifactDocument::updated(dto(), &[ArtifactField::Title]))
                .unwrap();
        assert_eq!(updated["changed"], serde_json::json!(["title"]));
        assert_eq!(updated["schema_version"], 1);
        let none = serde_json::to_value(ArtifactDocument::updated(dto(), &[])).unwrap();
        assert_eq!(none["changed"], serde_json::json!([]));
    }

    #[test]
    fn show_text_lists_what_is_present() {
        let text = show_text(&artifact(), None);
        assert_eq!(
            text,
            format!(
                "slug:        rfc-0000\nid:          {}\ntype:        rfc\ntitle:       Functor Vision\nrevision:    1\ncreated:     2026-10-12T18:00:00Z\nupdated:     2026-10-12T18:00:00Z\ndescription: Platform vision\ntags:        a, b\nmetadata:\n  author: Paulo\n  count: 3\ncontent:     markdown, path content/rfc-0000.md",
                ArtifactId::from_bits(1)
            )
        );
        assert!(show_text(&artifact(), Some("body")).ends_with("\n\nbody"));
    }

    #[test]
    fn control_characters_cannot_reach_the_terminal() {
        assert_eq!(sanitize("a\u{1b}[31mb\u{7}"), "a\\u{1b}[31mb\\u{7}");
        assert_eq!(sanitize("tab\there"), "tab\there");
        assert_eq!(sanitize("line\nbreak"), "line\\nbreak");
    }

    #[test]
    fn blocks_keep_their_line_breaks_but_not_escape_sequences() {
        assert_eq!(sanitize_block("a\nb\r\nc"), "a\nb\nc");
        assert_eq!(sanitize_block("x\u{1b}[0m\ny"), "x\\u{1b}[0m\ny");
        assert!(show_text(&artifact(), Some("one\ntwo\n")).ends_with("\n\none\ntwo"));
    }

    #[test]
    fn tables_align_columns_and_trim_trailing_space() {
        let rows = vec![
            vec![
                "rfc-0000".to_owned(),
                "rfc".to_owned(),
                "1".to_owned(),
                "Vision".to_owned(),
            ],
            vec![
                "a".to_owned(),
                "document".to_owned(),
                "12".to_owned(),
                String::new(),
            ],
        ];
        assert_eq!(
            table(&["SLUG", "TYPE", "REV", "TITLE"], &rows),
            "SLUG      TYPE      REV  TITLE\nrfc-0000  rfc       1    Vision\na         document  12"
        );
    }

    #[test]
    fn confirmations() {
        let a = artifact();
        assert_eq!(
            created_text(&a),
            format!("created artifact rfc-0000 ({})", a.id())
        );
        assert_eq!(updated_text(&a, &[]), "no changes");
        assert_eq!(
            updated_text(&a, &[ArtifactField::Title]),
            "updated artifact rfc-0000 (revision 1)"
        );
    }
}
