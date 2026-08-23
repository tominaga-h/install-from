mod cli;
mod detect;
mod error;
mod output;
mod resolve;

use clap::Parser;
use error::ResolveError;
use serde_json::Value;
use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Exit codes per SPEC.md §2.2.
const EXIT_OK: u8 = 0;
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

fn path_dirs() -> Vec<PathBuf> {
    env::var_os("PATH")
        .map(|v| env::split_paths(&v).collect())
        .unwrap_or_default()
}

/// The outcome of processing one resolved path: either it was classified
/// (possibly as `unknown`) or its symlink chain couldn't be walked.
enum Outcome {
    Classified {
        finding: detect::Finding,
        target: resolve::Target,
    },
    ChainTooDeep(PathBuf),
}

fn process(env: &detect::Env, path: &Path) -> Outcome {
    match resolve::chain(path) {
        Ok(target) => {
            let finding = detect::run(env, &target);
            Outcome::Classified { finding, target }
        }
        Err(ResolveError::ChainTooDeep(path)) => Outcome::ChainTooDeep(path),
        Err(_) => unreachable!("chain() only returns ChainTooDeep"),
    }
}

/// Print the stderr side of one outcome per SPEC.md §2.3, and return
/// whether it counts as "determined" for exit-code aggregation
/// (SPEC.md §2.2). Also returns this target's JSON value when relevant
/// (`--json`), so `--all --json` can collect them into one array printed
/// after the loop instead of printing one object per target.
fn report(display_name: &str, outcome: &Outcome, args: &cli::Args) -> (bool, Option<Value>) {
    match outcome {
        Outcome::Classified { finding, target } => {
            let is_unknown = finding.source == detect::Source::Unknown;

            let json_value = if args.json {
                Some(output::render_json(display_name, finding, target))
            } else {
                None
            };

            if let Some(value) = &json_value {
                if !args.all {
                    println!("{value}");
                }
            } else if args.detail {
                println!("{}", output::render_detail(display_name, finding, target));
            } else if !is_unknown {
                println!("{}", output::render_line(display_name, finding));
            }

            if is_unknown {
                eprintln!(
                    "install-from: could not determine install source of {}",
                    target.path.display()
                );
            }

            (!is_unknown, json_value)
        }
        Outcome::ChainTooDeep(path) => {
            eprintln!(
                "install-from: {}: symlink chain too deep or cyclic",
                path.display()
            );
            (false, None)
        }
    }
}

fn run(args: cli::Args) -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("install-from: cannot read current directory: {err}");
            return ExitCode::from(EXIT_NOT_FOUND);
        }
    };

    let candidates = match resolve::candidates(&args.command, &path_dirs(), &cwd, args.all) {
        Ok(candidates) => candidates,
        Err(ResolveError::CommandNotFound(name)) => {
            eprintln!("install-from: {name}: command not found");
            return ExitCode::from(EXIT_NOT_FOUND);
        }
        Err(ResolveError::NoSuchFile(path)) => {
            eprintln!("install-from: {}: no such file", path.display());
            return ExitCode::from(EXIT_NOT_FOUND);
        }
        Err(ResolveError::ChainTooDeep(path)) => {
            eprintln!(
                "install-from: {}: symlink chain too deep or cyclic",
                path.display()
            );
            return ExitCode::from(EXIT_UNKNOWN);
        }
    };

    let env = detect::Env::from_env();
    let display_name = display_name(&args.command);

    let mut all_determined = true;
    let mut json_values = Vec::with_capacity(candidates.len());
    for path in &candidates {
        let outcome = process(&env, path);
        let (determined, json_value) = report(&display_name, &outcome, &args);
        all_determined &= determined;
        if let Some(value) = json_value {
            json_values.push(value);
        }
    }

    if args.json && args.all {
        println!("{}", Value::Array(json_values));
    }

    ExitCode::from(if all_determined {
        EXIT_OK
    } else {
        EXIT_UNKNOWN
    })
}

/// The name shown in output: the argument as given, or its basename if a
/// path was given. SPEC.md §3.3 `name` field contract.
fn display_name(arg: &str) -> String {
    if arg.contains('/') {
        PathBuf::from(arg)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| arg.to_string())
    } else {
        arg.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_bare_command_is_unchanged() {
        assert_eq!(display_name("jq"), "jq");
    }

    #[test]
    fn display_name_path_uses_basename() {
        assert_eq!(display_name("/usr/local/bin/jq"), "jq");
        assert_eq!(display_name("./bin/jq"), "jq");
    }
}
