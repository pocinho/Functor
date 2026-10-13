//! The lenient loader: reads every entity file it can and reports every problem it finds.
//!
//! See `docs/storage-design.md`, "Read Protocol". The result feeds `validate` (all issues) and
//! strict snapshots (`SnapshotLoad::into_snapshot`).

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use functor_core::{FunctorError, LoadIssue, LoadIssueKind, Result, SnapshotLoad};
use functor_model::{Artifact, ArtifactId, Content, RelationshipId, RepositoryInfo, Slug};
use functor_repository::{
    GITKEEP, Layout, RepositoryFileError, TEMP_MARKER, parse_repository_file,
};

use crate::artifact_file::parse_artifact_file;
use crate::relationship_file::parse_relationship_file;

/// Entity and repository files larger than this are rejected as malformed.
pub(crate) const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

struct Candidate {
    path: PathBuf,
    name: String,
}

struct Loader<'a> {
    layout: &'a Layout,
    issues: Vec<LoadIssue>,
}

impl Loader<'_> {
    fn issue(&mut self, kind: LoadIssueKind, path: &Path, detail: impl Into<String>) {
        let relative = self.layout.relative(path);
        self.issues.push(LoadIssue::new(kind, relative, detail));
    }

    // Reads a file with the size cap, classifying failures as issues.
    fn read_text(&mut self, path: &Path) -> Option<String> {
        match fs::metadata(path) {
            Ok(metadata) if metadata.len() > MAX_FILE_BYTES => {
                self.issue(
                    LoadIssueKind::MalformedFile,
                    path,
                    format!("file is larger than {} MiB", MAX_FILE_BYTES / (1024 * 1024)),
                );
                return None;
            }
            Ok(_) => {}
            Err(error) => {
                self.issue(LoadIssueKind::Unreadable, path, error.to_string());
                return None;
            }
        }
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.issue(LoadIssueKind::Unreadable, path, error.to_string());
                return None;
            }
        };
        if let Ok(text) = String::from_utf8(bytes) {
            Some(text)
        } else {
            self.issue(
                LoadIssueKind::MalformedFile,
                path,
                "file is not valid UTF-8",
            );
            None
        }
    }

    // Lists `*.toml` entity files in `dir`, sorted by name. Anything else is an unexpected file.
    fn list(&mut self, dir: &Path) -> Vec<Candidate> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                self.issue(
                    LoadIssueKind::MissingStructure,
                    dir,
                    "required directory is missing",
                );
                return Vec::new();
            }
            Err(error) => {
                self.issue(LoadIssueKind::Unreadable, dir, error.to_string());
                return Vec::new();
            }
        };
        let mut candidates = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    self.issue(LoadIssueKind::Unreadable, dir, error.to_string());
                    continue;
                }
            };
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == GITKEEP {
                continue;
            }
            if name.contains(TEMP_MARKER) {
                self.issue(
                    LoadIssueKind::UnexpectedFile,
                    &path,
                    "leftover of an interrupted write",
                );
            } else if path.is_dir() || path.extension().is_none_or(|ext| ext != "toml") {
                self.issue(
                    LoadIssueKind::UnexpectedFile,
                    &path,
                    "not an entity file (expected `<id>.toml`)",
                );
            } else {
                candidates.push(Candidate { path, name });
            }
        }
        candidates.sort_by(|a, b| a.name.cmp(&b.name));
        candidates
    }

    fn info(&mut self) -> std::result::Result<RepositoryInfo, LoadIssue> {
        let path = self.layout.repository_file();
        let relative = self.layout.relative(&path);
        if !path.is_file() {
            return Err(LoadIssue::new(
                LoadIssueKind::MissingStructure,
                relative,
                "required file is missing",
            ));
        }
        let before = self.issues.len();
        let Some(text) = self.read_text(&path) else {
            return Err(self.issues.remove(before));
        };
        match parse_repository_file(&text) {
            Ok(parsed) => {
                if !parsed.unknown_fields.is_empty() {
                    self.issue(
                        LoadIssueKind::UnknownField,
                        &path,
                        format!("unknown field(s): {}", parsed.unknown_fields.join(", ")),
                    );
                }
                Ok(parsed.info)
            }
            Err(RepositoryFileError::UnsupportedFormat { found }) => Err(LoadIssue::new(
                LoadIssueKind::UnsupportedFormat,
                relative,
                format!("format version {found} is not supported (supported: 1)"),
            )),
            Err(RepositoryFileError::Problem(problem)) => {
                Err(LoadIssue::new(problem.kind, relative, problem.detail))
            }
        }
    }

    fn check_content(&mut self, file: &Path, artifact: &Artifact) {
        let Content::Path { path, .. } = artifact.content() else {
            return;
        };
        let full = self.layout.content_file(path);
        match fs::metadata(&full) {
            Err(error) if error.kind() == ErrorKind::NotFound => self.issue(
                LoadIssueKind::MissingContentFile,
                file,
                format!("content file `{path}` does not exist"),
            ),
            Err(error) => self.issue(LoadIssueKind::Unreadable, &full, error.to_string()),
            Ok(metadata) if !metadata.is_file() => self.issue(
                LoadIssueKind::MissingContentFile,
                file,
                format!("content path `{path}` is not a file"),
            ),
            Ok(_) => {
                // A link inside the repository must not lead outside it.
                let inside = match (
                    fs::canonicalize(&full),
                    fs::canonicalize(self.layout.root()),
                ) {
                    (Ok(real), Ok(root)) => real.starts_with(root),
                    _ => false,
                };
                if !inside {
                    self.issue(
                        LoadIssueKind::UnsafeContentPath,
                        file,
                        format!("content path `{path}` resolves outside the repository"),
                    );
                }
            }
        }
    }

    fn artifacts(&mut self) -> Vec<Artifact> {
        let mut loaded = Vec::new();
        let mut ids: BTreeMap<ArtifactId, String> = BTreeMap::new();
        let mut slugs: BTreeMap<Slug, String> = BTreeMap::new();

        for candidate in self.list(&self.layout.artifacts_dir()) {
            let Some(text) = self.read_text(&candidate.path) else {
                continue;
            };
            let parsed = match parse_artifact_file(&text) {
                Ok(parsed) => parsed,
                Err(problem) => {
                    self.issue(problem.kind, &candidate.path, problem.detail);
                    continue;
                }
            };
            if !parsed.unknown_fields.is_empty() {
                self.issue(
                    LoadIssueKind::UnknownField,
                    &candidate.path,
                    format!("unknown field(s): {}", parsed.unknown_fields.join(", ")),
                );
            }
            let artifact = parsed.artifact;
            if candidate.name != format!("{}.toml", artifact.id()) {
                self.issue(
                    LoadIssueKind::FilenameMismatch,
                    &candidate.path,
                    format!("file name does not match its id {}", artifact.id()),
                );
            }
            if let Some(first) = ids.get(artifact.id()) {
                self.issue(
                    LoadIssueKind::DuplicateArtifactId,
                    &candidate.path,
                    format!("id {} is already defined in {first}", artifact.id()),
                );
                continue;
            }
            if let Some(first) = slugs.get(artifact.slug()) {
                self.issue(
                    LoadIssueKind::DuplicateSlug,
                    &candidate.path,
                    format!("slug `{}` is already used in {first}", artifact.slug()),
                );
                continue;
            }
            self.check_content(&candidate.path, &artifact);
            ids.insert(*artifact.id(), candidate.name.clone());
            slugs.insert(artifact.slug().clone(), candidate.name);
            loaded.push(artifact);
        }
        loaded
    }

    fn relationships(&mut self) -> Vec<functor_model::Relationship> {
        let mut loaded = Vec::new();
        let mut ids: BTreeMap<RelationshipId, String> = BTreeMap::new();

        for candidate in self.list(&self.layout.relationships_dir()) {
            let Some(text) = self.read_text(&candidate.path) else {
                continue;
            };
            let parsed = match parse_relationship_file(&text) {
                Ok(parsed) => parsed,
                Err(problem) => {
                    self.issue(problem.kind, &candidate.path, problem.detail);
                    continue;
                }
            };
            if !parsed.unknown_fields.is_empty() {
                self.issue(
                    LoadIssueKind::UnknownField,
                    &candidate.path,
                    format!("unknown field(s): {}", parsed.unknown_fields.join(", ")),
                );
            }
            let relationship = parsed.relationship;
            if candidate.name != format!("{}.toml", relationship.id()) {
                self.issue(
                    LoadIssueKind::FilenameMismatch,
                    &candidate.path,
                    format!("file name does not match its id {}", relationship.id()),
                );
            }
            if let Some(first) = ids.get(relationship.id()) {
                self.issue(
                    LoadIssueKind::DuplicateRelationshipId,
                    &candidate.path,
                    format!("id {} is already defined in {first}", relationship.id()),
                );
                continue;
            }
            ids.insert(*relationship.id(), candidate.name);
            loaded.push(relationship);
        }
        loaded
    }
}

/// Loads the repository rooted at `layout`, collecting every problem instead of stopping.
///
/// # Errors
/// `NotARepository` when the root directory does not exist. Bad entity data is never an error
/// here; it is reported in the result.
pub(crate) fn load_repository(layout: &Layout) -> Result<SnapshotLoad> {
    if !layout.root().is_dir() {
        return Err(FunctorError::NotARepository {
            path: layout.root().to_path_buf(),
        });
    }
    let mut loader = Loader {
        layout,
        issues: Vec::new(),
    };
    let info = loader.info();
    let artifacts = loader.artifacts();
    let relationships = loader.relationships();
    Ok(SnapshotLoad {
        info,
        artifacts,
        relationships,
        issues: loader.issues,
    })
}
