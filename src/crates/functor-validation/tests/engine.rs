//! `RuleValidationEngine` over real repositories: one minimal broken repository per rule code,
//! determinism, scoping and the stores' agreement.

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::unused_self)]

use std::fs;
use std::path::PathBuf;

use functor_core::{
    ArtifactRepository, ErrorKind, InitOptions, RelationshipRepository, ValidationEngine,
    ValidationTarget,
};
use functor_model::{
    Artifact, ArtifactId, ArtifactRef, ArtifactType, NewArtifact, NewRelationship, Outcome,
    Relationship, RelationshipId, RelationshipType, RepoPath, Slug, Subject, Title,
    ValidationReport,
};
use functor_repository::FileRepositoryManager;
use functor_store::{FileStore, MemoryStore, StoreOptions};
use functor_testkit::Env;
use functor_validation::RuleValidationEngine;
use tempfile::TempDir;

struct Repo {
    dir: TempDir,
    env: Env,
    store: FileStore,
    alpha: Artifact,
    beta: Artifact,
    rel: Relationship,
}

fn new_artifact(slug: &str) -> NewArtifact {
    NewArtifact::new(
        Slug::parse(slug).unwrap(),
        ArtifactType::document(),
        Title::new(slug).unwrap(),
    )
}

/// `beta depends_on alpha`, stored as real files.
async fn repo() -> Repo {
    let env = Env::new();
    let dir = tempfile::tempdir().unwrap();
    let manager = FileRepositoryManager::new(env.clock(), env.ids());
    let handle = manager
        .init_blocking(dir.path(), &InitOptions::new().with_name("Test"))
        .unwrap();
    let store = FileStore::with_options(&handle, env.clock(), env.ids(), StoreOptions::default());
    let alpha = ArtifactRepository::create(&store, new_artifact("alpha"))
        .await
        .unwrap();
    let beta = ArtifactRepository::create(&store, new_artifact("beta"))
        .await
        .unwrap();
    let rel = RelationshipRepository::create(
        &store,
        NewRelationship::new(RelationshipType::DependsOn, *beta.id(), *alpha.id()),
    )
    .await
    .unwrap();
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

    fn artifact_file(&self, artifact: &Artifact) -> String {
        format!("artifacts/{}.toml", artifact.id())
    }

    fn rel_file(&self) -> String {
        format!("relationships/{}.toml", self.rel.id())
    }

    async fn validate(&self, target: ValidationTarget) -> ValidationReport {
        RuleValidationEngine::new(&self.store, self.env.clock())
            .validate(target)
            .await
            .unwrap()
    }

    async fn report(&self) -> ValidationReport {
        self.validate(ValidationTarget::Repository).await
    }
}

fn codes(report: &ValidationReport) -> Vec<&'static str> {
    report.findings().iter().map(|f| f.code.as_str()).collect()
}

async fn codes_of(repo: &Repo) -> Vec<&'static str> {
    codes(&repo.report().await)
}

#[tokio::test]
async fn a_healthy_repository_passes() {
    let repo = repo().await;
    let report = repo.report().await;
    assert_eq!(report.outcome(), Outcome::Passed);
    assert_eq!(report.findings(), []);
    assert_eq!(report.counts().artifacts, 2);
    assert_eq!(report.counts().relationships, 1);
    assert_eq!(
        report.completed(),
        repo.env.now(),
        "stamped by the injected clock"
    );
}

#[tokio::test]
async fn the_memory_store_reports_a_clean_repository_too() {
    let env = Env::new();
    let memory = MemoryStore::new(env.clock(), env.ids());
    let a = ArtifactRepository::create(&memory, new_artifact("alpha"))
        .await
        .unwrap();
    let b = ArtifactRepository::create(&memory, new_artifact("beta"))
        .await
        .unwrap();
    RelationshipRepository::create(
        &memory,
        NewRelationship::new(RelationshipType::DependsOn, *b.id(), *a.id()),
    )
    .await
    .unwrap();
    let report = RuleValidationEngine::new(&memory, env.clock())
        .validate(ValidationTarget::Repository)
        .await
        .unwrap();
    assert_eq!(report.outcome(), Outcome::Passed);
    assert_eq!(report.counts().artifacts, 2);
    assert_eq!(report.counts().relationships, 1);
}

// ---- one broken repository per rule code ----

#[tokio::test]
async fn v0001_malformed_file() {
    let repo = repo().await;
    repo.write("artifacts/junk.toml", "= = =");
    assert_eq!(codes_of(&repo).await, ["V0001"]);
    let report = repo.report().await;
    assert_eq!(report.outcome(), Outcome::Failed);
    let finding = &report.findings()[0];
    assert_eq!(
        finding.subject,
        Subject::File(RepoPath::new("artifacts/junk.toml").unwrap())
    );
    assert!(!finding.message.ends_with('.') && !finding.message.contains('\n'));
}

#[tokio::test]
async fn v0002_missing_identifier() {
    let repo = repo().await;
    let good = repo.read(&repo.artifact_file(&repo.alpha));
    repo.write(
        "artifacts/noid.toml",
        good.replace(&format!("id = \"{}\"\n", repo.alpha.id()), ""),
    );
    assert_eq!(codes_of(&repo).await, ["V0002"]);
}

#[tokio::test]
async fn v0003_duplicate_artifact_id() {
    let repo = repo().await;
    let good = repo.read(&repo.artifact_file(&repo.alpha));
    // A second file with the same id cannot also carry the id as its name.
    repo.write("artifacts/zzz-copy.toml", good);
    assert_eq!(codes_of(&repo).await, ["V0003", "V0017"]);
    let report = repo.report().await;
    assert_eq!(
        report.findings()[0].related,
        [Subject::File(
            RepoPath::new(&repo.artifact_file(&repo.alpha)).unwrap()
        )]
    );
}

#[tokio::test]
async fn v0004_duplicate_slug() {
    let repo = repo().await;
    let good = repo.read(&repo.artifact_file(&repo.alpha));
    let other = ArtifactId::from_bits(9999);
    repo.write(
        &format!("artifacts/{other}.toml"),
        good.replace(&repo.alpha.id().to_string(), &other.to_string()),
    );
    assert_eq!(codes_of(&repo).await, ["V0004"]);
}

#[tokio::test]
async fn v0005_invalid_identifier_or_slug() {
    let repo = repo().await;
    let good = repo.read(&repo.artifact_file(&repo.alpha));
    repo.write(
        "artifacts/badid.toml",
        good.replace(&repo.alpha.id().to_string(), "art_nope"),
    );
    repo.write(
        "artifacts/badslug.toml",
        good.replace("slug = \"alpha\"", "slug = \"Not A Slug\""),
    );
    assert_eq!(codes_of(&repo).await, ["V0005", "V0005"]);
}

#[tokio::test]
async fn v0006_relationship_source_does_not_exist() {
    let repo = repo().await;
    fs::remove_file(repo.path(&repo.artifact_file(&repo.beta))).unwrap();
    assert_eq!(codes_of(&repo).await, ["V0006"]);
    let report = repo.report().await;
    assert_eq!(
        report.findings()[0].subject,
        Subject::Relationship(*repo.rel.id())
    );
    assert_eq!(
        report.findings()[0].message,
        format!("source {} does not exist", repo.beta.id())
    );
}

#[tokio::test]
async fn v0007_relationship_target_does_not_exist() {
    let repo = repo().await;
    fs::remove_file(repo.path(&repo.artifact_file(&repo.alpha))).unwrap();
    assert_eq!(codes_of(&repo).await, ["V0007"]);
}

#[tokio::test]
async fn v0008_duplicate_relationship_id() {
    let repo = repo().await;
    let good = repo.read(&repo.rel_file());
    repo.write("relationships/rel_zzz-copy.toml", good);
    assert_eq!(codes_of(&repo).await, ["V0008", "V0017"]);
}

#[tokio::test]
async fn v0009_self_referencing_relationship() {
    let repo = repo().await;
    let good = repo.read(&repo.rel_file());
    let other = RelationshipId::from_bits(9999);
    repo.write(
        &format!("relationships/{other}.toml"),
        good.replace(&repo.beta.id().to_string(), &repo.alpha.id().to_string())
            .replace(&repo.rel.id().to_string(), &other.to_string()),
    );
    assert_eq!(codes_of(&repo).await, ["V0009"]);
}

#[tokio::test]
async fn v0010_unsafe_and_missing_content() {
    let repo = repo().await;
    let good = repo.read(&repo.artifact_file(&repo.alpha));
    let with_content = |bits: u128, slug: &str, path: &str| {
        let id = ArtifactId::from_bits(bits);
        let text = good
            .replace(&repo.alpha.id().to_string(), &id.to_string())
            .replace("slug = \"alpha\"", &format!("slug = \"{slug}\""));
        repo.write(
            &format!("artifacts/{id}.toml"),
            format!("{text}\n[content]\nformat = \"markdown\"\npath = \"{path}\"\n"),
        );
    };
    with_content(101, "escapes", "../outside.md");
    assert_eq!(codes_of(&repo).await, ["V0010"]);
    with_content(102, "missing", "content/missing.md");
    assert_eq!(codes_of(&repo).await, ["V0010", "V0010"]);
    repo.write("content/missing.md", "now it exists");
    assert_eq!(codes_of(&repo).await, ["V0010"]);
}

#[tokio::test]
async fn v0011_cycle_in_an_acyclic_type() {
    let repo = repo().await;
    RelationshipRepository::create(
        &repo.store,
        NewRelationship::new(
            RelationshipType::DependsOn,
            *repo.alpha.id(),
            *repo.beta.id(),
        ),
    )
    .await
    .unwrap();
    assert_eq!(codes_of(&repo).await, ["V0011"]);
    let report = repo.report().await;
    let finding = &report.findings()[0];
    assert_eq!(finding.subject, Subject::Artifact(*repo.alpha.id()));
    assert_eq!(
        finding.related,
        [
            Subject::Artifact(*repo.alpha.id()),
            Subject::Artifact(*repo.beta.id())
        ]
    );
    assert_eq!(
        finding.message,
        "cycle in `depends_on` relationships among alpha, beta"
    );
}

#[tokio::test]
async fn v0012_custom_relationship_type_is_a_warning() {
    let repo = repo().await;
    RelationshipRepository::create(
        &repo.store,
        NewRelationship::new(
            RelationshipType::parse("supports").unwrap(),
            *repo.alpha.id(),
            *repo.beta.id(),
        ),
    )
    .await
    .unwrap();
    assert_eq!(codes_of(&repo).await, ["V0012"]);
    assert_eq!(repo.report().await.outcome(), Outcome::Warning);
}

#[tokio::test]
async fn v0013_unsupported_format_version() {
    let repo = repo().await;
    repo.write(".functor/repository.toml", "format_version = 9\n");
    assert_eq!(codes_of(&repo).await, ["V0013"]);
    let report = repo.report().await;
    assert_eq!(report.outcome(), Outcome::Failed);
    assert_eq!(report.counts().artifacts, 2, "entities are still checked");
}

#[tokio::test]
async fn v0014_repository_structure_violation() {
    let repo = repo().await;
    fs::remove_dir_all(repo.path("relationships")).unwrap();
    assert_eq!(codes_of(&repo).await, ["V0014"]);

    fs::remove_file(repo.path(".functor/repository.toml")).unwrap();
    assert_eq!(codes_of(&repo).await, ["V0014", "V0014"]);
}

#[tokio::test]
async fn v0015_duplicate_relationship_is_a_warning() {
    let repo = repo().await;
    let good = repo.read(&repo.rel_file());
    let other = RelationshipId::from_bits(9999);
    repo.write(
        &format!("relationships/{other}.toml"),
        good.replace(&repo.rel.id().to_string(), &other.to_string()),
    );
    assert_eq!(codes_of(&repo).await, ["V0015"]);
    assert_eq!(repo.report().await.outcome(), Outcome::Warning);
}

#[tokio::test]
async fn v0016_unexpected_file_is_a_warning() {
    let repo = repo().await;
    repo.write("artifacts/notes.txt", "scratch");
    repo.write("relationships/.tmp-left-over", "");
    assert_eq!(codes_of(&repo).await, ["V0016", "V0016"]);
    assert_eq!(repo.report().await.outcome(), Outcome::Warning);
}

#[tokio::test]
async fn v0017_file_name_does_not_match_the_id() {
    let repo = repo().await;
    fs::rename(
        repo.path(&repo.artifact_file(&repo.beta)),
        repo.path("artifacts/renamed.toml"),
    )
    .unwrap();
    assert_eq!(codes_of(&repo).await, ["V0017"]);
}

#[tokio::test]
async fn v0018_unknown_field_is_informational() {
    let repo = repo().await;
    let file = repo.artifact_file(&repo.alpha);
    repo.write(&file, format!("{}future = 1\n", repo.read(&file)));
    assert_eq!(codes_of(&repo).await, ["V0018"]);
    let report = repo.report().await;
    assert_eq!(report.outcome(), Outcome::Passed);
    assert_eq!(report.counts().infos, 1);
}

// ---- behavior of the whole run ----

#[tokio::test]
async fn validation_does_not_modify_the_repository() {
    let repo = repo().await;
    repo.write("artifacts/junk.toml", "= = =");
    let snapshot = |repo: &Repo| -> Vec<(String, Vec<u8>)> {
        let mut files = Vec::new();
        for dir in [".functor", "artifacts", "relationships"] {
            for entry in fs::read_dir(repo.path(dir)).unwrap() {
                let path = entry.unwrap().path();
                if path.is_file() {
                    files.push((
                        path.strip_prefix(repo.dir.path())
                            .unwrap()
                            .display()
                            .to_string(),
                        fs::read(&path).unwrap(),
                    ));
                }
            }
        }
        files.sort();
        files
    };
    let before = snapshot(&repo);
    repo.report().await;
    repo.validate(ValidationTarget::Artifact(
        ArtifactRef::parse("alpha").unwrap(),
    ))
    .await;
    assert_eq!(snapshot(&repo), before);
}

#[tokio::test]
async fn repeated_runs_give_identical_reports() {
    let repo = repo().await;
    repo.write("artifacts/junk.toml", "= = =");
    repo.write("artifacts/notes.txt", "x");
    fs::remove_file(repo.path(&repo.artifact_file(&repo.alpha))).unwrap();
    let first = repo.report().await;
    let second = repo.report().await;
    assert_eq!(first, second);
    assert_eq!(codes(&first), ["V0001", "V0007", "V0016"]);
}

#[tokio::test]
async fn several_problems_are_all_reported_in_one_run() {
    let repo = repo().await;
    repo.write("artifacts/junk.toml", "= = =");
    repo.write("artifacts/notes.txt", "x");
    fs::remove_file(repo.path(&repo.artifact_file(&repo.alpha))).unwrap();
    let report = repo.report().await;
    assert_eq!(report.outcome(), Outcome::Failed);
    assert_eq!(report.counts().errors, 2);
    assert_eq!(report.counts().warnings, 1);
}

#[tokio::test]
async fn scoped_validation_follows_the_artifact() {
    let repo = repo().await;
    repo.write("artifacts/notes.txt", "x");
    RelationshipRepository::create(
        &repo.store,
        NewRelationship::new(
            RelationshipType::parse("supports").unwrap(),
            *repo.beta.id(),
            *repo.beta.id(),
        ),
    )
    .await
    .unwrap();
    let scoped = |slug: &str| {
        repo.validate(ValidationTarget::Artifact(
            ArtifactRef::parse(slug).unwrap(),
        ))
    };
    assert_eq!(codes(&repo.report().await), ["V0012", "V0016"]);
    assert_eq!(codes(&scoped("beta").await), ["V0012"]);
    assert_eq!(codes(&scoped("alpha").await), Vec::<&str>::new());
    assert_eq!(scoped("alpha").await.outcome(), Outcome::Passed);
}

#[tokio::test]
async fn scoping_an_unknown_artifact_is_not_found() {
    let repo = repo().await;
    let error = RuleValidationEngine::new(&repo.store, repo.env.clock())
        .validate(ValidationTarget::Artifact(
            ArtifactRef::parse("nope").unwrap(),
        ))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[tokio::test]
async fn a_missing_root_is_an_error_but_missing_structure_is_a_finding() {
    let repo = repo().await;
    let engine = RuleValidationEngine::new(&repo.store, repo.env.clock());
    let root = repo.dir.path().to_path_buf();
    fs::remove_dir_all(root.join("artifacts")).unwrap();
    fs::remove_dir_all(root.join("relationships")).unwrap();
    let report = engine.validate(ValidationTarget::Repository).await.unwrap();
    assert_eq!(codes(&report), ["V0014", "V0014"]);

    fs::remove_dir_all(&root).unwrap();
    let error = engine
        .validate(ValidationTarget::Repository)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotARepository);
}

#[tokio::test]
async fn the_clock_stamp_changes_nothing_else() {
    let repo = repo().await;
    let first = repo.report().await;
    repo.env.advance(60);
    let second = repo.report().await;
    assert_eq!(first.findings(), second.findings());
    assert_eq!(first.outcome(), second.outcome());
    assert_ne!(first.completed(), second.completed());
}
