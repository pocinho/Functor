//! The `.functor/repository.toml` file: parsing and canonical rendering.
//!
//! See `docs/design/repository-layout.md`.

use functor_core::LoadIssueKind;
use functor_model::{FormatVersion, RepositoryId, RepositoryInfo, RepositoryName};
use toml_edit::{DocumentMut, Item, Value};

use crate::toml_support::{
    FileProblem, optional_integer, optional_str, required_str, required_timestamp, set_value,
    timestamp_value, unknown_keys,
};

/// The keys this release understands, in canonical order.
pub const KNOWN_KEYS: [&str; 5] = ["id", "name", "description", "format_version", "created"];

/// Why a repository file could not be turned into repository information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryFileError {
    /// The file is malformed or inconsistent.
    Problem(FileProblem),
    /// The file declares a format version this release does not support.
    UnsupportedFormat {
        /// The version found in the file.
        found: u32,
    },
}

impl From<FileProblem> for RepositoryFileError {
    fn from(problem: FileProblem) -> Self {
        Self::Problem(problem)
    }
}

/// A parsed repository file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRepositoryFile {
    /// The repository identity and metadata.
    pub info: RepositoryInfo,
    /// Keys not understood by this release, kept for forward compatibility (`V0018`).
    pub unknown_fields: Vec<String>,
}

/// Parses the text of `repository.toml`.
///
/// The format version is checked before anything else is interpreted.
///
/// # Errors
/// Returns [`RepositoryFileError::UnsupportedFormat`] for another format version, and
/// [`RepositoryFileError::Problem`] for anything malformed.
pub fn parse_repository_file(text: &str) -> Result<ParsedRepositoryFile, RepositoryFileError> {
    let document: DocumentMut = text
        .parse()
        .map_err(|error| FileProblem::malformed(format!("invalid TOML: {error}")))?;
    let table = document.as_table();

    let declared = optional_integer(table, "format_version")?
        .ok_or_else(|| FileProblem::malformed("missing field `format_version`"))?;
    let version = u32::try_from(declared)
        .map_err(|_| FileProblem::malformed("`format_version` must be a positive integer"))?;
    if FormatVersion::new(version) != FormatVersion::CURRENT {
        return Err(RepositoryFileError::UnsupportedFormat { found: version });
    }

    let id: RepositoryId = required_str(table, "id")?
        .parse()
        .map_err(|error| FileProblem::new(LoadIssueKind::InvalidIdentifier, format!("{error}")))?;
    let name = RepositoryName::new(required_str(table, "name")?)
        .map_err(|error| FileProblem::malformed(error.to_string()))?;
    let description = optional_str(table, "description")?.map(str::to_owned);
    let created = required_timestamp(table, "created")?;

    Ok(ParsedRepositoryFile {
        info: RepositoryInfo::new(id, name, description, FormatVersion::new(version), created),
        unknown_fields: unknown_keys(table, &KNOWN_KEYS),
    })
}

/// Renders `repository.toml` in the canonical form.
#[must_use]
pub fn render_repository_file(info: &RepositoryInfo) -> String {
    let mut document = DocumentMut::new();
    let table = document.as_table_mut();
    table.insert("id", Item::Value(Value::from(info.id().to_string())));
    table.insert("name", Item::Value(Value::from(info.name().as_str())));
    if let Some(description) = info.description() {
        table.insert("description", Item::Value(Value::from(description)));
    }
    table.insert(
        "format_version",
        Item::Value(Value::from(i64::from(info.format_version().get()))),
    );
    set_value(table, "created", timestamp_value(info.created()));
    document.to_string()
}

#[cfg(test)]
mod tests {
    use functor_model::Timestamp;

    use super::*;

    fn info(description: Option<&str>) -> RepositoryInfo {
        RepositoryInfo::new(
            RepositoryId::from_bits(1),
            RepositoryName::new("Functor Core").unwrap(),
            description.map(str::to_owned),
            FormatVersion::CURRENT,
            Timestamp::parse("2026-10-12T18:00:00Z").unwrap(),
        )
    }

    #[test]
    fn renders_the_canonical_form() {
        let text = render_repository_file(&info(Some("Core RFC set")));
        assert_eq!(
            text,
            format!(
                "id = \"{}\"\nname = \"Functor Core\"\ndescription = \"Core RFC set\"\nformat_version = 1\ncreated = 2026-10-12T18:00:00Z\n",
                RepositoryId::from_bits(1)
            )
        );
        let without = render_repository_file(&info(None));
        assert!(!without.contains("description"));
    }

    #[test]
    fn rendered_files_parse_back() {
        for description in [None, Some("with \"quotes\" and\nnewline")] {
            let original = info(description);
            let parsed = parse_repository_file(&render_repository_file(&original)).unwrap();
            assert_eq!(parsed.info, original);
            assert_eq!(parsed.unknown_fields, Vec::<String>::new());
        }
    }

    #[test]
    fn rendering_is_deterministic() {
        assert_eq!(
            render_repository_file(&info(None)),
            render_repository_file(&info(None))
        );
    }

    #[test]
    fn unsupported_format_versions_are_detected_before_anything_else() {
        let text = "format_version = 2\nthis = \"is not even valid fields\"\n";
        assert_eq!(
            parse_repository_file(text),
            Err(RepositoryFileError::UnsupportedFormat { found: 2 })
        );
        assert_eq!(
            parse_repository_file("format_version = 0\n"),
            Err(RepositoryFileError::UnsupportedFormat { found: 0 })
        );
    }

    #[test]
    fn malformed_files_are_classified() {
        let id = RepositoryId::from_bits(1);
        let problem = |text: &str| match parse_repository_file(text) {
            Err(RepositoryFileError::Problem(p)) => p,
            other => panic!("expected a problem, got {other:?}"),
        };
        assert_eq!(problem("not toml = = =").kind, LoadIssueKind::MalformedFile);
        assert_eq!(problem("id = \"x\"\n").kind, LoadIssueKind::MalformedFile);
        assert!(problem("name = \"n\"\n").detail.contains("format_version"));
        assert_eq!(
            problem("format_version = -1\n").kind,
            LoadIssueKind::MalformedFile
        );
        let bad_id =
            "id = \"nope\"\nname = \"n\"\nformat_version = 1\ncreated = 2026-10-12T18:00:00Z\n";
        assert_eq!(problem(bad_id).kind, LoadIssueKind::InvalidIdentifier);
        let no_name =
            format!("id = \"{id}\"\nformat_version = 1\ncreated = 2026-10-12T18:00:00Z\n");
        assert!(problem(&no_name).detail.contains("name"));
        let empty_name = format!(
            "id = \"{id}\"\nname = \"  \"\nformat_version = 1\ncreated = 2026-10-12T18:00:00Z\n"
        );
        assert_eq!(problem(&empty_name).kind, LoadIssueKind::MalformedFile);
        let bad_time =
            format!("id = \"{id}\"\nname = \"n\"\nformat_version = 1\ncreated = \"yesterday\"\n");
        assert_eq!(problem(&bad_time).kind, LoadIssueKind::MalformedFile);
    }

    #[test]
    fn unknown_fields_and_comments_are_tolerated() {
        let id = RepositoryId::from_bits(1);
        let text = format!(
            "# my repository\nid = \"{id}\"\nname = \"n\"\nformat_version = 1\ncreated = 2026-10-12T18:00:00Z\nowner = \"someone\"\n"
        );
        let parsed = parse_repository_file(&text).unwrap();
        assert_eq!(parsed.unknown_fields, ["owner"]);
    }

    #[test]
    fn crlf_files_parse() {
        let id = RepositoryId::from_bits(1);
        let text = format!(
            "id = \"{id}\"\r\nname = \"n\"\r\nformat_version = 1\r\ncreated = 2026-10-12T18:00:00Z\r\n"
        );
        assert!(parse_repository_file(&text).is_ok());
    }
}
