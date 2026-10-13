//! Command line definition.
//!
//! See `docs/cli-spec.md`. This module only describes arguments; it contains no behavior.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// How results and errors are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputMode {
    /// Human-friendly text.
    Text,
    /// Machine-readable JSON with a stable schema.
    Json,
}

/// Create, relate, validate and evolve structured artifacts.
#[derive(Debug, Parser)]
#[command(name = "functor", version, about, arg_required_else_help = true)]
pub struct Cli {
    /// Repository root (or any directory inside it). `init` uses it as the target directory.
    #[arg(long, global = true, env = "FUNCTOR_REPO", value_name = "PATH")]
    pub repo: Option<PathBuf>,

    /// How to render results and errors.
    #[arg(short, long, global = true, value_enum, default_value_t = OutputMode::Text)]
    pub output: OutputMode,

    /// Show the cause chain of errors.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress confirmations in text mode (query results are still printed).
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// The commands of the CLI.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a repository.
    Init(InitArgs),
    /// Create, inspect and change artifacts.
    #[command(subcommand)]
    Artifact(ArtifactCommand),
    /// Create and inspect relationships between artifacts.
    #[command(subcommand)]
    Relation(RelationCommand),
    /// Search artifacts and navigate relationships (read-only).
    #[command(subcommand)]
    Query(QueryCommand),
    /// Check the repository for problems.
    Validate(ValidateArgs),
}

/// Arguments of `functor validate`.
#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Only report findings about this artifact (an id, canonical reference or slug) and the
    /// relationships touching it. All rules still run over the whole repository.
    pub artifact: Option<String>,

    /// Also fail (exit 6) when the outcome is `warning`.
    #[arg(long)]
    pub strict: bool,
}

/// Arguments of `functor init`.
#[derive(Debug, Args)]
pub struct InitArgs {
    /// Directory to create the repository in (default: --repo, then the current directory).
    pub path: Option<PathBuf>,

    /// Repository name (default: the directory name).
    #[arg(long)]
    pub name: Option<String>,

    /// Repository description.
    #[arg(long)]
    pub description: Option<String>,

    /// Allow creating a repository inside another repository.
    #[arg(long)]
    pub nested: bool,
}

/// The `functor artifact` subcommands.
#[derive(Debug, Subcommand)]
pub enum ArtifactCommand {
    /// Create an artifact with the given slug.
    Create(ArtifactCreateArgs),
    /// Show one artifact.
    Show(ArtifactShowArgs),
    /// List artifacts ordered by slug.
    List(ArtifactListArgs),
    /// Change an artifact.
    Update(ArtifactUpdateArgs),
}

/// Arguments of `functor artifact create`.
#[derive(Debug, Args)]
pub struct ArtifactCreateArgs {
    /// The slug: a unique, human-friendly alias such as `rfc-0000`.
    pub slug: String,

    /// Artifact type.
    #[arg(long = "type", value_name = "TYPE", default_value = "document")]
    pub artifact_type: String,

    /// Title (default: the slug).
    #[arg(long)]
    pub title: Option<String>,

    /// Description.
    #[arg(long)]
    pub description: Option<String>,

    /// A tag; repeat for several.
    #[arg(long = "tag", value_name = "TAG")]
    pub tags: Vec<String>,

    /// String metadata as KEY=VALUE; repeat for several.
    #[arg(long = "meta", value_name = "KEY=VALUE")]
    pub metadata: Vec<String>,

    /// Inline content.
    #[arg(long, value_name = "TEXT", conflicts_with = "content_path")]
    pub content_text: Option<String>,

    /// Reference an existing file inside the repository (it is not copied).
    #[arg(long, value_name = "PATH")]
    pub content_path: Option<PathBuf>,

    /// Content format (default: guessed from the file extension, else `text`).
    #[arg(long, value_name = "FORMAT")]
    pub content_format: Option<String>,
}

/// Arguments of `functor artifact show`.
#[derive(Debug, Args)]
pub struct ArtifactShowArgs {
    /// An artifact id, canonical reference or slug.
    pub artifact: String,

    /// Include the content (inline text, or the referenced file, up to 1 MiB).
    #[arg(long)]
    pub content: bool,
}

/// Arguments of `functor artifact list`.
#[derive(Debug, Args)]
pub struct ArtifactListArgs {
    /// Only artifacts of this type.
    #[arg(long = "type", value_name = "TYPE")]
    pub artifact_type: Option<String>,

    /// Only artifacts carrying this tag.
    #[arg(long, value_name = "TAG")]
    pub tag: Option<String>,

    /// Also show ids and update times.
    #[arg(long)]
    pub long: bool,
}

/// Arguments of `functor artifact update`.
#[derive(Debug, Args)]
pub struct ArtifactUpdateArgs {
    /// An artifact id, canonical reference or slug.
    pub artifact: String,

    /// Rename the slug.
    #[arg(long)]
    pub slug: Option<String>,

    /// New title.
    #[arg(long)]
    pub title: Option<String>,

    /// New description.
    #[arg(long, conflicts_with = "clear_description")]
    pub description: Option<String>,

    /// Remove the description.
    #[arg(long)]
    pub clear_description: bool,

    /// Replace the whole tag set with these comma-separated tags (`--tags ""` clears it).
    #[arg(long, value_name = "A,B,C")]
    pub tags: Option<String>,

    /// Set a metadata key as KEY=VALUE; repeat for several.
    #[arg(long = "meta", value_name = "KEY=VALUE")]
    pub metadata: Vec<String>,

    /// Remove a metadata key; repeat for several.
    #[arg(long = "unset-meta", value_name = "KEY")]
    pub unset_metadata: Vec<String>,

    /// Replace the content with inline text.
    #[arg(long, value_name = "TEXT", conflicts_with_all = ["content_path", "clear_content"])]
    pub content_text: Option<String>,

    /// Replace the content with a reference to a file inside the repository.
    #[arg(long, value_name = "PATH", conflicts_with = "clear_content")]
    pub content_path: Option<PathBuf>,

    /// Format of the new content.
    #[arg(long, value_name = "FORMAT")]
    pub content_format: Option<String>,

    /// Remove the content.
    #[arg(long)]
    pub clear_content: bool,

    /// Fail with exit code 4 unless the artifact is at this revision.
    #[arg(long, value_name = "N")]
    pub expect_revision: Option<u64>,
}

/// The `functor relation` subcommands.
#[derive(Debug, Subcommand)]
pub enum RelationCommand {
    /// Create `SOURCE --TYPE--> TARGET`.
    Add(RelationAddArgs),
    /// List relationships ordered by id.
    List(RelationListArgs),
    /// Show one relationship.
    Show(RelationShowArgs),
}

/// Arguments of `functor relation add`.
#[derive(Debug, Args)]
pub struct RelationAddArgs {
    /// The artifact that holds the relationship: an id, canonical reference or slug.
    pub source: String,

    /// `depends_on`, `references`, `derived_from` or `extends` (kebab-case is accepted).
    #[arg(value_name = "TYPE")]
    pub relation_type: String,

    /// The artifact pointed to: an id, canonical reference or slug.
    pub target: String,

    /// Accept a well-formed type name that is not built in.
    #[arg(long)]
    pub allow_custom_type: bool,

    /// String metadata as KEY=VALUE; repeat for several.
    #[arg(long = "meta", value_name = "KEY=VALUE")]
    pub metadata: Vec<String>,
}

/// Arguments of `functor relation list`.
#[derive(Debug, Args)]
pub struct RelationListArgs {
    /// Only relationships of this type; repeat for several.
    #[arg(long = "type", value_name = "TYPE")]
    pub types: Vec<String>,

    /// Only relationships starting at this artifact.
    #[arg(long, value_name = "ARTIFACT")]
    pub source: Option<String>,

    /// Only relationships pointing at this artifact.
    #[arg(long, value_name = "ARTIFACT")]
    pub target: Option<String>,
}

/// Arguments of `functor relation show`.
#[derive(Debug, Args)]
pub struct RelationShowArgs {
    /// A relationship id (`rel_...`) or canonical reference.
    pub relation: String,
}

/// The `functor query` subcommands. All are read-only.
#[derive(Debug, Subcommand)]
pub enum QueryCommand {
    /// Find artifacts by type, tag, text or metadata.
    Artifacts(QueryArtifactsArgs),
    /// What an artifact depends on.
    Dependencies(QueryTraverseArgs),
    /// What depends on an artifact.
    Dependents(QueryTraverseArgs),
    /// Every relationship touching an artifact.
    Relationships(QueryRelationshipsArgs),
}

/// Arguments of `functor query artifacts`. Criteria combine with AND.
#[derive(Debug, Args)]
pub struct QueryArtifactsArgs {
    /// Only artifacts of this type.
    #[arg(long = "type", value_name = "TYPE")]
    pub artifact_type: Option<String>,

    /// Only artifacts carrying this tag.
    #[arg(long, value_name = "TAG")]
    pub tag: Option<String>,

    /// Only artifacts whose slug, title or description contains this text (any case).
    #[arg(long, value_name = "TEXT")]
    pub text: Option<String>,

    /// Only artifacts with this string metadata, as KEY=VALUE; repeat for several.
    #[arg(long = "meta", value_name = "KEY=VALUE")]
    pub metadata: Vec<String>,
}

/// How far a traversal goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    /// At most this many steps.
    Limited(u32),
    /// The whole reachable closure.
    All,
}

fn parse_depth(text: &str) -> Result<Depth, String> {
    if text.eq_ignore_ascii_case("all") {
        return Ok(Depth::All);
    }
    text.parse::<u32>()
        .map(Depth::Limited)
        .map_err(|_| format!("`{text}` is not a whole number or `all`"))
}

/// Arguments of `functor query dependencies` and `functor query dependents`.
#[derive(Debug, Args)]
pub struct QueryTraverseArgs {
    /// An artifact id, canonical reference or slug.
    pub artifact: String,

    /// How many steps to follow: a number, or `all` for the transitive closure.
    #[arg(long, value_name = "N|all", value_parser = parse_depth, default_value = "1")]
    pub depth: Depth,

    /// Relationship type to follow instead of `depends_on`; repeat for several.
    #[arg(long = "type", value_name = "TYPE")]
    pub types: Vec<String>,
}

/// Which relationships of an artifact to list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RelationshipDirection {
    /// Relationships pointing at the artifact.
    In,
    /// Relationships starting at the artifact.
    Out,
    /// Both.
    Both,
}

/// Arguments of `functor query relationships`.
#[derive(Debug, Args)]
pub struct QueryRelationshipsArgs {
    /// An artifact id, canonical reference or slug.
    pub artifact: String,

    /// Only relationships of this type; repeat for several.
    #[arg(long = "type", value_name = "TYPE")]
    pub types: Vec<String>,

    /// Which direction to list.
    #[arg(long, value_enum, default_value_t = RelationshipDirection::Both)]
    pub direction: RelationshipDirection,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse<const N: usize>(args: [&str; N]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(args)
    }

    #[test]
    fn the_argument_grammar_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn init_accepts_its_documented_options() {
        let cli = parse([
            "functor",
            "init",
            "demo",
            "--name",
            "N",
            "--description",
            "D",
            "--nested",
            "-o",
            "json",
        ])
        .unwrap();
        assert_eq!(cli.output, OutputMode::Json);
        let Command::Init(args) = cli.command else {
            panic!("expected init");
        };
        assert_eq!(args.path, Some(PathBuf::from("demo")));
        assert_eq!(args.name.as_deref(), Some("N"));
        assert_eq!(args.description.as_deref(), Some("D"));
        assert!(args.nested);
    }

    #[test]
    fn global_options_work_before_and_after_the_command() {
        let cli = parse(["functor", "init"]).unwrap();
        assert_eq!(cli.output, OutputMode::Text);
        assert!(!cli.verbose && !cli.quiet);
        let cli = parse([
            "functor", "init", "--output", "json", "--repo", "x", "-v", "-q",
        ])
        .unwrap();
        assert_eq!(cli.output, OutputMode::Json);
        assert_eq!(cli.repo, Some(PathBuf::from("x")));
        assert!(cli.verbose && cli.quiet);
        let cli = parse(["functor", "-o", "json", "artifact", "list"]).unwrap();
        assert_eq!(cli.output, OutputMode::Json);
    }

    #[test]
    fn artifact_create_accepts_its_documented_options() {
        let cli = parse([
            "functor",
            "artifact",
            "create",
            "rfc-0000",
            "--type",
            "rfc",
            "--title",
            "Vision",
            "--description",
            "d",
            "--tag",
            "a",
            "--tag",
            "b",
            "--meta",
            "k=v",
            "--meta",
            "x=y",
            "--content-text",
            "hello",
            "--content-format",
            "markdown",
        ])
        .unwrap();
        let Command::Artifact(ArtifactCommand::Create(args)) = cli.command else {
            panic!("expected artifact create");
        };
        assert_eq!(args.slug, "rfc-0000");
        assert_eq!(args.artifact_type, "rfc");
        assert_eq!(args.tags, ["a", "b"]);
        assert_eq!(args.metadata, ["k=v", "x=y"]);
        assert_eq!(args.content_text.as_deref(), Some("hello"));
    }

    #[test]
    fn artifact_create_defaults_to_the_document_type() {
        let cli = parse(["functor", "artifact", "create", "a"]).unwrap();
        let Command::Artifact(ArtifactCommand::Create(args)) = cli.command else {
            panic!("expected artifact create");
        };
        assert_eq!(args.artifact_type, "document");
        assert_eq!(args.title, None);
    }

    #[test]
    fn mutually_exclusive_options_are_rejected() {
        assert!(
            parse([
                "functor",
                "artifact",
                "create",
                "a",
                "--content-text",
                "x",
                "--content-path",
                "y"
            ])
            .is_err()
        );
        assert!(
            parse([
                "functor",
                "artifact",
                "update",
                "a",
                "--description",
                "x",
                "--clear-description"
            ])
            .is_err()
        );
        assert!(
            parse([
                "functor",
                "artifact",
                "update",
                "a",
                "--content-text",
                "x",
                "--clear-content"
            ])
            .is_err()
        );
        assert!(
            parse([
                "functor",
                "artifact",
                "update",
                "a",
                "--content-text",
                "x",
                "--content-path",
                "y"
            ])
            .is_err()
        );
    }

    #[test]
    fn artifact_update_and_list_parse() {
        let cli = parse([
            "functor",
            "artifact",
            "update",
            "a",
            "--slug",
            "b",
            "--tags",
            "x,y",
            "--unset-meta",
            "k",
            "--expect-revision",
            "3",
        ])
        .unwrap();
        let Command::Artifact(ArtifactCommand::Update(args)) = cli.command else {
            panic!("expected artifact update");
        };
        assert_eq!(args.slug.as_deref(), Some("b"));
        assert_eq!(args.tags.as_deref(), Some("x,y"));
        assert_eq!(args.unset_metadata, ["k"]);
        assert_eq!(args.expect_revision, Some(3));

        let cli = parse([
            "functor", "artifact", "list", "--type", "rfc", "--tag", "core", "--long",
        ])
        .unwrap();
        let Command::Artifact(ArtifactCommand::List(args)) = cli.command else {
            panic!("expected artifact list");
        };
        assert!(args.long);
        assert_eq!(args.artifact_type.as_deref(), Some("rfc"));
    }

    #[test]
    fn bad_input_is_rejected() {
        assert!(parse(["functor"]).is_err());
        assert!(parse(["functor", "init", "--bogus"]).is_err());
        assert!(parse(["functor", "init", "-o", "xml"]).is_err());
        assert!(parse(["functor", "nope"]).is_err());
        assert!(parse(["functor", "artifact"]).is_err());
        assert!(parse(["functor", "artifact", "create"]).is_err());
        assert!(
            parse([
                "functor",
                "artifact",
                "update",
                "a",
                "--expect-revision",
                "x"
            ])
            .is_err()
        );
    }

    #[test]
    fn query_artifacts_accepts_every_criterion() {
        let cli = parse([
            "functor",
            "query",
            "artifacts",
            "--type",
            "rfc",
            "--tag",
            "core",
            "--text",
            "vision",
            "--meta",
            "status=draft",
            "--meta",
            "owner=ana",
        ])
        .unwrap();
        let Command::Query(QueryCommand::Artifacts(args)) = cli.command else {
            panic!("expected query artifacts");
        };
        assert_eq!(args.artifact_type.as_deref(), Some("rfc"));
        assert_eq!(args.tag.as_deref(), Some("core"));
        assert_eq!(args.text.as_deref(), Some("vision"));
        assert_eq!(args.metadata, ["status=draft", "owner=ana"]);
    }

    #[test]
    fn traversal_depth_is_a_number_or_all_and_defaults_to_one() {
        let depth = |extra: &[&str]| {
            let mut args = vec!["functor", "query", "dependencies", "a"];
            args.extend(extra);
            match Cli::try_parse_from(args).map(|cli| cli.command) {
                Ok(Command::Query(QueryCommand::Dependencies(args))) => Ok(args.depth),
                Ok(_) => panic!("expected query dependencies"),
                Err(error) => Err(error),
            }
        };
        assert_eq!(depth(&[]).unwrap(), Depth::Limited(1));
        assert_eq!(depth(&["--depth", "0"]).unwrap(), Depth::Limited(0));
        assert_eq!(depth(&["--depth", "7"]).unwrap(), Depth::Limited(7));
        assert_eq!(depth(&["--depth", "all"]).unwrap(), Depth::All);
        assert_eq!(depth(&["--depth", "ALL"]).unwrap(), Depth::All);
        for bad in ["x", "1.5", "", "4294967296"] {
            assert!(depth(&["--depth", bad]).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn dependencies_and_dependents_share_their_options() {
        for name in ["dependencies", "dependents"] {
            let cli = parse([
                "functor",
                "query",
                name,
                "rfc-0001",
                "--type",
                "depends_on",
                "--type",
                "extends",
            ])
            .unwrap();
            let Command::Query(QueryCommand::Dependencies(args) | QueryCommand::Dependents(args)) =
                cli.command
            else {
                panic!("expected a traversal command");
            };
            assert_eq!(args.artifact, "rfc-0001");
            assert_eq!(args.types, ["depends_on", "extends"]);
        }
    }

    #[test]
    fn query_relationships_directions_default_to_both() {
        let direction = |extra: &[&str]| {
            let mut args = vec!["functor", "query", "relationships", "a"];
            args.extend(extra);
            match Cli::try_parse_from(args).map(|cli| cli.command) {
                Ok(Command::Query(QueryCommand::Relationships(args))) => Ok(args.direction),
                Ok(_) => panic!("expected query relationships"),
                Err(error) => Err(error),
            }
        };
        assert_eq!(direction(&[]).unwrap(), RelationshipDirection::Both);
        assert_eq!(
            direction(&["--direction", "in"]).unwrap(),
            RelationshipDirection::In
        );
        assert_eq!(
            direction(&["--direction", "out"]).unwrap(),
            RelationshipDirection::Out
        );
        assert_eq!(
            direction(&["--direction", "both"]).unwrap(),
            RelationshipDirection::Both
        );
        assert!(direction(&["--direction", "sideways"]).is_err());
    }

    #[test]
    fn validate_takes_an_optional_artifact_and_strict() {
        let cli = parse(["functor", "validate"]).unwrap();
        let Command::Validate(args) = cli.command else {
            panic!("expected validate");
        };
        assert_eq!(args.artifact, None);
        assert!(!args.strict);

        let cli = parse(["functor", "validate", "rfc-0001", "--strict", "-o", "json"]).unwrap();
        assert_eq!(cli.output, OutputMode::Json);
        let Command::Validate(args) = cli.command else {
            panic!("expected validate");
        };
        assert_eq!(args.artifact.as_deref(), Some("rfc-0001"));
        assert!(args.strict);
        assert!(parse(["functor", "validate", "a", "b"]).is_err());
    }

    #[test]
    fn query_needs_a_subcommand_and_artifact() {
        assert!(parse(["functor", "query"]).is_err());
        assert!(parse(["functor", "query", "dependencies"]).is_err());
        assert!(parse(["functor", "query", "relationships"]).is_err());
        assert!(parse(["functor", "query", "nope"]).is_err());
    }

    #[test]
    fn relation_add_takes_source_type_target() {
        let cli = parse([
            "functor",
            "relation",
            "add",
            "rfc-0001",
            "depends-on",
            "rfc-0000",
            "--allow-custom-type",
            "--meta",
            "why=because",
        ])
        .unwrap();
        let Command::Relation(RelationCommand::Add(args)) = cli.command else {
            panic!("expected relation add");
        };
        assert_eq!(args.source, "rfc-0001");
        assert_eq!(args.relation_type, "depends-on");
        assert_eq!(args.target, "rfc-0000");
        assert!(args.allow_custom_type);
        assert_eq!(args.metadata, ["why=because"]);
        assert!(parse(["functor", "relation", "add", "a", "depends_on"]).is_err());
    }

    #[test]
    fn relation_list_and_show_parse() {
        let cli = parse([
            "functor",
            "relation",
            "list",
            "--type",
            "depends_on",
            "--type",
            "extends",
            "--source",
            "a",
            "--target",
            "b",
        ])
        .unwrap();
        let Command::Relation(RelationCommand::List(args)) = cli.command else {
            panic!("expected relation list");
        };
        assert_eq!(args.types, ["depends_on", "extends"]);
        assert_eq!(args.source.as_deref(), Some("a"));
        assert_eq!(args.target.as_deref(), Some("b"));

        let cli = parse(["functor", "relation", "show", "rel_X"]).unwrap();
        let Command::Relation(RelationCommand::Show(args)) = cli.command else {
            panic!("expected relation show");
        };
        assert_eq!(args.relation, "rel_X");
        assert!(parse(["functor", "relation", "show"]).is_err());
        assert!(parse(["functor", "relation"]).is_err());
    }
}
