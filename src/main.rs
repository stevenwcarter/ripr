use ripr::RipError;

fn run() -> Result<(), RipError> {
    // TODO: parse CLI args and dispatch to reader
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("ripr: {e}");
        std::process::exit(e.exit_code());
    }
}
