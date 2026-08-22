mod cli;
mod detect;
mod error;
mod output;
mod resolve;

use clap::Parser;
use error::ResolveError;
use std::env;
use std::path::PathBuf;
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

fn run(args: cli::Args) -> ExitCode {
    // --all, --detail, --json land in Tasks 5 and 11; this first vertical
    // slice resolves and reports a single target as plain text.
    let cwd = match env::current_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("install-from: cannot read current directory: {err}");
            return ExitCode::from(EXIT_NOT_FOUND);
        }
    };

    let candidates = match resolve::candidates(&args.command, &path_dirs(), &cwd, false) {
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
    let path = &candidates[0];

    let target = match resolve::chain(path) {
        Ok(target) => target,
        Err(ResolveError::ChainTooDeep(path)) => {
            eprintln!(
                "install-from: {}: symlink chain too deep or cyclic",
                path.display()
            );
            return ExitCode::from(EXIT_UNKNOWN);
        }
        Err(_) => unreachable!("chain() only returns ChainTooDeep"),
    };

    let env = detect::Env::from_env();
    let finding = detect::run(&env, &target);

    let display_name = display_name(&args.command);

    if finding.source == detect::Source::Unknown {
        eprintln!(
            "install-from: could not determine install source of {}",
            path.display()
        );
        return ExitCode::from(EXIT_UNKNOWN);
    }

    println!("{}", output::render_line(&display_name, &finding));
    ExitCode::from(EXIT_OK)
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
