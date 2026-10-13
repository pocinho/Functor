//! Artifacts and their value types (RFC-0003).
//!
//! See `docs/domain-model.md` and `docs/artifact-format.md`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::num::NonZeroU64;
use std::str::FromStr;

use crate::error::{ModelError, Result};
use crate::grammar::{
    check_name, check_text_line, is_lower, is_lower_digit, is_lower_digit_underscore,
};
use crate::id::{ArtifactId, Slug};
use crate::timestamp::Timestamp;

macro_rules! name_type {
    ($(#[$meta:meta])* $name:ident, $field:literal, $max:expr, $first:path, $rest:path, $shape:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// Validates and wraps the text.
            ///
            /// # Errors
            /// Fails when the text violates the grammar.
            pub fn new(input: &str) -> Result<Self> {
                check_name($field, input, $max, $first, $rest, $shape)?;
                Ok(Self(input.to_owned()))
            }

            /// The text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = ModelError;

            fn from_str(input: &str) -> Result<Self> {
                Self::new(input)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

fn is_tag_rest(c: char) -> bool {
    is_lower_digit(c) || c == '_' || c == '-'
}

fn is_format_rest(c: char) -> bool {
    is_lower_digit(c) || matches!(c, '_' | '.' | '+' | '-')
}

name_type!(
    /// Classifies an artifact (RFC-0003 "Type"): `[a-z][a-z0-9_]{0,31}`. The set is open.
    ArtifactType, "artifact type", 32, is_lower, is_lower_digit_underscore, "[a-z][a-z0-9_]{0,31}"
);
name_type!(
    /// A tag: `[a-z0-9][a-z0-9_-]{0,39}`.
    Tag, "tag", 40, is_lower_digit, is_tag_rest, "[a-z0-9][a-z0-9_-]{0,39}"
);
name_type!(
    /// A metadata key: `[a-z0-9][a-z0-9_-]{0,39}`.
    MetadataKey, "metadata key", 40, is_lower_digit, is_tag_rest, "[a-z0-9][a-z0-9_-]{0,39}"
);
name_type!(
    /// A content format such as `markdown` or `toml`: `[a-z0-9][a-z0-9_.+-]{0,31}`.
    ContentFormat, "content format", 32, is_lower_digit, is_format_rest, "[a-z0-9][a-z0-9_.+-]{0,31}"
);

impl ArtifactType {
    /// The well-known type `document`.
    pub const DOCUMENT: &'static str = "document";
    /// The well-known type `rfc`.
    pub const RFC: &'static str = "rfc";
    /// The well-known type `note`.
    pub const NOTE: &'static str = "note";
    /// The well-known type `specification`.
    pub const SPECIFICATION: &'static str = "specification";
    /// The well-known type `dataset`.
    pub const DATASET: &'static str = "dataset";
    /// The well-known type `workflow`.
    pub const WORKFLOW: &'static str = "workflow";
    /// The well-known type `source`.
    pub const SOURCE: &'static str = "source";
    /// The well-known type `report`.
    pub const REPORT: &'static str = "report";

    /// The default type, `document`.
    #[must_use]
    pub fn document() -> Self {
        Self(Self::DOCUMENT.to_owned())
    }
}

/// An artifact title: trimmed, non-empty, at most 200 characters, without control characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Title(String);

impl Title {
    /// The maximum number of characters.
    pub const MAX_CHARS: usize = 200;

    /// Validates the text and stores it trimmed.
    ///
    /// # Errors
    /// Fails when the title is empty, too long or contains control characters.
    pub fn new(input: &str) -> Result<Self> {
        check_text_line("title", input, Self::MAX_CHARS).map(Self)
    }

    /// The title text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Title {
    type Err = ModelError;

    fn from_str(input: &str) -> Result<Self> {
        Self::new(input)
    }
}

/// The revision counter of an artifact: starts at 1 and increments on every update.
///
/// This is the MVP form of RFC-0003 "Version"; full versioning is deferred (RFC-0016).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Revision(NonZeroU64);

impl Revision {
    /// The first revision.
    pub const FIRST: Self = Self(NonZeroU64::MIN);

    /// Wraps a revision number.
    ///
    /// # Errors
    /// Fails when the number is zero.
    pub fn new(number: u64) -> Result<Self> {
        NonZeroU64::new(number)
            .map(Self)
            .ok_or_else(|| ModelError::new("revision", "must be at least 1"))
    }

    /// The revision number.
    #[must_use]
    pub fn get(self) -> u64 {
        self.0.get()
    }

    /// The next revision, or `None` if the counter would overflow.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A sorted, de-duplicated set of tags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Tags(BTreeSet<Tag>);

impl Tags {
    /// Builds a tag set from tags; duplicates collapse and order is normalized.
    #[must_use]
    pub fn new(tags: impl IntoIterator<Item = Tag>) -> Self {
        Self(tags.into_iter().collect())
    }

    /// Whether the set contains the tag.
    #[must_use]
    pub fn contains(&self, tag: &Tag) -> bool {
        self.0.contains(tag)
    }

    /// Iterates tags in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = &Tag> {
        self.0.iter()
    }

    /// The number of tags.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no tags.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromIterator<Tag> for Tags {
    fn from_iter<T: IntoIterator<Item = Tag>>(iter: T) -> Self {
        Self::new(iter)
    }
}

/// A metadata value: a string, an integer, a boolean or a list of strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MetadataValue {
    /// A string.
    String(String),
    /// A signed integer.
    Integer(i64),
    /// A boolean.
    Bool(bool),
    /// A list of strings.
    StringList(Vec<String>),
}

impl MetadataValue {
    /// Whether this value satisfies a metadata filter `needle`.
    ///
    /// Equal values match. A string list also matches a string `needle` it contains
    /// (see `docs/query-model-mvp.md`).
    #[must_use]
    pub fn matches(&self, needle: &Self) -> bool {
        match (self, needle) {
            (Self::StringList(items), Self::String(wanted)) => items.contains(wanted),
            _ => self == needle,
        }
    }
}

impl From<&str> for MetadataValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for MetadataValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<i64> for MetadataValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<bool> for MetadataValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Vec<String>> for MetadataValue {
    fn from(value: Vec<String>) -> Self {
        Self::StringList(value)
    }
}

/// Extensible key/value metadata. It never alters semantics (RFC-0003, RFC-0004).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Metadata(BTreeMap<MetadataKey, MetadataValue>);

impl Metadata {
    /// Creates empty metadata.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a key, returning the previous value if any.
    pub fn insert(&mut self, key: MetadataKey, value: MetadataValue) -> Option<MetadataValue> {
        self.0.insert(key, value)
    }

    /// Removes a key, returning its value if it was present.
    pub fn remove(&mut self, key: &MetadataKey) -> Option<MetadataValue> {
        self.0.remove(key)
    }

    /// Looks up a key.
    #[must_use]
    pub fn get(&self, key: &MetadataKey) -> Option<&MetadataValue> {
        self.0.get(key)
    }

    /// Iterates entries in ascending key order.
    pub fn iter(&self) -> impl Iterator<Item = (&MetadataKey, &MetadataValue)> {
        self.0.iter()
    }

    /// The number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromIterator<(MetadataKey, MetadataValue)> for Metadata {
    fn from_iter<T: IntoIterator<Item = (MetadataKey, MetadataValue)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// A path to artifact content, relative to the repository root.
///
/// Uses `/` separators and is normalized: not absolute, no empty, `.` or `..` segments, no
/// backslashes, no drive prefixes and no control characters. Whether the file exists is checked
/// by validation (`V0010`), not here, because the model does no I/O.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentPath(String);

impl ContentPath {
    /// Validates and wraps a content path.
    ///
    /// # Errors
    /// Fails when the path is empty, absolute, not normalized or contains unsafe characters.
    pub fn new(input: &str) -> Result<Self> {
        let invalid = |reason: &str| Err(ModelError::new("content path", reason));
        if input.is_empty() {
            return invalid("must not be empty");
        }
        if input.starts_with('/') || input.contains('\\') || input.contains(':') {
            return invalid("must be relative and use `/` separators");
        }
        if input.chars().any(char::is_control) {
            return invalid("must not contain control characters");
        }
        if input
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        {
            return invalid("must not contain empty, `.` or `..` segments");
        }
        Ok(Self(input.to_owned()))
    }

    /// The path text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ContentPath {
    type Err = ModelError;

    fn from_str(input: &str) -> Result<Self> {
        Self::new(input)
    }
}

/// The primary representation of an artifact (RFC-0003 "Content").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Content {
    /// No content.
    #[default]
    None,
    /// Content stored inside the artifact file.
    Inline {
        /// The content format.
        format: ContentFormat,
        /// The content text.
        text: String,
    },
    /// Content stored in a separate file inside the repository.
    Path {
        /// The content format.
        format: ContentFormat,
        /// The location of the content file.
        path: ContentPath,
    },
}

/// A durable, identifiable unit of information (RFC-0003).
// `artifact_type` mirrors the RFC-0003 term and the documented field name.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    id: ArtifactId,
    slug: Slug,
    artifact_type: ArtifactType,
    title: Title,
    revision: Revision,
    description: Option<String>,
    tags: Tags,
    metadata: Metadata,
    content: Content,
    created: Timestamp,
    updated: Timestamp,
}

/// The data needed to create an artifact. Id, revision and timestamps are assigned on creation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NewArtifact {
    /// The slug alias.
    pub slug: Slug,
    /// The artifact type.
    pub artifact_type: ArtifactType,
    /// The title.
    pub title: Title,
    /// An optional description.
    pub description: Option<String>,
    /// Tags.
    pub tags: Tags,
    /// Metadata.
    pub metadata: Metadata,
    /// Content.
    pub content: Content,
}

impl NewArtifact {
    /// Starts a new artifact with the required fields.
    #[must_use]
    pub fn new(slug: Slug, artifact_type: ArtifactType, title: Title) -> Self {
        Self {
            slug,
            artifact_type,
            title,
            description: None,
            tags: Tags::default(),
            metadata: Metadata::default(),
            content: Content::None,
        }
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Tags) -> Self {
        self.tags = tags;
        self
    }

    /// Sets the metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets the content.
    #[must_use]
    pub fn with_content(mut self, content: Content) -> Self {
        self.content = content;
        self
    }
}

/// The mutable fields of an artifact. The type is deliberately absent: changing it is deferred.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ArtifactPatch {
    /// A new slug.
    pub slug: Option<Slug>,
    /// A new title.
    pub title: Option<Title>,
    /// `Some(Some(text))` sets the description, `Some(None)` clears it.
    pub description: Option<Option<String>>,
    /// Replaces the whole tag set.
    pub tags: Option<Tags>,
    /// Replaces the whole metadata map.
    pub metadata: Option<Metadata>,
    /// Replaces the content.
    pub content: Option<Content>,
}

impl ArtifactPatch {
    /// Creates a patch that changes nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Renames the slug.
    #[must_use]
    pub fn with_slug(mut self, slug: Slug) -> Self {
        self.slug = Some(slug);
        self
    }

    /// Changes the title.
    #[must_use]
    pub fn with_title(mut self, title: Title) -> Self {
        self.title = Some(title);
        self
    }

    /// Sets or clears the description.
    #[must_use]
    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = Some(description);
        self
    }

    /// Replaces the tags.
    #[must_use]
    pub fn with_tags(mut self, tags: Tags) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Replaces the metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Replaces the content.
    #[must_use]
    pub fn with_content(mut self, content: Content) -> Self {
        self.content = Some(content);
        self
    }
}

/// An artifact field that a patch changed; reported in `artifact.updated` events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ArtifactField {
    /// The slug.
    Slug,
    /// The title.
    Title,
    /// The description.
    Description,
    /// The tags.
    Tags,
    /// The metadata.
    Metadata,
    /// The content.
    Content,
}

impl ArtifactField {
    /// The field name as it appears in events and the CLI.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Slug => "slug",
            Self::Title => "title",
            Self::Description => "description",
            Self::Tags => "tags",
            Self::Metadata => "metadata",
            Self::Content => "content",
        }
    }
}

impl fmt::Display for ArtifactField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The result of applying a patch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patched {
    /// The resulting artifact. Identical to the input when `changed` is empty.
    pub artifact: Artifact,
    /// The fields whose values actually changed, in field order.
    pub changed: Vec<ArtifactField>,
}

impl Artifact {
    /// Creates a new artifact at revision 1 with `created = updated = now`.
    #[must_use]
    pub fn create(id: ArtifactId, new: NewArtifact, now: Timestamp) -> Self {
        Self {
            id,
            slug: new.slug,
            artifact_type: new.artifact_type,
            title: new.title,
            revision: Revision::FIRST,
            description: new.description,
            tags: new.tags,
            metadata: new.metadata,
            content: new.content,
            created: now,
            updated: now,
        }
    }

    /// Reconstitutes an artifact from stored parts, checking `updated >= created`.
    ///
    /// # Errors
    /// Fails when `updated` is earlier than `created`.
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: ArtifactId,
        slug: Slug,
        artifact_type: ArtifactType,
        title: Title,
        revision: Revision,
        description: Option<String>,
        tags: Tags,
        metadata: Metadata,
        content: Content,
        created: Timestamp,
        updated: Timestamp,
    ) -> Result<Self> {
        if updated < created {
            return Err(ModelError::new(
                "updated",
                "must not be earlier than created",
            ));
        }
        Ok(Self {
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
        })
    }

    /// Applies a patch. If nothing changes, the artifact is returned untouched (same revision and
    /// `updated`) with an empty `changed` list. Otherwise the revision increments and `updated`
    /// becomes `now`, but never earlier than `created`.
    ///
    /// # Errors
    /// Fails if the revision counter would overflow.
    pub fn apply_patch(&self, patch: ArtifactPatch, now: Timestamp) -> Result<Patched> {
        let mut next = self.clone();
        let mut changed = Vec::new();

        if let Some(slug) = patch.slug
            && slug != next.slug
        {
            next.slug = slug;
            changed.push(ArtifactField::Slug);
        }
        if let Some(title) = patch.title
            && title != next.title
        {
            next.title = title;
            changed.push(ArtifactField::Title);
        }
        if let Some(description) = patch.description
            && description != next.description
        {
            next.description = description;
            changed.push(ArtifactField::Description);
        }
        if let Some(tags) = patch.tags
            && tags != next.tags
        {
            next.tags = tags;
            changed.push(ArtifactField::Tags);
        }
        if let Some(metadata) = patch.metadata
            && metadata != next.metadata
        {
            next.metadata = metadata;
            changed.push(ArtifactField::Metadata);
        }
        if let Some(content) = patch.content
            && content != next.content
        {
            next.content = content;
            changed.push(ArtifactField::Content);
        }

        if changed.is_empty() {
            return Ok(Patched {
                artifact: self.clone(),
                changed,
            });
        }
        next.revision = self
            .revision
            .next()
            .ok_or_else(|| ModelError::new("revision", "counter overflow"))?;
        next.updated = now.max(self.created);
        Ok(Patched {
            artifact: next,
            changed,
        })
    }

    /// The identity.
    #[must_use]
    pub fn id(&self) -> &ArtifactId {
        &self.id
    }

    /// The slug alias.
    #[must_use]
    pub fn slug(&self) -> &Slug {
        &self.slug
    }

    /// The type.
    #[must_use]
    pub fn artifact_type(&self) -> &ArtifactType {
        &self.artifact_type
    }

    /// The title.
    #[must_use]
    pub fn title(&self) -> &Title {
        &self.title
    }

    /// The revision.
    #[must_use]
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// The description, if any.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The tags.
    #[must_use]
    pub fn tags(&self) -> &Tags {
        &self.tags
    }

    /// The metadata.
    #[must_use]
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// The content.
    #[must_use]
    pub fn content(&self) -> &Content {
        &self.content
    }

    /// When the artifact was created.
    #[must_use]
    pub fn created(&self) -> Timestamp {
        self.created
    }

    /// When the artifact was last updated.
    #[must_use]
    pub fn updated(&self) -> Timestamp {
        self.updated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Artifact {
        let new = NewArtifact::new(
            Slug::parse("rfc-0000").unwrap(),
            ArtifactType::new("rfc").unwrap(),
            Title::new("Functor Vision").unwrap(),
        );
        Artifact::create(
            ArtifactId::from_bits(1),
            new,
            Timestamp::parse("2026-10-12T18:00:00Z").unwrap(),
        )
    }

    #[test]
    fn name_grammars() {
        assert!(ArtifactType::new("rfc").is_ok());
        assert!(ArtifactType::new("a_1").is_ok());
        for bad in ["", "Rfc", "1rfc", "r-fc", &"a".repeat(33)] {
            assert!(ArtifactType::new(bad).is_err(), "{bad:?}");
        }
        assert!(Tag::new("core").is_ok());
        assert!(Tag::new("0-a_b").is_ok());
        for bad in ["", "-a", "_a", "A", &"a".repeat(41)] {
            assert!(Tag::new(bad).is_err(), "{bad:?}");
        }
        assert!(MetadataKey::new("author").is_ok());
        assert!(MetadataKey::new("Author").is_err());
        assert!(ContentFormat::new("markdown").is_ok());
        assert!(ContentFormat::new("text+x.y").is_ok());
        assert!(ContentFormat::new("Markdown").is_err());
        assert!(ContentFormat::new("").is_err());
    }

    #[test]
    fn unknown_artifact_types_are_accepted() {
        assert!(ArtifactType::new("totally_new_kind").is_ok());
    }

    #[test]
    fn title_is_trimmed_and_checked() {
        assert_eq!(Title::new("  Hello  ").unwrap().as_str(), "Hello");
        for bad in ["", "   ", "a\nb", "a\u{7}b"] {
            assert!(Title::new(bad).is_err(), "{bad:?}");
        }
        assert!(Title::new(&"x".repeat(200)).is_ok());
        assert!(Title::new(&"x".repeat(201)).is_err());
        assert!(Title::new(&"é".repeat(200)).is_ok());
    }

    #[test]
    fn revision_is_never_zero() {
        assert!(Revision::new(0).is_err());
        assert_eq!(Revision::new(1).unwrap(), Revision::FIRST);
        assert_eq!(Revision::FIRST.next().unwrap().get(), 2);
        assert!(Revision::new(u64::MAX).unwrap().next().is_none());
    }

    #[test]
    fn tags_are_sorted_and_unique() {
        let tags = Tags::new(["b", "a", "b"].map(|t| Tag::new(t).unwrap()));
        let listed: Vec<&str> = tags.iter().map(Tag::as_str).collect();
        assert_eq!(listed, ["a", "b"]);
    }

    #[test]
    fn content_path_rejects_unsafe_paths() {
        for good in ["a", "content/rfc-0000.md", "a/b/c.txt"] {
            assert!(ContentPath::new(good).is_ok(), "{good:?}");
        }
        for bad in [
            "",
            "/etc/passwd",
            "C:/x",
            "c:x",
            "a\\b",
            "../x",
            "a/../b",
            "a/./b",
            "a//b",
            "a/",
            ".",
            "a\u{0}b",
        ] {
            assert!(ContentPath::new(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn metadata_value_matching() {
        let list = MetadataValue::StringList(vec!["x".into(), "y".into()]);
        assert!(list.matches(&"x".into()));
        assert!(!list.matches(&"z".into()));
        assert!(list.matches(&list.clone()));
        assert!(MetadataValue::from(1_i64).matches(&1_i64.into()));
        assert!(!MetadataValue::from(1_i64).matches(&"1".into()));
    }

    #[test]
    fn create_starts_at_revision_one() {
        let artifact = sample();
        assert_eq!(artifact.revision(), Revision::FIRST);
        assert_eq!(artifact.created(), artifact.updated());
        assert_eq!(artifact.description(), None);
        assert_eq!(artifact.content(), &Content::None);
    }

    #[test]
    fn patch_increments_revision_and_reports_changes() {
        let artifact = sample();
        let now = Timestamp::parse("2026-10-14T09:30:00Z").unwrap();
        let patch = ArtifactPatch::new()
            .with_title(Title::new("Vision").unwrap())
            .with_description(Some("text".into()));
        let patched = artifact.apply_patch(patch, now).unwrap();
        assert_eq!(
            patched.changed,
            [ArtifactField::Title, ArtifactField::Description]
        );
        assert_eq!(patched.artifact.revision().get(), 2);
        assert_eq!(patched.artifact.updated(), now);
        assert_eq!(patched.artifact.created(), artifact.created());
        assert_eq!(patched.artifact.id(), artifact.id());
        assert_eq!(patched.artifact.artifact_type(), artifact.artifact_type());
    }

    #[test]
    fn patch_that_changes_nothing_is_a_no_op() {
        let artifact = sample();
        let now = Timestamp::parse("2027-01-01T00:00:00Z").unwrap();
        let patch = ArtifactPatch::new()
            .with_slug(artifact.slug().clone())
            .with_title(artifact.title().clone())
            .with_description(None);
        let patched = artifact.apply_patch(patch, now).unwrap();
        assert_eq!(patched.changed, []);
        assert_eq!(patched.artifact, artifact);
        assert_eq!(
            artifact
                .apply_patch(ArtifactPatch::new(), now)
                .unwrap()
                .artifact,
            artifact
        );
    }

    #[test]
    fn patch_can_clear_description_and_rename_slug() {
        let artifact = sample()
            .apply_patch(
                ArtifactPatch::new().with_description(Some("d".into())),
                Timestamp::parse("2026-10-13T00:00:00Z").unwrap(),
            )
            .unwrap()
            .artifact;
        let patched = artifact
            .apply_patch(
                ArtifactPatch::new()
                    .with_description(None)
                    .with_slug(Slug::parse("vision").unwrap()),
                Timestamp::parse("2026-10-14T00:00:00Z").unwrap(),
            )
            .unwrap();
        assert_eq!(
            patched.changed,
            [ArtifactField::Slug, ArtifactField::Description]
        );
        assert_eq!(patched.artifact.description(), None);
        assert_eq!(patched.artifact.slug().as_str(), "vision");
    }

    #[test]
    fn updated_never_precedes_created() {
        let artifact = sample();
        let earlier = Timestamp::parse("2020-01-01T00:00:00Z").unwrap();
        let patched = artifact
            .apply_patch(
                ArtifactPatch::new().with_title(Title::new("x").unwrap()),
                earlier,
            )
            .unwrap();
        assert_eq!(patched.artifact.updated(), artifact.created());
    }

    #[test]
    fn from_parts_checks_timestamps() {
        let a = sample();
        let build = |updated| {
            Artifact::from_parts(
                *a.id(),
                a.slug().clone(),
                a.artifact_type().clone(),
                a.title().clone(),
                a.revision(),
                None,
                Tags::default(),
                Metadata::default(),
                Content::None,
                a.created(),
                updated,
            )
        };
        assert!(build(a.created()).is_ok());
        assert!(build(Timestamp::MIN).is_err());
    }
}
