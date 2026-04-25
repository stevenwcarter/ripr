use std::io::{self, BufWriter};
use std::path::Path;
use std::process;

use clap::Parser;

use ripr::cli::{Cli, Commands, WhitelistAction};
use ripr::config::resolve_config_path;
use ripr::whitelist::Whitelist;
use ripr::{RipError, range, reader, sed_compat};

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("ripr: {e}");
        process::exit(e.exit_code());
    }
}

fn run(cli: Cli) -> Result<(), RipError> {
    let config_path = resolve_config_path(cli.config.as_deref())?;

    match cli.command {
        Some(Commands::Whitelist(args)) => {
            let mut whitelist = Whitelist::load(config_path)?;
            match args.action {
                WhitelistAction::Add { path } => {
                    whitelist.add(&path)?;
                    println!("Added: {}", path.display());
                }
                WhitelistAction::Remove { path } => {
                    whitelist.remove(&path)?;
                    println!("Removed: {}", path.display());
                }
                WhitelistAction::List => {
                    for p in whitelist.list() {
                        println!("{p}");
                    }
                }
            }
        }
        None => {
            // Determine ranges from -n (sed) or positional range arg
            let ranges = if let Some(expr) = cli.sed {
                sed_compat::parse_sed_n(&expr)?
            } else if let Some(range_str) = cli.range {
                range::parse_ranges(&range_str)?
            } else {
                return Err(RipError::Parse(
                    "no range specified — use 'ripr RANGE FILE' or 'ripr -n EXPR FILE'".to_string(),
                ));
            };

            if cli.files.is_empty() {
                return Err(RipError::Parse("no files specified".to_string()));
            }

            let whitelist = Whitelist::load(config_path)?;
            let stdout = io::stdout();
            let mut out = BufWriter::new(stdout.lock());

            for path in &cli.files {
                if path == Path::new("-") {
                    reader::read_stdin(&ranges, &mut out)?;
                } else {
                    whitelist.check(path)?;
                    reader::read_file(path, &ranges, &mut out)?;
                }
            }
        }
    }

    Ok(())
}
