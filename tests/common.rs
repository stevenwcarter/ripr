use assert_cmd::Command;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

/// Create a temp file with numbered lines 1..=n
pub fn numbered_file(n: u32) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    for i in 1..=n {
        writeln!(f, "{i}").unwrap();
    }
    f
}

/// Create a temp dir for config isolation
pub fn isolated_config() -> TempDir {
    tempfile::tempdir().unwrap()
}

/// Build a ripr Command with an isolated config dir via RIPR_CONFIG env var
pub fn ripr(config_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("ripr").unwrap();
    cmd.env("RIPR_CONFIG", config_dir.path().join("config.toml"));
    cmd
}

/// Whitelist a file path in the given config
#[allow(dead_code)]
pub fn whitelist_add(config_dir: &TempDir, path: &str) {
    ripr(config_dir)
        .args(["whitelist", "add", path])
        .assert()
        .success();
}
