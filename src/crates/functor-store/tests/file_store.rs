//! `FileStore` behavior on a real filesystem: the shared conformance suite, the on-disk format,
//! in-place edits, damaged repositories, concurrency and portability.

// Test support code: panicking on unexpected errors is the point.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stderr,
    clippy::unused_self
)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use functor_core::{
    ArtifactRepository, ErrorKind, InitOptions, LoadIssueKind, RelationshipRepository,
    RepositoryReader,
};
use functor_model::{
    Artifact, ArtifactId, ArtifactPatch, ArtifactType, Content, ContentFormat, NewArtifact,
    NewRelationship, Relationship, RelationshipType, RepoPath, Slug, Subject, Title,
};
use functor_repository::{FileRepositoryManager, Layout, WriteLock};
use functor_store::{
    FileStore, StoreOptions, parse_artifact_file, render_artifact_file, render_relationship_file,
};
use functor_testkit::{Env, Fresh, run_store_conformance};
use tempfile::TempDir;

fn init(env: &Env, options: StoreOptions) -> (TempDir, FileStore) {
    let dir = tempfile::tempdir().unwrap();
    let manager = FileRepositoryManager::new(env.clock(), env.ids());
    let handle = manager
        .init_blocking(dir.path(), &InitOptions::new().with_name("Test"))
        .unwrap();
    let store = FileStore::with_options(&handle, env.clock(), env.ids(), options);
    (dir, store)
}

fn new_artifact(slug: &str) -> NewArtifact {
    NewArtifact::new(
        Slug::parse(slug).unwrap(),
        ArtifactType::document(),
        Title::new(slug).unwrap(),
    )
}

async fn add_artifact(store: &FileStore, slug: &str) -> Artifact {
    ArtifactRepository::create(store, new_artifact(slug))
        .await
        .unwrap()
}

async fn add_dependency(store: &FileStore, source: &Artifact, target: &Artifact) -> Relationship {
    RelationshipRepository::create(
        store,
        NewRelationship::new(RelationshipType::DependsOn, *source.id(), *target.id()),
    )
    .await
    .unwrap()
}

struct Repo {
    dir: TempDir,
    env: Env,
    store: FileStore,
    alpha: Artifact,
    beta: Artifact,
    rel: Relationship,
}

/// A repository with two artifacts and one relationship, like the MVP success path.
async fn repo() -> Repo {
    let env = Env::new();
    let (dir, store) = init(&env, StoreOptions::default());
    let alpha = add_artifact(&store, "alpha").await;
    let beta = add_artifact(&store, "beta").await;
    let rel = add_dependency(&store, &alpha, &beta).await;
    Repo {
        dir,
        env,
        store,
        alpha,
        beta,
        rel,
    }
}

impl Repo {
    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    fn write(&self, relative: &str, contents: impl AsRef<[u8]>) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).unwrap()
    }

    fn artifact_path(&self, artifact: &Artifact) -> String {
        format!("artifacts/{}.toml", artifact.id())
    }

    async fn issues(&self) -> Vec<(LoadIssueKind, String)> {
        RepositoryReader::load(&self.store)
            .await
            .unwrap()
            .issues
            .iter()
            .map(|issue| (issue.kind, issue.path.to_string()))
            .collect()
    }
}

fn artifact_text_with(id: u128, slug: &str, extra: &str) -> String {
    let artifact = Artifact::create(
        ArtifactId::from_bits(id),
        new_artifact(slug),
        functor_model::Timestamp::parse("2026-10-12T18:00:00Z").unwrap(),
    );
    format!("{}{extra}", render_artifact_file(&artifact))
}

// ---------------------------------------------------------------------------------------------
// Shared contract
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn file_store_passes_the_conformance_suite() {
    run_store_conformance(|env: &Env| {
        let (dir, store) = init(env, StoreOptions::default());
        Fresh::with_guard(store, dir)
    })
    .await;
}

// ---------------------------------------------------------------------------------------------
// On-disk format
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn entities_are_stored_as_canonical_files_named_by_id() {
    let repo = repo().await;
    let artifact_file = repo.artifact_path(&repo.alpha);
    assert_eq!(repo.read(&artifact_file), render_artifact_file(&repo.alpha));
    let relationship_file = format!("relationships/{}.toml", repo.rel.id());
    assert_eq!(
        repo.read(&relationship_file),
        render_relationship_file(&repo.rel)
    );

    let text = repo.read(&artifact_file);
    assert!(text.starts_with(&format!("id = \"{}\"\nslug = \"alpha\"\n", repo.alpha.id())));
    assert!(!text.contains('\r'), "new files use LF");
    assert!(text.ends_with('\n') && !text.ends_with("\n\n"));
}

#[tokio::test]
async fn nothing_but_entity_files_is_left_behind_by_writes() {
    let repo = repo().await;
    let mut names: Vec<String> = fs::read_dir(repo.path("artifacts"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            ".gitkeep".to_owned(),
            format!("{}.toml", repo.alpha.id()),
            format!("{}.toml", repo.beta.id())
        ]
    );
}

#[tokio::test]
async fn moving_the_repository_directory_keeps_identities() {
    let env = Env::new();
    let parent = tempfile::tempdir().unwrap();
    let first = parent.path().join("first");
    let manager = FileRepositoryManager::new(env.clock(), env.ids());
    let handle = manager.init_blocking(&first, &InitOptions::new()).unwrap();
    let store = FileStore::new(&handle, env.clock(), env.ids());
    let alpha = add_artifact(&store, "alpha").await;

    let second = parent.path().join("second");
    fs::rename(&first, &second).unwrap();
    let moved = manager.open_blocking(&second).unwrap();
    let moved_store = FileStore::new(&moved, env.clock(), env.ids());

    assert_eq!(
        moved.info, handle.info,
        "the repository id does not depend on its location"
    );
    assert_eq!(
        ArtifactRepository::list(&moved_store).await.unwrap(),
        [alpha]
    );
}

// ---------------------------------------------------------------------------------------------
// In-place edits
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn updates_preserve_comments_formatting_and_unknown_keys() {
    let repo = repo().await;
    let file = repo.artifact_path(&repo.alpha);
    let original = repo.read(&file);
    let hand_edited = format!(
        "# written by hand\n{}",
        original
            .replace("slug = \"alpha\"", "slug   =   \"alpha\"   # the alias")
            .replace("revision = 1\n", "revision = 1\nowner = \"someone\"\n")
    ) + "\n[extra]\nkeep = \"me\"\n";
    repo.write(&file, &hand_edited);
    repo.env.advance(60);

    let patched = ArtifactRepository::update(
        &repo.store,
        repo.alpha.id(),
        ArtifactPatch::new().with_title(Title::new("Alpha Two").unwrap()),
        None,
    )
    .await
    .unwrap();
    assert_eq!(patched.artifact.revision().get(), 2);

    let edited = repo.read(&file);
    assert!(edited.starts_with("# written by hand\n"));
    assert!(edited.contains("slug   =   \"alpha\"   # the alias\n"));
    assert!(edited.contains("owner = \"someone\"\n"));
    assert!(edited.contains("\n[extra]\nkeep = \"me\"\n"));
    assert!(edited.contains("title = \"Alpha Two\"\n"));
    assert!(edited.contains("revision = 2\n"));
    let loaded = parse_artifact_file(&edited).unwrap();
    assert_eq!(loaded.artifact, patched.artifact);
    assert_eq!(loaded.unknown_fields, ["owner", "extra"]);

    // Hand edits survive the strict load too: unknown fields are non-fatal.
    assert!(RepositoryReader::snapshot(&repo.store).await.is_ok());
}

#[tokio::test]
async fn a_no_op_update_does_not_touch_the_file() {
    let repo = repo().await;
    let file = repo.path(&repo.artifact_path(&repo.alpha));
    let before_text = fs::read(&file).unwrap();
    let before_time = fs::metadata(&file).unwrap().modified().unwrap();
    thread::sleep(Duration::from_millis(50));
    repo.env.advance(60);

    let patched = ArtifactRepository::update(
        &repo.store,
        repo.alpha.id(),
        ArtifactPatch::new().with_title(repo.alpha.title().clone()),
        None,
    )
    .await
    .unwrap();

    assert_eq!(patched.changed, []);
    assert_eq!(fs::read(&file).unwrap(), before_text);
    assert_eq!(
        fs::metadata(&file).unwrap().modified().unwrap(),
        before_time
    );
}

#[tokio::test]
async fn crlf_checkouts_load_and_keep_their_line_endings_when_updated() {
    let repo = repo().await;
    for entry in ["artifacts", "relationships", ".functor"] {
        for file in fs::read_dir(repo.path(entry)).unwrap() {
            let path = file.unwrap().path();
            if path.extension().is_some_and(|e| e == "toml") {
                let text = fs::read_to_string(&path).unwrap();
                fs::write(&path, text.replace('\n', "\r\n")).unwrap();
            }
        }
    }
    let snapshot = RepositoryReader::snapshot(&repo.store).await.unwrap();
    assert_eq!(snapshot.artifacts().len(), 2);
    assert_eq!(snapshot.relationships().len(), 1);

    repo.env.advance(1);
    ArtifactRepository::update(
        &repo.store,
        repo.alpha.id(),
        ArtifactPatch::new().with_description(Some("added".into())),
        None,
    )
    .await
    .unwrap();
    let edited = repo.read(&repo.artifact_path(&repo.alpha));
    assert!(edited.contains("description = \"added\"\r\n"));
    assert_eq!(
        edited.replace("\r\n", "").matches('\n').count(),
        0,
        "no bare LF"
    );
}

// ---------------------------------------------------------------------------------------------
// Damaged repositories: every kind of problem is reported, none aborts the load
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_healthy_repository_has_no_issues() {
    let repo = repo().await;
    assert_eq!(repo.issues().await, []);
}

#[tokio::test]
async fn stray_temporary_files_are_ignored_by_strict_loads_and_reported_by_lenient_ones() {
    let repo = repo().await;
    repo.write("artifacts/art_X.toml.tmp-1-1", "partial");
    assert_eq!(
        repo.issues().await,
        [(
            LoadIssueKind::UnexpectedFile,
            "artifacts/art_X.toml.tmp-1-1".to_owned()
        )]
    );
    let snapshot = RepositoryReader::snapshot(&repo.store).await.unwrap();
    assert_eq!(snapshot.artifacts().len(), 2);
}

#[tokio::test]
async fn unexpected_files_and_directories_are_reported_but_not_fatal() {
    let repo = repo().await;
    repo.write("artifacts/notes.txt", "hi");
    fs::create_dir(repo.path("relationships/sub")).unwrap();
    assert_eq!(
        repo.issues().await,
        [
            (
                LoadIssueKind::UnexpectedFile,
                "artifacts/notes.txt".to_owned()
            ),
            (
                LoadIssueKind::UnexpectedFile,
                "relationships/sub".to_owned()
            ),
        ]
    );
    assert!(RepositoryReader::snapshot(&repo.store).await.is_ok());
}

#[tokio::test]
async fn malformed_files_are_reported_with_their_path() {
    let repo = repo().await;
    repo.write("artifacts/bad.toml", "= = =");
    repo.write("relationships/bad.toml", "= = =");
    assert_eq!(
        repo.issues().await,
        [
            (
                LoadIssueKind::MalformedFile,
                "artifacts/bad.toml".to_owned()
            ),
            (
                LoadIssueKind::MalformedFile,
                "relationships/bad.toml".to_owned()
            ),
        ]
    );
    let load = RepositoryReader::load(&repo.store).await.unwrap();
    assert_eq!(load.artifacts.len(), 2, "good entities still load");
    assert!(load.has_fatal_issues());
}

#[tokio::test]
async fn missing_and_invalid_identifiers_are_classified() {
    let repo = repo().await;
    let good = repo.read(&repo.artifact_path(&repo.alpha));
    repo.write(
        "artifacts/noid.toml",
        good.replace(&format!("id = \"{}\"\n", repo.alpha.id()), ""),
    );
    repo.write(
        "artifacts/badid.toml",
        good.replace(&repo.alpha.id().to_string(), "art_nope"),
    );
    repo.write(
        "artifacts/badslug.toml",
        good.replace("slug = \"alpha\"", "slug = \"Not A Slug\""),
    );
    assert_eq!(
        repo.issues().await,
        [
            (
                LoadIssueKind::InvalidIdentifier,
                "artifacts/badid.toml".to_owned()
            ),
            (
                LoadIssueKind::InvalidIdentifier,
                "artifacts/badslug.toml".to_owned()
            ),
            (
                LoadIssueKind::MissingIdentifier,
                "artifacts/noid.toml".to_owned()
            ),
        ]
    );
}

#[tokio::test]
async fn a_file_whose_name_does_not_match_its_id_is_reported() {
    let repo = repo().await;
    let from = repo.path(&repo.artifact_path(&repo.beta));
    fs::rename(&from, repo.path("artifacts/renamed.toml")).unwrap();
    assert_eq!(
        repo.issues().await,
        [(
            LoadIssueKind::FilenameMismatch,
            "artifacts/renamed.toml".to_owned()
        )]
    );
}

#[tokio::test]
async fn duplicate_ids_keep_the_first_file_and_report_the_second() {
    let repo = repo().await;
    let text = repo.read(&repo.artifact_path(&repo.alpha));
    repo.write("artifacts/zzz-copy.toml", &text);
    assert_eq!(
        repo.issues().await,
        [
            (
                LoadIssueKind::FilenameMismatch,
                "artifacts/zzz-copy.toml".to_owned()
            ),
            (
                LoadIssueKind::DuplicateArtifactId,
                "artifacts/zzz-copy.toml".to_owned()
            ),
        ]
    );
    let load = RepositoryReader::load(&repo.store).await.unwrap();
    assert_eq!(
        load.artifacts.len(),
        2,
        "the duplicate is excluded, so the load stays consistent"
    );
    let error = RepositoryReader::snapshot(&repo.store).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Inconsistent);
    assert!(error.to_string().contains("functor validate"));
}

#[tokio::test]
async fn duplicate_slugs_keep_the_first_file_and_report_the_second() {
    let repo = repo().await;
    let id = ArtifactId::from_bits(9999);
    repo.write(
        &format!("artifacts/{id}.toml"),
        artifact_text_with(9999, "alpha", ""),
    );
    assert_eq!(
        repo.issues().await,
        [(LoadIssueKind::DuplicateSlug, format!("artifacts/{id}.toml"))]
    );
}

#[tokio::test]
async fn issues_name_their_entity_and_the_file_that_defined_a_duplicate_first() {
    let repo = repo().await;
    // A slug clash: the first file (alpha's own) is named as related.
    let clash = ArtifactId::from_bits(9999);
    repo.write(
        &format!("artifacts/{clash}.toml"),
        artifact_text_with(9999, "alpha", ""),
    );
    // A renamed file keeps its entity.
    fs::rename(
        repo.path(&repo.artifact_path(&repo.beta)),
        repo.path("artifacts/renamed.toml"),
    )
    .unwrap();

    let issues = RepositoryReader::load(&repo.store).await.unwrap().issues;
    let slug_clash = issues
        .iter()
        .find(|i| i.kind == LoadIssueKind::DuplicateSlug)
        .unwrap();
    assert_eq!(slug_clash.entity, Some(Subject::Artifact(clash)));
    assert_eq!(
        slug_clash.related,
        [Subject::File(
            RepoPath::new(&repo.artifact_path(&repo.alpha)).unwrap()
        )]
    );
    let renamed = issues
        .iter()
        .find(|i| i.kind == LoadIssueKind::FilenameMismatch)
        .unwrap();
    assert_eq!(renamed.entity, Some(Subject::Artifact(*repo.beta.id())));
    assert_eq!(renamed.related, []);
}

#[tokio::test]
async fn relationship_problems_are_classified() {
    let repo = repo().await;
    let good = repo.read(&format!("relationships/{}.toml", repo.rel.id()));

    let self_ref = good.replace(&repo.beta.id().to_string(), &repo.alpha.id().to_string());
    repo.write("relationships/rel_self.toml", &self_ref);
    repo.write("relationships/rel_copy.toml", &good);
    // File names sort as `rel_<ULID>.toml` (the real file), `rel_copy.toml`, `rel_self.toml`:
    // the copy is a filename mismatch and then a duplicate of the real file's id; the
    // self-referencing file cannot be built, so it is reported before any duplicate check.
    assert_eq!(
        repo.issues().await,
        [
            (
                LoadIssueKind::FilenameMismatch,
                "relationships/rel_copy.toml".to_owned()
            ),
            (
                LoadIssueKind::DuplicateRelationshipId,
                "relationships/rel_copy.toml".to_owned()
            ),
            (
                LoadIssueKind::SelfReference,
                "relationships/rel_self.toml".to_owned()
            ),
        ]
    );
}

#[tokio::test]
async fn dangling_relationships_make_the_strict_load_fail_but_the_lenient_one_report_nothing() {
    let repo = repo().await;
    fs::remove_file(repo.path(&repo.artifact_path(&repo.beta))).unwrap();
    // Dangling endpoints are found by validation (V0006/V0007) from the loaded entities.
    assert_eq!(repo.issues().await, []);
    let load = RepositoryReader::load(&repo.store).await.unwrap();
    assert_eq!(load.relationships.len(), 1);
    assert_eq!(load.artifacts.len(), 1);
    let error = RepositoryReader::snapshot(&repo.store).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Inconsistent);
}

#[tokio::test]
async fn content_paths_are_checked_for_safety_and_existence() {
    let repo = repo().await;
    let content = |path: &str| format!("\n[content]\nformat = \"markdown\"\npath = \"{path}\"\n");
    repo.write(
        &format!("artifacts/{}.toml", ArtifactId::from_bits(101)),
        artifact_text_with(101, "escapes", &content("../outside.md")),
    );
    repo.write(
        &format!("artifacts/{}.toml", ArtifactId::from_bits(102)),
        artifact_text_with(102, "missing", &content("content/missing.md")),
    );
    repo.write(
        &format!("artifacts/{}.toml", ArtifactId::from_bits(103)),
        artifact_text_with(103, "present", &content("content/present.md")),
    );
    repo.write("content/present.md", "# hello");
    repo.write(
        &format!("artifacts/{}.toml", ArtifactId::from_bits(104)),
        artifact_text_with(104, "directory", &content("content/dir")),
    );
    fs::create_dir_all(repo.path("content/dir")).unwrap();

    assert_eq!(
        repo.issues().await,
        [
            (
                LoadIssueKind::UnsafeContentPath,
                format!("artifacts/{}.toml", ArtifactId::from_bits(101))
            ),
            (
                LoadIssueKind::MissingContentFile,
                format!("artifacts/{}.toml", ArtifactId::from_bits(102))
            ),
            (
                LoadIssueKind::MissingContentFile,
                format!("artifacts/{}.toml", ArtifactId::from_bits(104))
            ),
        ]
    );
    let load = RepositoryReader::load(&repo.store).await.unwrap();
    let slugs: Vec<&str> = load.artifacts.iter().map(|a| a.slug().as_str()).collect();
    assert!(slugs.contains(&"missing") && slugs.contains(&"present"));
    assert!(
        !slugs.contains(&"escapes"),
        "an unsafe path cannot be represented, so the entity is excluded"
    );
}

#[tokio::test]
async fn a_link_inside_the_repository_cannot_lead_outside_it() {
    let repo = repo().await;
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.md"), "secret").unwrap();
    let link = repo.path("content/link.md");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    #[cfg(unix)]
    let linked = std::os::unix::fs::symlink(outside.path().join("secret.md"), &link).is_ok();
    #[cfg(windows)]
    let linked =
        std::os::windows::fs::symlink_file(outside.path().join("secret.md"), &link).is_ok();
    if !linked {
        eprintln!("skipped: symbolic links cannot be created in this environment");
        return;
    }
    let id = ArtifactId::from_bits(105);
    let text = artifact_text_with(
        105,
        "linked",
        "\n[content]\nformat = \"markdown\"\npath = \"content/link.md\"\n",
    );
    repo.write(&format!("artifacts/{id}.toml"), text);
    assert_eq!(
        repo.issues().await,
        [(
            LoadIssueKind::UnsafeContentPath,
            format!("artifacts/{id}.toml")
        )]
    );
}

#[tokio::test]
async fn unknown_fields_are_non_fatal_issues() {
    let repo = repo().await;
    let file = repo.artifact_path(&repo.alpha);
    repo.write(&file, format!("{}future = 1\n", repo.read(&file)));
    let repository_file = ".functor/repository.toml";
    repo.write(
        repository_file,
        format!("{}owner = \"x\"\n", repo.read(repository_file)),
    );
    assert_eq!(
        repo.issues().await,
        [
            (
                LoadIssueKind::UnknownField,
                ".functor/repository.toml".to_owned()
            ),
            (LoadIssueKind::UnknownField, file),
        ]
    );
    let load = RepositoryReader::load(&repo.store).await.unwrap();
    assert!(!load.has_fatal_issues());
    assert!(RepositoryReader::snapshot(&repo.store).await.is_ok());
}

#[tokio::test]
async fn oversized_and_non_utf8_files_are_malformed() {
    let repo = repo().await;
    repo.write(
        "artifacts/big.toml",
        format!("# {}\n", "x".repeat(2 * 1024 * 1024)),
    );
    repo.write("artifacts/binary.toml", [0xff_u8, 0xfe, 0x00]);
    assert_eq!(
        repo.issues().await,
        [
            (
                LoadIssueKind::MalformedFile,
                "artifacts/big.toml".to_owned()
            ),
            (
                LoadIssueKind::MalformedFile,
                "artifacts/binary.toml".to_owned()
            ),
        ]
    );
}

#[tokio::test]
async fn missing_required_structure_is_reported() {
    let repo = repo().await;
    fs::remove_dir_all(repo.path("relationships")).unwrap();
    assert_eq!(
        repo.issues().await,
        [(LoadIssueKind::MissingStructure, "relationships".to_owned())]
    );
    fs::remove_file(repo.path(".functor/repository.toml")).unwrap();
    let load = RepositoryReader::load(&repo.store).await.unwrap();
    let info_issue = load.info.unwrap_err();
    assert_eq!(info_issue.kind, LoadIssueKind::MissingStructure);
    assert_eq!(info_issue.path.to_string(), ".functor/repository.toml");
    assert_eq!(load.artifacts.len(), 2);
}

#[tokio::test]
async fn an_unsupported_format_version_is_reported_without_hiding_the_entities() {
    let repo = repo().await;
    repo.write(".functor/repository.toml", "format_version = 2\n");
    let load = RepositoryReader::load(&repo.store).await.unwrap();
    assert_eq!(
        load.info.unwrap_err().kind,
        LoadIssueKind::UnsupportedFormat
    );
    assert_eq!(load.artifacts.len(), 2);
    let error = RepositoryReader::info(&repo.store).await.unwrap_err();
    assert!(matches!(
        error,
        functor_core::FunctorError::UnsupportedFormat {
            found: 2,
            supported: 1
        }
    ));
}

#[tokio::test]
async fn a_deleted_repository_is_not_a_repository() {
    let repo = repo().await;
    let Repo { dir, store, .. } = repo;
    let root = dir.path().to_path_buf();
    drop(dir);
    assert!(!root.exists());
    let error = RepositoryReader::load(&store).await.unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotARepository);
}

#[tokio::test]
async fn writes_into_a_damaged_repository_are_refused_with_inconsistent() {
    let repo = repo().await;
    repo.write("artifacts/bad.toml", "= = =");
    let error = ArtifactRepository::create(&repo.store, new_artifact("gamma"))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Inconsistent);
    assert!(!repo.path("artifacts/bad.toml.new").exists());
}

// ---------------------------------------------------------------------------------------------
// Locking and concurrency
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_writer_gives_up_with_locked_when_another_process_holds_the_lock() {
    let env = Env::new();
    let (dir, store) = init(
        &env,
        StoreOptions::default().with_lock_timeout(Duration::from_millis(150)),
    );
    let lock = WriteLock::new(&Layout::new(dir.path()), Duration::from_secs(5));

    let (held_tx, held_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let holder = thread::spawn(move || {
        lock.run(|| {
            held_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(())
        })
        .unwrap();
    });
    held_rx.recv().unwrap();

    let error = ArtifactRepository::create(&store, new_artifact("alpha"))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Locked);
    assert_eq!(
        ArtifactRepository::list(&store).await.unwrap(),
        [],
        "readers are never blocked, and the refused write left nothing behind"
    );

    release_tx.send(()).unwrap();
    holder.join().unwrap();
    assert!(
        ArtifactRepository::create(&store, new_artifact("alpha"))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn concurrent_writers_never_lose_or_corrupt_entities() {
    let env = Env::new();
    let (_dir, store) = init(&env, StoreOptions::default());
    let store = Arc::new(store);

    let mut tasks = Vec::new();
    for index in 0..12 {
        let store = Arc::clone(&store);
        tasks.push(tokio::spawn(async move {
            ArtifactRepository::create(&*store, new_artifact(&format!("item-{index}"))).await
        }));
    }
    for task in tasks {
        task.await.unwrap().unwrap();
    }

    let snapshot = RepositoryReader::snapshot(&*store).await.unwrap();
    assert_eq!(snapshot.artifacts().len(), 12);
    let mut ids: Vec<_> = snapshot.artifacts().iter().map(|a| *a.id()).collect();
    ids.dedup();
    assert_eq!(ids.len(), 12, "ids are unique");
    assert_eq!(
        RepositoryReader::load(&*store).await.unwrap().issues,
        [],
        "no partial or temporary files remain"
    );
}

#[tokio::test]
async fn racing_creates_of_one_slug_have_exactly_one_winner() {
    let env = Env::new();
    let (_dir, store) = init(&env, StoreOptions::default());
    let store = Arc::new(store);

    let mut tasks = Vec::new();
    for _ in 0..6 {
        let store = Arc::clone(&store);
        tasks.push(tokio::spawn(async move {
            ArtifactRepository::create(&*store, new_artifact("contested")).await
        }));
    }
    let mut winners = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(_) => winners += 1,
            Err(error) => assert_eq!(error.kind(), ErrorKind::Conflict),
        }
    }
    assert_eq!(winners, 1);
    assert_eq!(ArtifactRepository::list(&*store).await.unwrap().len(), 1);
}

#[tokio::test]
async fn separate_store_handles_see_each_others_writes() {
    let env = Env::new();
    let (dir, first) = init(&env, StoreOptions::default());
    let manager = FileRepositoryManager::new(env.clock(), env.ids());
    let handle = manager.open_blocking(dir.path()).unwrap();
    let second = FileStore::new(&handle, env.clock(), env.ids());

    let alpha = add_artifact(&first, "alpha").await;
    assert_eq!(
        ArtifactRepository::list(&second).await.unwrap(),
        std::slice::from_ref(&alpha)
    );
    let patched = ArtifactRepository::update(
        &second,
        alpha.id(),
        ArtifactPatch::new().with_content(Content::Inline {
            format: ContentFormat::new("text").unwrap(),
            text: "hello".into(),
        }),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        ArtifactRepository::get(&first, alpha.id()).await.unwrap(),
        Some(patched.artifact)
    );
}
