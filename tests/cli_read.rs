mod common;
use common::{isolated_config, numbered_file, ripr, whitelist_add};
use predicates::prelude::*;

#[test]
fn test_inclusive_range() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["3-5", path])
        .assert()
        .success()
        .stdout("3\n4\n5\n");
}

#[test]
fn test_comma_as_range_separator() {
    // comma is an alias for dash — both mean inclusive range, not a point-list
    // "3,5" selects lines 3 through 5 inclusive, same as "3-5"
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["3,5", path])
        .assert()
        .success()
        .stdout("3\n4\n5\n");
}

#[test]
fn test_exclusive_start() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["(3-5", path])
        .assert()
        .success()
        .stdout("4\n5\n");
}

#[test]
fn test_exclusive_end() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["3-5)", path])
        .assert()
        .success()
        .stdout("3\n4\n");
}

#[test]
fn test_both_exclusive() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["(3-5)", path])
        .assert()
        .success()
        .stdout("4\n");
}

#[test]
fn test_multi_range() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["2-3;8-9", path])
        .assert()
        .success()
        .stdout("2\n3\n8\n9\n");
}

#[test]
fn test_sed_mode() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["-n", "3,5p", path])
        .assert()
        .success()
        .stdout("3\n4\n5\n");
}

#[test]
fn test_sed_mode_dollar() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["-n", "8,$p", path])
        .assert()
        .success()
        .stdout("8\n9\n10\n");
}

#[test]
fn test_sed_last_line() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["-n", "$p", path])
        .assert()
        .success()
        .stdout("10\n");
}

#[test]
fn test_multi_file() {
    let cfg = isolated_config();
    let f1 = numbered_file(10);
    let f2 = numbered_file(10);
    let path1 = f1.path().to_str().unwrap();
    let path2 = f2.path().to_str().unwrap();
    whitelist_add(&cfg, path1);
    whitelist_add(&cfg, path2);

    ripr(&cfg)
        .args(["2-3", path1, path2])
        .assert()
        .success()
        .stdout("2\n3\n2\n3\n");
}

#[test]
fn test_stdin() {
    let cfg = isolated_config();

    ripr(&cfg)
        .args(["2-3", "-"])
        .write_stdin("a\nb\nc\nd\n")
        .assert()
        .success()
        .stdout("b\nc\n");
}

#[test]
fn test_range_beyond_eof() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["15-20", path])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn test_denied_file_exits_2() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    // Deliberately do NOT whitelist the file

    ripr(&cfg)
        .args(["1-3", path])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("access denied"))
        .stderr(predicate::str::contains("ripr whitelist add"));
}

#[test]
fn test_denied_file_no_stdout() {
    let cfg = isolated_config();
    let f1 = numbered_file(10);
    let f2 = numbered_file(10);
    let path1 = f1.path().to_str().unwrap();
    let path2 = f2.path().to_str().unwrap();
    // Whitelist only the first file; second is denied
    whitelist_add(&cfg, path1);
    // Do NOT whitelist path2

    // Pre-validation should catch path2 before any output is emitted
    ripr(&cfg)
        .args(["2-3", path1, path2])
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("access denied"));
}

#[test]
fn test_bad_range_exits_3() {
    let cfg = isolated_config();
    let f = numbered_file(10);
    let path = f.path().to_str().unwrap();
    whitelist_add(&cfg, path);

    ripr(&cfg)
        .args(["abc-5", path])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("parse error"));
}
