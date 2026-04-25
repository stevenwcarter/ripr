mod common;
use assert_cmd::Command;
use common::{isolated_config, numbered_file, ripr};
use predicates::prelude::*;

#[test]
fn test_whitelist_add_and_list() {
    let cfg = isolated_config();
    let f = numbered_file(5);
    let path = f.path().to_str().unwrap();

    ripr(&cfg)
        .args(["whitelist", "add", path])
        .assert()
        .success()
        .stderr(predicate::str::contains("Added"));

    let canonical = f.path().canonicalize().unwrap();
    let canonical_str = canonical.to_string_lossy();
    ripr(&cfg)
        .args(["whitelist", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(canonical_str.as_ref()))
        .stderr("");
}

#[test]
fn test_whitelist_add_idempotent() {
    let cfg = isolated_config();
    let f = numbered_file(5);
    let path = f.path().to_str().unwrap();

    // Add twice
    ripr(&cfg)
        .args(["whitelist", "add", path])
        .assert()
        .success()
        .stderr(predicate::str::contains("Added"));
    ripr(&cfg)
        .args(["whitelist", "add", path])
        .assert()
        .success()
        .stderr(predicate::str::contains("Added"));

    // List output should contain path exactly once (one newline-terminated line)
    let output = ripr(&cfg)
        .args(["whitelist", "list"])
        .assert()
        .success()
        .stderr("")
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(output).unwrap();
    // Canonicalize the temp path to match what the whitelist stores
    let canonical = f.path().canonicalize().unwrap();
    let canonical_str = canonical.to_string_lossy();
    let count = stdout.lines().filter(|line| *line == canonical_str).count();
    assert_eq!(count, 1, "path should appear exactly once, got:\n{stdout}");
}

#[test]
fn test_whitelist_add_then_read() {
    let cfg = isolated_config();
    let f = numbered_file(5);
    let path = f.path().to_str().unwrap();

    ripr(&cfg)
        .args(["whitelist", "add", path])
        .assert()
        .success()
        .stderr(predicate::str::contains("Added"));

    ripr(&cfg)
        .args(["1-3", path])
        .assert()
        .success()
        .stdout("1\n2\n3\n");
}

#[test]
fn test_whitelist_remove() {
    let cfg = isolated_config();
    let f = numbered_file(5);
    let path = f.path().to_str().unwrap();

    ripr(&cfg)
        .args(["whitelist", "add", path])
        .assert()
        .success()
        .stderr(predicate::str::contains("Added"));
    ripr(&cfg)
        .args(["whitelist", "remove", path])
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed"));

    // List should now be empty
    ripr(&cfg)
        .args(["whitelist", "list"])
        .assert()
        .success()
        .stdout("")
        .stderr("");
}

#[test]
fn test_whitelist_remove_nonexistent() {
    let cfg = isolated_config();
    let f = numbered_file(5);
    let path = f.path().to_str().unwrap();

    // Remove a path that was never added — should succeed silently
    // Note: remove uses canonicalize which requires the path to exist on disk,
    // so we use a real file that just isn't in the whitelist.
    ripr(&cfg)
        .args(["whitelist", "remove", path])
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed"));
}

#[test]
fn test_whitelist_add_dir_then_read_file_inside() {
    let cfg = isolated_config();
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("data.txt");
    std::fs::write(&file_path, "alpha\nbeta\ngamma\ndelta\nepsilon\n").unwrap();

    // Whitelist the directory, not the file directly
    ripr(&cfg)
        .args(["whitelist", "add", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("Added"));

    // Reading a file inside the whitelisted directory should succeed
    ripr(&cfg)
        .args(["2-3", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout("beta\ngamma\n");
}

#[test]
fn test_denied_error_format() {
    let cfg = isolated_config();
    let f = numbered_file(5);
    let path = f.path().to_str().unwrap();
    // Do NOT whitelist

    let output = ripr(&cfg)
        .args(["1-2", path])
        .assert()
        .failure()
        .code(2)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).unwrap();

    // Must contain the file path
    assert!(
        stderr.contains(path),
        "stderr should contain the denied path; got:\n{stderr}"
    );

    // Must contain a single-quoted copy-paste command
    assert!(
        stderr.contains("ripr whitelist add '"),
        "stderr should contain single-quoted whitelist command; got:\n{stderr}"
    );
}

#[test]
fn test_config_flag_overrides_env() {
    let cfg_flag = isolated_config(); // config used via --config
    let cfg_env = isolated_config(); // config used via RIPR_CONFIG (should be ignored)
    let f = numbered_file(5);
    let path = f.path().to_str().unwrap();

    // Whitelist in the --config file only (not in RIPR_CONFIG env file)
    Command::cargo_bin("ripr")
        .unwrap()
        .arg("--config")
        .arg(cfg_flag.path().join("config.toml"))
        .env("RIPR_CONFIG", cfg_env.path().join("config.toml"))
        .args(["whitelist", "add", path])
        .assert()
        .success();

    // Read should succeed using --config (not RIPR_CONFIG which has empty whitelist)
    Command::cargo_bin("ripr")
        .unwrap()
        .arg("--config")
        .arg(cfg_flag.path().join("config.toml"))
        .env("RIPR_CONFIG", cfg_env.path().join("config.toml"))
        .args(["1-2", path])
        .assert()
        .success()
        .stdout("1\n2\n");

    // Read using RIPR_CONFIG only (empty whitelist) should fail
    Command::cargo_bin("ripr")
        .unwrap()
        .env("RIPR_CONFIG", cfg_env.path().join("config.toml"))
        .args(["1-2", path])
        .assert()
        .failure()
        .code(2);
}
