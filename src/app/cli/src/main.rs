//! The `functor` command line interface.
//!
//! The CLI is a projection over the core API and the composition root; see `docs/cli/cli-spec.md`.

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
mod run_render;
#[cfg(feature = "test-support")]
mod test_support;

use cli::{Cli, Command};
use context::{Globals, Services};

/// Runs a command and returns its policy exit code. Validation/run outcomes are result data;
/// infrastructure failures are errors.
async fn run(cli: Cli) -> Result<u8, FunctorError> {
    let globals = Globals {
        repo: cli.repo,
        output: cli.output,
        quiet: cli.quiet,
        verbose: cli.verbose,
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
        Command::Validate(args) => {
            return commands::validate::run(args, &globals, &services).await;
        }
        Command::Run(command) => {
            return commands::run::run(command, &globals, &services).await;
        }
        Command::Watch(args) => {
            let context = context::open(&globals, &services).await?;
            commands::watch::run(args, &context, &globals).await
        }
    }
    .map(|()| exit::SUCCESS)
}

fn main() -> ExitCode {
    // `parse` prints help and version to stdout (exit 0) and usage errors to stderr (exit 2).
    let cli = Cli::parse();
    let (mode, verbose) = (cli.output, cli.verbose);

    // A current-thread runtime is enough: the core is async, but this CLI does one thing at a time.
    // Timers (for `watch`) and signal handling (Ctrl-C) need the drivers enabled.
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let error =
                FunctorError::Internal(format!("could not start the async runtime: {error}"));
            return ExitCode::from(output::report_error(&error, mode, verbose));
        }
    };
    match runtime.block_on(run(cli)) {
        Ok(code) => ExitCode::from(code),
        Err(error) => ExitCode::from(output::report_error(&error, mode, verbose)),
    }
}
