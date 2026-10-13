//! Where things live inside a repository.
//!
//! See `docs/design/repository-layout.md`.

use std::path::{Path, PathBuf};

use functor_model::{ArtifactId, ContentPath, RelationshipId, RepoPath};

/// Name of the directory marking a repository root.
pub const FUNCTOR_DIR: &str = ".functor";
/// File name used to keep empty directories in version control; ignored by loaders.
pub const GITKEEP: &str = ".gitkeep";
/// Marker in the names of temporary files left by interrupted writes.
pub const TEMP_MARKER: &str = ".tmp-";

/// The paths of a repository rooted at a directory.
///
/// All entity file names derive from validated ids, never from user text (see
/// `docs/design/storage-design.md`, "Path Safety").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    root: PathBuf,
}

impl Layout {
    /// Creates the layout for a repository root.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The repository root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `.functor/`
    #[must_use]
    pub fn functor_dir(&self) -> PathBuf {
        self.root.join(FUNCTOR_DIR)
    }

    /// `.functor/repository.toml`, the commit point of a repository.
    #[must_use]
    pub fn repository_file(&self) -> PathBuf {
        self.functor_dir().join("repository.toml")
    }

    /// `.functor/lock`, the advisory write lock.
    #[must_use]
    pub fn lock_file(&self) -> PathBuf {
        self.functor_dir().join("lock")
    }

    /// `artifacts/`
    #[must_use]
    pub fn artifacts_dir(&self) -> PathBuf {
        self.root.join("artifacts")
    }

    /// `relationships/`
    #[must_use]
    pub fn relationships_dir(&self) -> PathBuf {
        self.root.join("relationships")
    }

    /// `events/`
    #[must_use]
    pub fn events_dir(&self) -> PathBuf {
        self.root.join("events")
    }

    /// `events/current.log`
    #[must_use]
    pub fn events_log(&self) -> PathBuf {
        self.events_dir().join("current.log")
    }

    /// `events/archive/`
    #[must_use]
    pub fn events_archive_dir(&self) -> PathBuf {
        self.events_dir().join("archive")
    }

    /// `artifacts/<artifact-id>.toml`
    #[must_use]
    pub fn artifact_file(&self, id: &ArtifactId) -> PathBuf {
        self.artifacts_dir().join(format!("{id}.toml"))
    }

    /// `relationships/<relationship-id>.toml`
    #[must_use]
    pub fn relationship_file(&self, id: &RelationshipId) -> PathBuf {
        self.relationships_dir().join(format!("{id}.toml"))
    }

    /// The file a content path refers to. The path is already validated to be relative and
    /// normalized; callers must still check the result does not escape the root through links.
    #[must_use]
    pub fn content_file(&self, path: &ContentPath) -> PathBuf {
        let mut full = self.root.clone();
        full.extend(path.as_str().split('/'));
        full
    }

    /// Expresses `path` relative to the root with `/` separators, for use in findings and issues.
    ///
    /// Paths outside the root are returned as given.
    #[must_use]
    pub fn relative(&self, path: &Path) -> RepoPath {
        let shown = path.strip_prefix(&self.root).unwrap_or(path);
        let text = shown.to_string_lossy();
        RepoPath::new(&text).unwrap_or_else(|_| {
            RepoPath::new("?").unwrap_or_else(|_| unreachable!("`?` is a valid repository path"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_follow_the_documented_layout() {
        let layout = Layout::new("/repo");
        let artifact = ArtifactId::from_bits(1);
        let relationship = RelationshipId::from_bits(2);
        let rel = |path: PathBuf| layout.relative(&path).to_string();
        assert_eq!(rel(layout.repository_file()), ".functor/repository.toml");
        assert_eq!(rel(layout.lock_file()), ".functor/lock");
        assert_eq!(rel(layout.events_log()), "events/current.log");
        assert_eq!(rel(layout.events_archive_dir()), "events/archive");
        assert_eq!(
            rel(layout.artifact_file(&artifact)),
            format!("artifacts/{artifact}.toml")
        );
        assert_eq!(
            rel(layout.relationship_file(&relationship)),
            format!("relationships/{relationship}.toml")
        );
    }

    #[test]
    fn content_paths_stay_under_the_root() {
        let layout = Layout::new("/repo");
        let path = ContentPath::new("content/rfc-0000.md").unwrap();
        let full = layout.content_file(&path);
        assert!(full.starts_with("/repo"));
        assert_eq!(layout.relative(&full).to_string(), "content/rfc-0000.md");
    }

    #[test]
    fn paths_outside_the_root_are_kept_as_given() {
        let layout = Layout::new("/repo");
        assert_eq!(
            layout.relative(Path::new("/elsewhere/x")).to_string(),
            "/elsewhere/x"
        );
    }
}
