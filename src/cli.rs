use clap::Parser;

/// Report which mechanism placed a command's binary on disk.
#[derive(Debug, Parser)]
#[command(name = "install-from", version, about, long_about = None)]
pub struct Args {
    /// Command name (searched in $PATH) or a path (contains '/')
    pub command: String,

    /// Evaluate every match on $PATH, not only the first
    #[arg(long)]
    pub all: bool,

    /// Print path / symlink chain / shebang below the result line
    #[arg(long)]
    pub detail: bool,

    /// Emit JSON (always includes detail fields)
    #[arg(long)]
    pub json: bool,
}
