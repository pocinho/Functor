//! Repository lifecycle and read access traits.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use functor_model::{
    Artifact, Relationship, RepoPath, RepositoryInfo, RepositorySnapshot, SnapshotError, Subject,
};

use crate::error::{FunctorError, Result};

/// Options for creating a repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct InitOptions {
    /// The repository name; defaults to the directory name.
    pub name: Option<String>,
    /// An optional description.
    pub description: Option<String>,
    /// Allow creating a repository inside another repository.
    pub nested: bool,
}

impl InitOptions {
    /// Default options.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Allows nesting inside another repository.
    #[must_use]
    pub fn nested(mut self) -> Self {
        self.nested = true;
        self
    }
}

/// A storage-neutral handle: where a repository is and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryHandle {
    /// The repository root directory.
    pub root: PathBuf,
    /// The repository identity and metadata.
    pub info: RepositoryInfo,
}

/// Creates, opens and discovers repositories (RFC-0015).
#[async_trait]
pub trait RepositoryManager: Send + Sync {
    /// Creates a repository at `at`.
    ///
    /// Fails with `Conflict` if the target already contains `.functor/`, or an ancestor does and
    /// `options.nested` is not set.
    async fn init(&self, at: &Path, options: InitOptions) -> Result<RepositoryHandle>;

    /// Opens the repository rooted exactly at `root`.
    ///
    /// Fails with `NotARepository` or `UnsupportedFormat`.
    async fn open(&self, root: &Path) -> Result<RepositoryHandle>;

    /// Finds the repository containing `start` by walking up the directory tree.
    ///
    /// Fails with `NotARepository` if no ancestor is a repository.
    async fn discover(&self, start: &Path) -> Result<RepositoryHandle>;
}

/// What kind of problem a loader found in repository data.
///
/// Each kind maps to a validation rule (see `docs/storage-design.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LoadIssueKind {
    /// Invalid TOML, a missing or invalid required field, a wrong type, or a file over the size
    /// cap (`V0001`).
    MalformedFile,
    /// An entity file has no `id` (`V0002`).
    MissingIdentifier,
    /// Two artifact files claim the same id (`V0003`).
    DuplicateArtifactId,
    /// Two artifacts share a slug (`V0004`).
    DuplicateSlug,
    /// A malformed identifier or slug (`V0005`).
    InvalidIdentifier,
    /// A relationship of an acyclic type points at its own source (`V0009`).
    SelfReference,
    /// Two relationship files claim the same id (`V0008`).
    DuplicateRelationshipId,
    /// A content path escapes the repository or is otherwise unsafe (`V0010`).
    UnsafeContentPath,
    /// A referenced content file does not exist (`V0010`).
    MissingContentFile,
    /// An unsupported `format_version` (`V0013`).
    UnsupportedFormat,
    /// A required repository file or directory is missing (`V0014`).
    MissingStructure,
    /// An unexpected file in `artifacts/` or `relationships/`, or a stray temporary file
    /// (`V0016`). Does not prevent a strict load.
    UnexpectedFile,
    /// The file name does not match the entity id (`V0017`).
    FilenameMismatch,
    /// An unknown field kept for forward compatibility (`V0018`). Does not prevent a strict load.
    UnknownField,
    /// A file or directory could not be read; the validation outcome becomes `Unknown`.
    Unreadable,
}

impl LoadIssueKind {
    /// Whether the issue prevents a strict load. Unexpected files and unknown fields do not.
    #[must_use]
    pub fn is_fatal(self) -> bool {
        !matches!(self, Self::UnexpectedFile | Self::UnknownField)
    }
}

/// One problem found while loading a repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadIssue {
    /// The kind of problem.
    pub kind: LoadIssueKind,
    /// The file the problem was found in.
    pub path: RepoPath,
    /// A human-readable explanation, including other involved files for duplicates.
    pub detail: String,
    /// The entity the file defines, when the file could be read far enough to know it. Scoped
    /// validation uses it to decide which problems concern an artifact.
    pub entity: Option<Subject>,
    /// Other subjects involved, for example the file that defined a duplicate id first.
    pub related: Vec<Subject>,
}

impl LoadIssue {
    /// Creates an issue about a file.
    #[must_use]
    pub fn new(kind: LoadIssueKind, path: RepoPath, detail: impl Into<String>) -> Self {
        Self {
            kind,
            path,
            detail: detail.into(),
            entity: None,
            related: Vec::new(),
        }
    }

    /// Names the entity the file defines.
    #[must_use]
    pub fn with_entity(mut self, entity: Subject) -> Self {
        self.entity = Some(entity);
        self
    }

    /// Adds related subjects.
    #[must_use]
    pub fn with_related(mut self, related: impl IntoIterator<Item = Subject>) -> Self {
        self.related = related.into_iter().collect();
        self
    }
}

/// The result of a lenient load: everything that could be read, plus every problem found.
#[derive(Debug, Clone)]
pub struct SnapshotLoad {
    /// The repository information, or the issue that prevented reading it.
    pub info: std::result::Result<RepositoryInfo, LoadIssue>,
    /// The artifacts that loaded cleanly, in any order.
    pub artifacts: Vec<Artifact>,
    /// The relationships that loaded cleanly, in any order.
    pub relationships: Vec<Relationship>,
    /// Every problem found.
    pub issues: Vec<LoadIssue>,
}

impl SnapshotLoad {
    /// Whether any issue prevents a strict load.
    #[must_use]
    pub fn has_fatal_issues(&self) -> bool {
        self.info.is_err() || self.issues.iter().any(|issue| issue.kind.is_fatal())
    }

    /// Converts a lenient load into a strict snapshot.
    ///
    /// # Errors
    /// Returns `Inconsistent` (pointing the user to `functor validate`) when a fatal issue exists
    /// or the entities do not form a well-formed snapshot.
    pub fn into_snapshot(self) -> Result<RepositorySnapshot> {
        let info = match self.info {
            Ok(info) => info,
            Err(issue) => {
                return Err(FunctorError::Inconsistent {
                    detail: format!("{}: {}", issue.path, issue.detail),
                });
            }
        };
        if let Some(issue) = self.issues.iter().find(|issue| issue.kind.is_fatal()) {
            return Err(FunctorError::Inconsistent {
                detail: format!("{}: {}", issue.path, issue.detail),
            });
        }
        RepositorySnapshot::new(info, self.artifacts, self.relationships)
            .map_err(|error: SnapshotError| error.into())
    }
}

/// Read access to the state of an opened repository.
#[async_trait]
pub trait RepositoryReader: Send + Sync {
    /// The repository identity and metadata.
    async fn info(&self) -> Result<RepositoryInfo>;

    /// Lenient load used by validation. Never fails because of bad entity data; only because the
    /// repository cannot be accessed at all.
    async fn load(&self) -> Result<SnapshotLoad>;

    /// Strict load: fails with `Inconsistent` on any malformed or inconsistent entity.
    ///
    /// The default implementation converts [`RepositoryReader::load`]; implementations may
    /// override it for efficiency but must keep the same semantics.
    async fn snapshot(&self) -> Result<RepositorySnapshot> {
        self.load().await?.into_snapshot()
    }
}

#[cfg(test)]
mod tests {
    use functor_model::{
        ArtifactId, ArtifactType, FormatVersion, NewArtifact, RepositoryId, RepositoryName, Slug,
        Timestamp, Title,
    };

    use super::*;
    use crate::error::ErrorKind;

    fn at() -> Timestamp {
        Timestamp::parse("2026-10-12T18:00:00Z").unwrap()
    }

    fn info() -> RepositoryInfo {
        RepositoryInfo::new(
            RepositoryId::from_bits(1),
            RepositoryName::new("Demo").unwrap(),
            None,
            FormatVersion::CURRENT,
            at(),
        )
    }

    fn artifact(bits: u128, slug: &str) -> Artifact {
        Artifact::create(
            ArtifactId::from_bits(bits),
            NewArtifact::new(
                Slug::parse(slug).unwrap(),
                ArtifactType::document(),
                Title::new(slug).unwrap(),
            ),
            at(),
        )
    }

    fn issue(kind: LoadIssueKind) -> LoadIssue {
        LoadIssue::new(kind, RepoPath::new("artifacts/x.toml").unwrap(), "detail")
    }

    fn load(issues: Vec<LoadIssue>) -> SnapshotLoad {
        SnapshotLoad {
            info: Ok(info()),
            artifacts: vec![artifact(1, "a")],
            relationships: Vec::new(),
            issues,
        }
    }

    #[test]
    fn clean_load_becomes_a_snapshot() {
        let snapshot = load(Vec::new()).into_snapshot().unwrap();
        assert_eq!(snapshot.artifacts().len(), 1);
    }

    #[test]
    fn issues_start_without_an_entity_and_gain_context_through_builders() {
        let plain = issue(LoadIssueKind::MalformedFile);
        assert_eq!(plain.entity, None);
        assert_eq!(plain.related, []);

        let other = Subject::File(RepoPath::new("artifacts/first.toml").unwrap());
        let rich = issue(LoadIssueKind::DuplicateSlug)
            .with_entity(Subject::Artifact(ArtifactId::from_bits(2)))
            .with_related([other.clone()]);
        assert_eq!(
            rich.entity,
            Some(Subject::Artifact(ArtifactId::from_bits(2)))
        );
        assert_eq!(rich.related, [other]);
    }

    #[test]
    fn non_fatal_issues_do_not_block_a_strict_load() {
        let load = load(vec![
            issue(LoadIssueKind::UnexpectedFile),
            issue(LoadIssueKind::UnknownField),
        ]);
        assert!(!load.has_fatal_issues());
        assert!(load.into_snapshot().is_ok());
    }

    #[test]
    fn fatal_issues_block_a_strict_load() {
        for kind in [
            LoadIssueKind::MalformedFile,
            LoadIssueKind::MissingIdentifier,
            LoadIssueKind::DuplicateArtifactId,
            LoadIssueKind::DuplicateSlug,
            LoadIssueKind::InvalidIdentifier,
            LoadIssueKind::SelfReference,
            LoadIssueKind::DuplicateRelationshipId,
            LoadIssueKind::UnsafeContentPath,
            LoadIssueKind::MissingContentFile,
            LoadIssueKind::UnsupportedFormat,
            LoadIssueKind::MissingStructure,
            LoadIssueKind::FilenameMismatch,
            LoadIssueKind::Unreadable,
        ] {
            let load = load(vec![issue(kind)]);
            assert!(load.has_fatal_issues(), "{kind:?}");
            let error = load.into_snapshot().unwrap_err();
            assert_eq!(error.kind(), ErrorKind::Inconsistent, "{kind:?}");
            assert!(error.to_string().contains("functor validate"));
        }
    }

    #[test]
    fn unreadable_repository_info_blocks_a_strict_load() {
        let mut load = load(Vec::new());
        load.info = Err(issue(LoadIssueKind::MalformedFile));
        assert!(load.has_fatal_issues());
        assert_eq!(
            load.into_snapshot().unwrap_err().kind(),
            ErrorKind::Inconsistent
        );
    }

    #[test]
    fn inconsistent_entities_block_a_strict_load() {
        let mut load = load(Vec::new());
        load.artifacts.push(artifact(2, "a"));
        assert_eq!(
            load.into_snapshot().unwrap_err().kind(),
            ErrorKind::Inconsistent
        );
    }

    #[test]
    fn init_options_builders() {
        let options = InitOptions::new()
            .with_name("n")
            .with_description("d")
            .nested();
        assert_eq!(options.name.as_deref(), Some("n"));
        assert_eq!(options.description.as_deref(), Some("d"));
        assert!(options.nested);
        assert!(!InitOptions::default().nested);
    }
}
