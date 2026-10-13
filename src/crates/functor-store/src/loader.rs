//! The lenient loader: reads every entity file it can and reports every problem it finds.
//!
//! See `docs/storage-design.md`, "Read Protocol". The result feeds `validate` (all issues) and
//! strict snapshots (`SnapshotLoad::into_snapshot`).

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

use functor_core::{FunctorError, LoadIssue, LoadIssueKind, Result, SnapshotLoad};
use functor_model::{Artifact, ArtifactId, Content, RelationshipId, RepositoryInfo, Slug, Subject};
use functor_repository::toml_support::FileResult;
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

/// Reads a file with the size cap. A failure is the issue to report: kind and detail.
fn read_limited(path: &Path) -> std::result::Result<String, (LoadIssueKind, String)> {
    let unreadable = |error: std::io::Error| (LoadIssueKind::Unreadable, error.to_string());
    let file = File::open(path).map_err(unreadable)?;
    let mut bytes = Vec::new();
    // One byte more than the cap tells an oversized file from one that is exactly at the cap.
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err((
            LoadIssueKind::MalformedFile,
            format!("file is larger than {} MiB", MAX_FILE_BYTES / (1024 * 1024)),
        ));
    }
    String::from_utf8(bytes).map_err(|_| {
        (
            LoadIssueKind::MalformedFile,
            "file is not valid UTF-8".to_owned(),
        )
    })
}

/// What reading and parsing one entity file produced.
enum Fetched<P> {
    /// The file could not be read, or is too large or not UTF-8.
    Failed(LoadIssueKind, String),
    /// The file was read; it parsed or did not.
    Parsed(FileResult<P>),
}

/// Files fewer than this are read on the calling thread; spawning would cost more than it saves.
const PARALLEL_THRESHOLD: usize = 64;
/// At most this many threads read files at once.
const MAX_THREADS: usize = 8;

/// Reads and parses every candidate, in parallel for large directories. The result has one entry
/// per candidate, in the candidates' order, so everything downstream stays deterministic.
fn fetch_all<P: Send>(
    candidates: &[Candidate],
    parse: fn(&str) -> FileResult<P>,
) -> Vec<Fetched<P>> {
    let fetch = |candidate: &Candidate| match read_limited(&candidate.path) {
        Ok(text) => Fetched::Parsed(parse(&text)),
        Err((kind, detail)) => Fetched::Failed(kind, detail),
    };
    let threads = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(MAX_THREADS);
    if candidates.len() < PARALLEL_THRESHOLD || threads < 2 {
        return candidates.iter().map(fetch).collect();
    }
    let chunk = candidates.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let workers: Vec<_> = candidates
            .chunks(chunk)
            .map(|part| scope.spawn(|| part.iter().map(&fetch).collect::<Vec<_>>()))
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| {
                // A panic in a parser is a bug; surface it instead of dropping files silently.
                worker
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    })
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

    // An issue about a file whose entity is known.
    fn entity_issue(
        &mut self,
        kind: LoadIssueKind,
        path: &Path,
        detail: impl Into<String>,
        entity: Subject,
    ) {
        let relative = self.layout.relative(path);
        self.issues
            .push(LoadIssue::new(kind, relative, detail).with_entity(entity));
    }

    // An issue about a file that repeats an id or slug first defined in `first`.
    fn duplicate_issue(
        &mut self,
        kind: LoadIssueKind,
        path: &Path,
        first: &Path,
        detail: impl Into<String>,
        entity: Subject,
    ) {
        let relative = self.layout.relative(path);
        let other = Subject::File(self.layout.relative(first));
        self.issues.push(
            LoadIssue::new(kind, relative, detail)
                .with_entity(entity)
                .with_related([other]),
        );
    }

    // Reads a file with the size cap, classifying failures as issues.
    fn read_text(&mut self, path: &Path) -> Option<String> {
        match read_limited(path) {
            Ok(text) => Some(text),
            Err((kind, detail)) => {
                self.issue(kind, path, detail);
                None
            }
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
            // The directory entry already knows whether it is a directory; no extra stat call.
            let is_dir = entry
                .file_type()
                .map_or_else(|_| path.is_dir(), |t| t.is_dir());
            if name.contains(TEMP_MARKER) {
                self.issue(
                    LoadIssueKind::UnexpectedFile,
                    &path,
                    "leftover of an interrupted write",
                );
            } else if is_dir || path.extension().is_none_or(|ext| ext != "toml") {
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
        let entity = Subject::Artifact(*artifact.id());
        let full = self.layout.content_file(path);
        match fs::metadata(&full) {
            Err(error) if error.kind() == ErrorKind::NotFound => self.entity_issue(
                LoadIssueKind::MissingContentFile,
                file,
                format!("content file `{path}` does not exist"),
                entity,
            ),
            Err(error) => self.issue(LoadIssueKind::Unreadable, &full, error.to_string()),
            Ok(metadata) if !metadata.is_file() => self.entity_issue(
                LoadIssueKind::MissingContentFile,
                file,
                format!("content path `{path}` is not a file"),
                entity,
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
                    self.entity_issue(
                        LoadIssueKind::UnsafeContentPath,
                        file,
                        format!("content path `{path}` resolves outside the repository"),
                        entity,
                    );
                }
            }
        }
    }

    fn artifacts(&mut self) -> Vec<Artifact> {
        let mut loaded = Vec::new();
        let mut ids: BTreeMap<ArtifactId, PathBuf> = BTreeMap::new();
        let mut slugs: BTreeMap<Slug, PathBuf> = BTreeMap::new();

        let candidates = self.list(&self.layout.artifacts_dir());
        let fetched = fetch_all(&candidates, parse_artifact_file);
        for (candidate, fetched) in candidates.into_iter().zip(fetched) {
            let parsed = match fetched {
                Fetched::Parsed(Ok(parsed)) => parsed,
                Fetched::Parsed(Err(problem)) => {
                    self.issue(problem.kind, &candidate.path, problem.detail);
                    continue;
                }
                Fetched::Failed(kind, detail) => {
                    self.issue(kind, &candidate.path, detail);
                    continue;
                }
            };
            let artifact = parsed.artifact;
            let entity = Subject::Artifact(*artifact.id());
            if !parsed.unknown_fields.is_empty() {
                self.entity_issue(
                    LoadIssueKind::UnknownField,
                    &candidate.path,
                    format!("unknown field(s): {}", parsed.unknown_fields.join(", ")),
                    entity.clone(),
                );
            }
            if candidate.name != format!("{}.toml", artifact.id()) {
                self.entity_issue(
                    LoadIssueKind::FilenameMismatch,
                    &candidate.path,
                    format!("file name does not match its id {}", artifact.id()),
                    entity.clone(),
                );
            }
            if let Some(first) = ids.get(artifact.id()) {
                let detail = format!(
                    "id {} is already defined in {}",
                    artifact.id(),
                    file_name(first)
                );
                let first = first.clone();
                self.duplicate_issue(
                    LoadIssueKind::DuplicateArtifactId,
                    &candidate.path,
                    &first,
                    detail,
                    entity,
                );
                continue;
            }
            if let Some(first) = slugs.get(artifact.slug()) {
                let detail = format!(
                    "slug `{}` is already used in {}",
                    artifact.slug(),
                    file_name(first)
                );
                let first = first.clone();
                self.duplicate_issue(
                    LoadIssueKind::DuplicateSlug,
                    &candidate.path,
                    &first,
                    detail,
                    entity,
                );
                continue;
            }
            self.check_content(&candidate.path, &artifact);
            ids.insert(*artifact.id(), candidate.path.clone());
            slugs.insert(artifact.slug().clone(), candidate.path);
            loaded.push(artifact);
        }
        loaded
    }

    fn relationships(&mut self) -> Vec<functor_model::Relationship> {
        let mut loaded = Vec::new();
        let mut ids: BTreeMap<RelationshipId, PathBuf> = BTreeMap::new();

        let candidates = self.list(&self.layout.relationships_dir());
        let fetched = fetch_all(&candidates, parse_relationship_file);
        for (candidate, fetched) in candidates.into_iter().zip(fetched) {
            let parsed = match fetched {
                Fetched::Parsed(Ok(parsed)) => parsed,
                Fetched::Parsed(Err(problem)) => {
                    self.issue(problem.kind, &candidate.path, problem.detail);
                    continue;
                }
                Fetched::Failed(kind, detail) => {
                    self.issue(kind, &candidate.path, detail);
                    continue;
                }
            };
            let relationship = parsed.relationship;
            let entity = Subject::Relationship(*relationship.id());
            if !parsed.unknown_fields.is_empty() {
                self.entity_issue(
                    LoadIssueKind::UnknownField,
                    &candidate.path,
                    format!("unknown field(s): {}", parsed.unknown_fields.join(", ")),
                    entity.clone(),
                );
            }
            if candidate.name != format!("{}.toml", relationship.id()) {
                self.entity_issue(
                    LoadIssueKind::FilenameMismatch,
                    &candidate.path,
                    format!("file name does not match its id {}", relationship.id()),
                    entity.clone(),
                );
            }
            if let Some(first) = ids.get(relationship.id()) {
                let detail = format!(
                    "id {} is already defined in {}",
                    relationship.id(),
                    file_name(first)
                );
                let first = first.clone();
                self.duplicate_issue(
                    LoadIssueKind::DuplicateRelationshipId,
                    &candidate.path,
                    &first,
                    detail,
                    entity,
                );
                continue;
            }
            ids.insert(*relationship.id(), candidate.path);
            loaded.push(relationship);
        }
        loaded
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
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
