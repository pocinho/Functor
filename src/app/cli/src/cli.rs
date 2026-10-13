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

    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// The commands of the CLI.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a repository.
    Init(InitArgs),
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

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn the_argument_grammar_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn init_accepts_its_documented_options() {
        let cli = Cli::try_parse_from([
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
        let Command::Init(args) = cli.command;
        assert_eq!(args.path, Some(PathBuf::from("demo")));
        assert_eq!(args.name.as_deref(), Some("N"));
        assert_eq!(args.description.as_deref(), Some("D"));
        assert!(args.nested);
    }

    #[test]
    fn output_defaults_to_text_and_global_options_work_after_the_command() {
        let cli = Cli::try_parse_from(["functor", "init"]).unwrap();
        assert_eq!(cli.output, OutputMode::Text);
        let cli =
            Cli::try_parse_from(["functor", "init", "--output", "json", "--repo", "x"]).unwrap();
        assert_eq!(cli.output, OutputMode::Json);
        assert_eq!(cli.repo, Some(PathBuf::from("x")));
    }

    #[test]
    fn bad_input_is_rejected() {
        assert!(Cli::try_parse_from(["functor"]).is_err());
        assert!(Cli::try_parse_from(["functor", "init", "--bogus"]).is_err());
        assert!(Cli::try_parse_from(["functor", "init", "-o", "xml"]).is_err());
        assert!(Cli::try_parse_from(["functor", "nope"]).is_err());
    }
}
