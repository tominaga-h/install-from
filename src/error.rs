use std::path::PathBuf;
use thiserror::Error;

/// Errors from resolving a command-name/path argument into a filesystem
/// target. Each variant maps to an exit code per SPEC.md §2.2.
#[derive(Debug, Error)]
pub enum ResolveError {
    /// A bare command name was not found on any `$PATH` entry.
    /// Maps to exit code 2.
    #[error("{0}: command not found")]
    CommandNotFound(String),

    /// An explicit path (argument containing '/') does not exist, is a
    /// directory, or is otherwise not a usable file. Maps to exit code 2.
    #[error("{0}: no such file")]
    NoSuchFile(PathBuf),

    /// Symlink chain exceeded the hop cap or contains a cycle.
    /// Maps to exit code 1.
    #[error("{0}: symlink chain too deep or cyclic")]
    ChainTooDeep(PathBuf),
}
