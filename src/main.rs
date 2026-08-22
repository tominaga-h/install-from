mod cli;
mod detect;
mod error;
mod resolve;

use clap::Parser;
use std::process::ExitCode;

/// Exit codes per SPEC.md §2.2.
const EXIT_OK: u8 = 0;
#[allow(dead_code)]
const EXIT_UNKNOWN: u8 = 1;
const EXIT_NOT_FOUND: u8 = 2;
const EXIT_USAGE: u8 = 3;

fn main() -> ExitCode {
    let args = match cli::Args::try_parse() {
        Ok(args) => args,
        Err(err) => {
            // clap prints --help/--version text to stdout and exits 0 on its own;
            // any other parse error (unknown flag, missing arg, ...) is a usage error.
            if err.exit_code() == 0 {
                print!("{err}");
                return ExitCode::from(EXIT_OK);
            }
            eprint!("{err}");
            return ExitCode::from(EXIT_USAGE);
        }
    };

    run(args)
}

fn run(args: cli::Args) -> ExitCode {
    // Placeholder until Task 2 (resolve) and Task 4 (detect + output) land.
    eprintln!(
        "install-from: not yet implemented (command: {})",
        args.command
    );
    ExitCode::from(EXIT_NOT_FOUND)
}
