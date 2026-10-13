//! The `functor` command line interface.
//!
//! The CLI is a projection over the core API and the composition root; see `docs/cli-spec.md`.

// The CLI is the only crate allowed to print.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::process::ExitCode;

use clap::Parser;
use functor_core::FunctorError;

mod cli;
mod exit;
mod init;
mod output;

use cli::{Cli, Command};

async fn run(cli: Cli) -> Result<(), FunctorError> {
    match cli.command {
        Command::Init(args) => init::run(args, cli.repo, cli.output).await,
    }
}

fn main() -> ExitCode {
    // `parse` prints help and version to stdout (exit 0) and usage errors to stderr (exit 2).
    let cli = Cli::parse();
    let mode = cli.output;

    // A current-thread runtime is enough: the core is async, but this CLI does one thing at a time.
    let runtime = match tokio::runtime::Builder::new_current_thread().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            let error =
                FunctorError::Internal(format!("could not start the async runtime: {error}"));
            return ExitCode::from(output::report_error(&error, mode));
        }
    };
    match runtime.block_on(run(cli)) {
        Ok(()) => ExitCode::from(exit::SUCCESS),
        Err(error) => ExitCode::from(output::report_error(&error, mode)),
    }
}
