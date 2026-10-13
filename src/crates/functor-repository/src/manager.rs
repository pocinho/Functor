//! Creating, opening and discovering repositories on the local filesystem.
//!
//! See `docs/repository-lifecycle.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use functor_core::{
    Clock, FunctorError, IdGenerator, InitOptions, RepositoryHandle, RepositoryManager, Result,
};
use functor_model::{EntityKind, FormatVersion, RepositoryInfo, RepositoryName};

use crate::fsutil::atomic_write;
use crate::layout::{FUNCTOR_DIR, GITKEEP, Layout};
use crate::repository_file::{
    ParsedRepositoryFile, RepositoryFileError, parse_repository_file, render_repository_file,
};

const DEFAULT_NAME: &str = "repository";

fn absolute(path: &Path) -> Result<PathBuf> {
    std::path::absolute(path).map_err(|error| FunctorError::storage("resolve path", path, error))
}

/// Reads and checks `.functor/repository.toml` of the repository rooted at `layout`.
///
/// # Errors
/// `NotARepository` when the file is missing, `UnsupportedFormat` for another format version,
/// `Inconsistent` when the file is malformed, `Storage` when it cannot be read.
pub fn read_repository_file(layout: &Layout) -> Result<ParsedRepositoryFile> {
    let path = layout.repository_file();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(FunctorError::NotARepository {
                path: layout.root().to_path_buf(),
            });
        }
        Err(error) => return Err(FunctorError::storage("read", path, error)),
    };
    parse_repository_file(&text).map_err(|error| match error {
        RepositoryFileError::UnsupportedFormat { found } => FunctorError::UnsupportedFormat {
            found,
            supported: FormatVersion::CURRENT.get(),
        },
        RepositoryFileError::Problem(problem) => FunctorError::Inconsistent {
            detail: format!(".functor/repository.toml: {}", problem.detail),
        },
    })
}

/// Opens the repository rooted exactly at `root`.
///
/// # Errors
/// See [`read_repository_file`].
pub fn open_repository(root: &Path) -> Result<RepositoryHandle> {
    let root = absolute(root)?;
    let parsed = read_repository_file(&Layout::new(&root))?;
    Ok(RepositoryHandle {
        root,
        info: parsed.info,
    })
}

fn is_repository_root(dir: &Path) -> bool {
    Layout::new(dir).repository_file().is_file()
}

/// Finds the root of the repository containing `start` by walking up the directory tree, without
/// reading `repository.toml`. The nearest repository wins.
///
/// `validate` uses this so that a malformed or unsupported `repository.toml` is reported as a
/// finding instead of stopping the command before it starts.
///
/// # Errors
/// `NotARepository` when no ancestor contains `.functor/repository.toml`.
pub fn discover_root(start: &Path) -> Result<PathBuf> {
    let start_abs = absolute(start)?;
    let first = if start_abs.is_file() {
        start_abs.parent().map(Path::to_path_buf)
    } else {
        Some(start_abs.clone())
    };
    first
        .iter()
        .flat_map(|dir| dir.ancestors())
        .find(|dir| is_repository_root(dir))
        .map(Path::to_path_buf)
        .ok_or(FunctorError::NotARepository { path: start_abs })
}

/// Finds the repository containing `start` by walking up the directory tree. The nearest
/// repository wins, so nested repositories shadow their parents.
///
/// # Errors
/// `NotARepository` when no ancestor is a repository, otherwise as [`open_repository`].
pub fn discover_repository(start: &Path) -> Result<RepositoryHandle> {
    open_repository(&discover_root(start)?)
}

/// The repository manager for the local filesystem.
///
/// The async trait methods run the blocking filesystem work on tokio's blocking pool; the
/// `*_blocking` methods are for synchronous callers such as test factories.
#[derive(Clone)]
pub struct FileRepositoryManager {
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl std::fmt::Debug for FileRepositoryManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileRepositoryManager")
            .finish_non_exhaustive()
    }
}

impl FileRepositoryManager {
    /// Creates a manager that takes time from `clock` and new identifiers from `ids`.
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>, ids: Arc<dyn IdGenerator>) -> Self {
        Self { clock, ids }
    }

    /// Creates a repository at `at` (blocking).
    ///
    /// The steps are those of `docs/repository-lifecycle.md`; `repository.toml` is written last
    /// and atomically, so a partially created directory is never recognized as a repository.
    ///
    /// # Errors
    /// `Conflict` if `at` is already a repository, or an ancestor is and `options.nested` is not
    /// set; `InvalidInput` for an invalid name; `Storage` for filesystem failures (after which
    /// directories created by this call are removed on a best-effort basis).
    pub fn init_blocking(&self, at: &Path, options: &InitOptions) -> Result<RepositoryHandle> {
        let root = absolute(at)?;
        let layout = Layout::new(&root);

        if layout.functor_dir().exists() {
            return Err(FunctorError::conflict(
                EntityKind::Repository,
                format!(
                    "{} already contains a `{FUNCTOR_DIR}` directory",
                    root.display()
                ),
            ));
        }
        if !options.nested
            && let Some(parent) = root.ancestors().skip(1).find(|dir| is_repository_root(dir))
        {
            return Err(FunctorError::conflict(
                EntityKind::Repository,
                format!(
                    "{} is inside the repository at {}; use --nested to create a nested one",
                    root.display(),
                    parent.display()
                ),
            ));
        }

        let name = match &options.name {
            Some(name) => RepositoryName::new(name)?,
            None => default_name(&root),
        };
        let info = RepositoryInfo::new(
            self.ids.repository_id(),
            name,
            options.description.clone(),
            FormatVersion::CURRENT,
            self.clock.now(),
        );

        let root_existed = root.exists();
        match create_structure(&layout, &info) {
            Ok(()) => Ok(RepositoryHandle { root, info }),
            Err(error) => {
                remove_structure(&layout, root_existed);
                Err(error)
            }
        }
    }

    /// Opens the repository rooted at `root` (blocking). See [`open_repository`].
    ///
    /// # Errors
    /// See [`open_repository`].
    #[allow(clippy::unused_self)]
    pub fn open_blocking(&self, root: &Path) -> Result<RepositoryHandle> {
        open_repository(root)
    }

    /// Discovers the repository containing `start` (blocking). See [`discover_repository`].
    ///
    /// # Errors
    /// See [`discover_repository`].
    #[allow(clippy::unused_self)]
    pub fn discover_blocking(&self, start: &Path) -> Result<RepositoryHandle> {
        discover_repository(start)
    }
}

fn default_name(root: &Path) -> RepositoryName {
    root.file_name()
        .and_then(|name| RepositoryName::new(&name.to_string_lossy()).ok())
        .or_else(|| RepositoryName::new(DEFAULT_NAME).ok())
        .unwrap_or_else(|| unreachable!("the default repository name is valid"))
}

fn create_structure(layout: &Layout, info: &RepositoryInfo) -> Result<()> {
    let mkdir = |path: PathBuf| {
        fs::create_dir_all(&path)
            .map_err(|error| FunctorError::storage("create directory", path, error))
    };
    mkdir(layout.functor_dir())?;
    for dir in [
        layout.artifacts_dir(),
        layout.relationships_dir(),
        layout.events_dir(),
    ] {
        mkdir(dir.clone())?;
        let keep = dir.join(GITKEEP);
        fs::write(&keep, b"").map_err(|error| FunctorError::storage("write", keep, error))?;
    }
    // The commit point: only now does the directory become a repository.
    atomic_write(
        &layout.repository_file(),
        render_repository_file(info).as_bytes(),
    )
}

// Best effort: removes only what `create_structure` makes, so existing content is never touched.
fn remove_structure(layout: &Layout, root_existed: bool) {
    for dir in [
        layout.artifacts_dir(),
        layout.relationships_dir(),
        layout.events_dir(),
    ] {
        let _ = fs::remove_file(dir.join(GITKEEP));
        let _ = fs::remove_dir(&dir);
    }
    let _ = fs::remove_dir_all(layout.functor_dir());
    if !root_existed {
        let _ = fs::remove_dir(layout.root());
    }
}

use crate::fsutil::run_blocking as blocking;

#[async_trait]
impl RepositoryManager for FileRepositoryManager {
    async fn init(&self, at: &Path, options: InitOptions) -> Result<RepositoryHandle> {
        let manager = self.clone();
        let at = at.to_path_buf();
        blocking(move || manager.init_blocking(&at, &options)).await
    }

    async fn open(&self, root: &Path) -> Result<RepositoryHandle> {
        let root = root.to_path_buf();
        blocking(move || open_repository(&root)).await
    }

    async fn discover(&self, start: &Path) -> Result<RepositoryHandle> {
        let start = start.to_path_buf();
        blocking(move || discover_repository(&start)).await
    }
}

#[cfg(test)]
mod tests {
    use functor_core::ErrorKind;
    use functor_testkit::{Env, FixedClock, SequentialIds};

    use super::*;

    fn manager(env: &Env) -> FileRepositoryManager {
        FileRepositoryManager::new(env.clock(), env.ids())
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut found: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        found
    }

    #[test]
    fn init_creates_the_documented_layout() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("demo");
        let env = Env::new();
        let handle = manager(&env)
            .init_blocking(&target, &InitOptions::new().with_description("d"))
            .unwrap();

        assert_eq!(
            handle.info.name().as_str(),
            "demo",
            "name defaults to the directory name"
        );
        assert_eq!(handle.info.description(), Some("d"));
        assert_eq!(handle.info.format_version(), FormatVersion::CURRENT);
        assert_eq!(handle.info.created(), env.now());
        assert_eq!(
            names(&target),
            [".functor", "artifacts", "events", "relationships"]
        );
        assert_eq!(names(&target.join(".functor")), ["repository.toml"]);
        for dir in ["artifacts", "events", "relationships"] {
            assert_eq!(names(&target.join(dir)), [".gitkeep"], "{dir}");
        }
        let written = fs::read_to_string(target.join(".functor/repository.toml")).unwrap();
        assert_eq!(written, render_repository_file(&handle.info));
    }

    #[test]
    fn init_creates_missing_target_directories_and_honours_the_name() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("a").join("b");
        let handle = manager(&Env::new())
            .init_blocking(&target, &InitOptions::new().with_name("  My Repo "))
            .unwrap();
        assert_eq!(handle.info.name().as_str(), "My Repo");
        assert!(target.join(".functor/repository.toml").is_file());
    }

    #[test]
    fn init_twice_is_a_conflict_and_changes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let env = Env::new();
        let first = manager(&env)
            .init_blocking(temp.path(), &InitOptions::new())
            .unwrap();
        let before = fs::read(temp.path().join(".functor/repository.toml")).unwrap();

        let error = manager(&env)
            .init_blocking(temp.path(), &InitOptions::new())
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Conflict);
        assert_eq!(
            fs::read(temp.path().join(".functor/repository.toml")).unwrap(),
            before
        );
        assert_eq!(
            open_repository(temp.path()).unwrap().info.id(),
            first.info.id()
        );
    }

    #[test]
    fn nested_repositories_need_the_nested_option() {
        let temp = tempfile::tempdir().unwrap();
        let env = Env::new();
        manager(&env)
            .init_blocking(temp.path(), &InitOptions::new())
            .unwrap();
        let inner = temp.path().join("inner");

        let error = manager(&env)
            .init_blocking(&inner, &InitOptions::new())
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Conflict);
        assert!(error.to_string().contains("--nested"));
        assert!(!inner.exists(), "nothing was created");

        manager(&env)
            .init_blocking(&inner, &InitOptions::new().nested())
            .unwrap();
        assert!(inner.join(".functor/repository.toml").is_file());
    }

    #[test]
    fn invalid_names_are_rejected_before_anything_is_created() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("x");
        let error = manager(&Env::new())
            .init_blocking(&target, &InitOptions::new().with_name("   "))
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidInput);
        assert!(!target.exists());
    }

    #[test]
    fn a_failed_init_leaves_no_recognizable_repository_and_can_be_retried() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("demo");
        fs::create_dir_all(&target).unwrap();
        // A file where the `events` directory must go makes the structure step fail midway.
        fs::write(target.join("events"), b"in the way").unwrap();
        let env = Env::new();

        let error = manager(&env)
            .init_blocking(&target, &InitOptions::new())
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Storage);
        assert!(
            !target.join(".functor").exists(),
            "the partial structure was removed"
        );
        assert!(!target.join("artifacts").exists());
        assert!(!target.join("relationships").exists());
        assert!(
            target.join("events").is_file(),
            "pre-existing content is untouched"
        );
        assert!(matches!(
            discover_repository(&target).unwrap_err().kind(),
            ErrorKind::NotARepository
        ));

        fs::remove_file(target.join("events")).unwrap();
        assert!(
            manager(&env)
                .init_blocking(&target, &InitOptions::new())
                .is_ok()
        );
    }

    #[test]
    fn init_under_a_file_is_a_storage_error() {
        let temp = tempfile::tempdir().unwrap();
        // The parent is a file, so creating the target fails.
        let blocker = temp.path().join("blocker");
        fs::write(&blocker, b"file").unwrap();
        let error = manager(&Env::new())
            .init_blocking(&blocker.join("repo"), &InitOptions::new())
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Storage);
    }

    #[test]
    fn open_requires_the_exact_root() {
        let temp = tempfile::tempdir().unwrap();
        let env = Env::new();
        let handle = manager(&env)
            .init_blocking(temp.path(), &InitOptions::new())
            .unwrap();
        let sub = temp.path().join("artifacts");

        assert_eq!(open_repository(temp.path()).unwrap().info, handle.info);
        assert_eq!(
            open_repository(&sub).unwrap_err().kind(),
            ErrorKind::NotARepository
        );
    }

    #[test]
    fn discover_walks_up_and_the_nearest_repository_wins() {
        let temp = tempfile::tempdir().unwrap();
        let env = Env::new();
        let outer = manager(&env)
            .init_blocking(temp.path(), &InitOptions::new())
            .unwrap();
        let deep = temp.path().join("x").join("y");
        fs::create_dir_all(&deep).unwrap();

        let found = discover_repository(&deep).unwrap();
        assert_eq!(found.info.id(), outer.info.id());
        assert_eq!(found.root, std::path::absolute(temp.path()).unwrap());

        let inner_root = temp.path().join("inner");
        let inner = manager(&env)
            .init_blocking(&inner_root, &InitOptions::new().nested())
            .unwrap();
        let found = discover_repository(&inner_root.join("artifacts")).unwrap();
        assert_eq!(
            found.info.id(),
            inner.info.id(),
            "nested repositories shadow their parents"
        );
    }

    #[test]
    fn discover_outside_any_repository_is_not_a_repository() {
        let temp = tempfile::tempdir().unwrap();
        let error = discover_repository(temp.path()).unwrap_err();
        assert!(matches!(error, FunctorError::NotARepository { .. }));
    }

    #[test]
    fn discover_root_finds_the_repository_even_when_repository_toml_is_broken() {
        let temp = tempfile::tempdir().unwrap();
        manager(&Env::new())
            .init_blocking(temp.path(), &InitOptions::new())
            .unwrap();
        fs::write(
            temp.path().join(".functor/repository.toml"),
            "format_version = 9\n",
        )
        .unwrap();
        let nested = temp.path().join("artifacts");

        assert_eq!(
            discover_root(&nested).unwrap(),
            std::path::absolute(temp.path()).unwrap()
        );
        // The checking discovery still refuses it.
        assert!(matches!(
            discover_repository(&nested).unwrap_err(),
            FunctorError::UnsupportedFormat { .. }
        ));

        let outside = tempfile::tempdir().unwrap();
        assert!(matches!(
            discover_root(outside.path()).unwrap_err(),
            FunctorError::NotARepository { .. }
        ));
    }

    #[test]
    fn discover_accepts_a_file_path() {
        let temp = tempfile::tempdir().unwrap();
        manager(&Env::new())
            .init_blocking(temp.path(), &InitOptions::new())
            .unwrap();
        let file = temp.path().join("notes.txt");
        fs::write(&file, b"x").unwrap();
        assert!(discover_repository(&file).is_ok());
    }

    #[test]
    fn unsupported_and_malformed_repository_files_are_reported() {
        let temp = tempfile::tempdir().unwrap();
        manager(&Env::new())
            .init_blocking(temp.path(), &InitOptions::new())
            .unwrap();
        let file = temp.path().join(".functor/repository.toml");

        fs::write(&file, "format_version = 2\n").unwrap();
        assert!(matches!(
            open_repository(temp.path()).unwrap_err(),
            FunctorError::UnsupportedFormat {
                found: 2,
                supported: 1
            }
        ));

        fs::write(&file, "this is = = not toml").unwrap();
        let error = open_repository(temp.path()).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Inconsistent);
        assert!(error.to_string().contains("repository.toml"));
    }

    #[test]
    fn the_repository_id_is_stable_across_opens() {
        let temp = tempfile::tempdir().unwrap();
        let created = manager(&Env::new())
            .init_blocking(temp.path(), &InitOptions::new())
            .unwrap();
        for _ in 0..3 {
            assert_eq!(
                open_repository(temp.path()).unwrap().info.id(),
                created.info.id()
            );
        }
    }

    #[test]
    fn the_manager_uses_the_injected_clock_and_ids() {
        let temp = tempfile::tempdir().unwrap();
        let clock = Arc::new(FixedClock::new(
            functor_model::Timestamp::parse("2030-01-02T03:04:05Z").unwrap(),
        ));
        let manager = FileRepositoryManager::new(clock, Arc::new(SequentialIds::starting_at(77)));
        let handle = manager
            .init_blocking(temp.path(), &InitOptions::new())
            .unwrap();
        assert_eq!(handle.info.created().to_string(), "2030-01-02T03:04:05Z");
        assert_eq!(
            *handle.info.id(),
            functor_model::RepositoryId::from_bits(77)
        );
    }

    #[tokio::test]
    async fn the_async_trait_methods_work_through_a_trait_object() {
        let temp = tempfile::tempdir().unwrap();
        let env = Env::new();
        let manager: Box<dyn RepositoryManager> = Box::new(self::manager(&env));

        let created = manager.init(temp.path(), InitOptions::new()).await.unwrap();
        let opened = manager.open(temp.path()).await.unwrap();
        let sub = temp.path().join("artifacts");
        let discovered = manager.discover(&sub).await.unwrap();
        assert_eq!(created.info, opened.info);
        assert_eq!(created.info, discovered.info);
        let again = manager
            .init(temp.path(), InitOptions::new())
            .await
            .unwrap_err();
        assert_eq!(again.kind(), ErrorKind::Conflict);
    }
}
