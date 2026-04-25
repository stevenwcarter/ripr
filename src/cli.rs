use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ripr",
    version,
    about = "Read specific line ranges from files",
    long_about = "ripr returns line ranges from files.\nIt is a safe, whitelisted alternative to `sed -n 'N,Mp'` for read-only line extraction.",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Path to the config file (overrides RIPR_CONFIG env and default location)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Line range(s) to extract, e.g. "5-10", "(3,8)", "5-10;20-25"
    pub range: Option<String>,

    /// sed-style expression, e.g. "5,10p" or "1,$p"
    #[arg(short = 'n')]
    pub sed: Option<String>,

    /// File(s) to read. Use "-" for stdin.
    pub files: Vec<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage the file/directory whitelist
    Whitelist(WhitelistArgs),
}

#[derive(Args, Debug)]
pub struct WhitelistArgs {
    #[command(subcommand)]
    pub action: WhitelistAction,
}

#[derive(Subcommand, Debug)]
pub enum WhitelistAction {
    /// Add a path to the whitelist
    Add {
        /// Path to add (file or directory)
        path: PathBuf,
    },
    /// Remove a path from the whitelist
    Remove {
        /// Path to remove
        path: PathBuf,
    },
    /// List all whitelisted paths
    List,
}
