use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
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
            // Resolve ranges and file list from cli.sed and cli.args
            let (ranges, files): (Vec<range::RangeSpec>, Vec<PathBuf>) = if let Some(expr) = cli.sed
            {
                // sed mode: all args are files
                if cli.args.is_empty() {
                    return Err(RipError::Parse(
                        "no files specified — usage: ripr -n '5,10p' FILE [FILE...]".to_string(),
                    ));
                }
                let ranges = sed_compat::parse_sed_n(&expr)?;
                let files = cli.args.iter().map(PathBuf::from).collect();
                (ranges, files)
            } else {
                // native mode: first arg is range, rest are files
                if cli.args.is_empty() {
                    return Err(RipError::Parse(
                        "usage: ripr RANGE FILE [FILE...] or ripr -n EXPR FILE [FILE...]"
                            .to_string(),
                    ));
                }
                let range_str = &cli.args[0];
                if cli.args.len() < 2 {
                    return Err(RipError::Parse(format!(
                        "no files specified — usage: ripr {range_str} FILE [FILE...]"
                    )));
                }
                let ranges = range::parse_ranges(range_str)?;
                let files = cli.args[1..].iter().map(PathBuf::from).collect();
                (ranges, files)
            };

            let whitelist = Whitelist::load(config_path)?;
            let stdout = io::stdout();
            let mut out = BufWriter::new(stdout.lock());

            for path in &files {
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
