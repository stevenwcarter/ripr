use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ripr",
    version,
    about = "Read specific line ranges from files",
    long_about = None,
    arg_required_else_help = true,
)]
pub struct Cli {
    /// Path to the config file (overrides RIPR_CONFIG env and default location)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// sed-style expression, e.g. "5,10p" or "1,$p"
    #[arg(short = 'n', long = "sed", value_name = "EXPR")]
    pub sed: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    /// In native mode: RANGE FILE [FILE...]
    /// In sed mode (-n EXPR): FILE [FILE...]
    pub args: Vec<String>,
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
