//! The `functor` command line interface.
//!
//! The CLI is a projection over the core API and the composition root; see `docs/cli-spec.md`.

// The CLI is the only crate allowed to print.
#![allow(clippy::print_stdout, clippy::print_stderr)]

use clap::Parser;

// Wired into command handling in M4; until then only the tests use it.
#[cfg_attr(not(test), allow(dead_code))]
mod exit;

/// Create, relate, validate and evolve structured artifacts.
#[derive(Parser)]
#[command(name = "functor", version, about, arg_required_else_help = true)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
