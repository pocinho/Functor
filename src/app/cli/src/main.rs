//! The `functor` command line interface.
//!
//! The CLI is a projection over the core API and the composition root; see `docs/cli-spec.md`.

// The CLI is the only crate allowed to print.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::process::ExitCode;

use clap::Parser;
use functor_core::FunctorError;

mod cli;
mod commands;
mod context;
mod events;
mod exit;
mod output;
mod render;
mod resolve;
#[cfg(feature = "test-support")]
mod test_support;

use cli::{Cli, Command};
use context::{Globals, Services};

async fn run(cli: Cli) -> Result<(), FunctorError> {
    let globals = Globals {
        repo: cli.repo,
        output: cli.output,
        quiet: cli.quiet,
    };
    let services = Services::from_environment()?;
    match cli.command {
        Command::Init(args) => commands::init::run(args, &globals, &services).await,
        Command::Artifact(command) => {
            let context = context::open(&globals, &services).await?;
            commands::artifact::run(command, &context, &globals).await
        }
        Command::Relation(command) => {
            let context = context::open(&globals, &services).await?;
            commands::relation::run(command, &context, &globals).await
        }
        Command::Query(command) => {
            let context = context::open(&globals, &services).await?;
            commands::query::run(command, &context, &globals).await
        }
    }
}

fn main() -> ExitCode {
    // `parse` prints help and version to stdout (exit 0) and usage errors to stderr (exit 2).
    let cli = Cli::parse();
    let (mode, verbose) = (cli.output, cli.verbose);

    // A current-thread runtime is enough: the core is async, but this CLI does one thing at a time.
    let runtime = match tokio::runtime::Builder::new_current_thread().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            let error =
                FunctorError::Internal(format!("could not start the async runtime: {error}"));
            return ExitCode::from(output::report_error(&error, mode, verbose));
        }
    };
    match runtime.block_on(run(cli)) {
        Ok(()) => ExitCode::from(exit::SUCCESS),
        Err(error) => ExitCode::from(output::report_error(&error, mode, verbose)),
    }
}
