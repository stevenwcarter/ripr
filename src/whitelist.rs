use std::path::{Path, PathBuf};

use crate::RipError;
use crate::config::{Config, load_config, save_config};

/// Canonicalize `path`, resolving symlinks. If the path no longer exists on disk,
/// walk up to the nearest existing ancestor, canonicalize that, and re-append the
/// missing components. This preserves the same canonical prefix that was stored
/// when the path existed (e.g. `/private/var` on macOS, `\\?\` on Windows),
/// so `remove()` can still match a stored entry after the path has been deleted.
fn best_effort_canonical(path: &Path) -> String {
    if let Ok(p) = path.canonicalize() {
        return p.to_string_lossy().into_owned();
    }
    let components: Vec<_> = path.components().collect();
    // Try progressively shorter prefixes (longest first) until one exists on disk.
    for i in (0..components.len()).rev() {
        let mut ancestor = PathBuf::new();
        for c in &components[..i] {
            ancestor.push(c);
        }
        if let Ok(cp) = ancestor.canonicalize() {
            let mut suffix = PathBuf::new();
            for c in &components[i..] {
                suffix.push(c);
            }
            return cp.join(suffix).to_string_lossy().into_owned();
        }
    }
    path.to_string_lossy().into_owned()
}

pub struct Whitelist {
    config_path: PathBuf,
    config: Config,
}

impl Whitelist {
    /// Load the whitelist from the given config file path.
    pub fn load(config_path: PathBuf) -> Result<Self, RipError> {
        let config = load_config(&config_path)?;
        Ok(Self {
            config_path,
            config,
        })
    }

    /// Check if `target` is allowed. Canonicalizes `target` (resolves symlinks),
    /// then checks if it starts with any whitelisted entry (also canonicalized,
    /// falling back to the raw string if the entry no longer exists on disk).
    ///
    /// Returns `Ok(())` if the path is whitelisted.
    /// Returns `Err(RipError::AccessDenied(...))` if the path exists but is not whitelisted.
    /// Returns `Err(RipError::Io(ENOENT))` if the path does not exist.
    pub fn check(&self, target: &Path) -> Result<(), RipError> {
        let canonical = match target.canonicalize() {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // The target doesn't exist — this is an I/O error, not an access control decision.
                return Err(RipError::Io(e));
            }
            Err(e) => return Err(RipError::Io(e)),
        };

        for entry in &self.config.paths {
            let entry_path = Path::new(entry)
                .canonicalize()
                .unwrap_or_else(|_| PathBuf::from(entry));
            // starts_with uses component boundaries so "/foo/bar" never matches "/foo/ba"
            if canonical.starts_with(&entry_path) {
                return Ok(());
            }
        }
        // Use the original (non-canonical) path in the error so the user sees
        // what they typed, not the internal canonical form (e.g. no \\?\ prefix
        // on Windows, no /private prefix on macOS). The whitelist add hint
        // is still correct because add() canonicalizes its input.
        Err(RipError::AccessDenied(target.to_path_buf()))
    }

    /// Add a path to the whitelist. Canonicalizes the input path first.
    /// If the path is already in the whitelist, this is a no-op.
    /// Saves the config after adding (staged write: disk is updated before in-memory state).
    pub fn add(&mut self, path: &Path) -> Result<(), RipError> {
        let canonical = path.canonicalize().map_err(RipError::Io)?;
        let canonical_str = canonical.to_string_lossy().into_owned();

        if !self.config.paths.contains(&canonical_str) {
            let mut staged = self.config.paths.clone();
            staged.push(canonical_str);
            staged.sort();
            save_config(
                &self.config_path,
                &Config {
                    paths: staged.clone(),
                },
            )?;
            self.config.paths = staged;
        }

        Ok(())
    }

    /// Remove a path from the whitelist.
    /// Uses best-effort canonicalization so entries for deleted paths can still be removed.
    /// If the path is not in the whitelist, this is a no-op (no error).
    /// Saves the config after removing.
    pub fn remove(&mut self, path: &Path) -> Result<(), RipError> {
        let canonical_str = best_effort_canonical(path);

        let original_len = self.config.paths.len();
        self.config.paths.retain(|p| p != &canonical_str);

        if self.config.paths.len() != original_len {
            save_config(&self.config_path, &self.config)?;
        }

        Ok(())
    }

    /// List all whitelisted paths, one per line (sorted).
    pub fn list(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = self.config.paths.iter().map(|s| s.as_str()).collect();
        paths.sort();
        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_whitelist(config_path: PathBuf) -> Whitelist {
        Whitelist::load(config_path).unwrap()
    }

    #[test]
    fn check_allows_whitelisted_file_exact_match() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("data.csv");
        std::fs::write(&file, "").unwrap();

        let config_path = dir.path().join("config.toml");
        let mut wl = make_whitelist(config_path);
        wl.add(&file).unwrap();

        assert!(wl.check(&file).is_ok());
    }

    #[test]
    fn check_allows_file_inside_whitelisted_directory() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("project");
        std::fs::create_dir(&subdir).unwrap();
        let file = subdir.join("main.rs");
        std::fs::write(&file, "").unwrap();

        let config_path = dir.path().join("config.toml");
        let mut wl = make_whitelist(config_path);
        wl.add(&subdir).unwrap();

        assert!(wl.check(&file).is_ok());
    }

    #[test]
    fn check_denies_file_not_in_whitelist() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("secret.txt");
        std::fs::write(&file, "").unwrap();

        let config_path = dir.path().join("config.toml");
        let wl = make_whitelist(config_path);

        assert!(wl.check(&file).is_err());
    }

    #[test]
    fn add_and_check_round_trip() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("myproject");
        std::fs::create_dir(&subdir).unwrap();
        let file = subdir.join("lib.rs");
        std::fs::write(&file, "").unwrap();

        let config_path = dir.path().join("config.toml");
        let mut wl = make_whitelist(config_path);

        // File inside dir should be denied before we add the dir
        assert!(wl.check(&file).is_err());

        wl.add(&subdir).unwrap();

        // Now it should be allowed
        assert!(wl.check(&file).is_ok());
    }

    #[test]
    fn add_is_idempotent() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("myproject");
        std::fs::create_dir(&subdir).unwrap();

        let config_path = dir.path().join("config.toml");
        let mut wl = make_whitelist(config_path);

        wl.add(&subdir).unwrap();
        wl.add(&subdir).unwrap();

        assert_eq!(wl.config.paths.len(), 1);
    }

    #[test]
    fn remove_removes_path_and_subsequent_check_denies() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("data.csv");
        std::fs::write(&file, "").unwrap();

        let config_path = dir.path().join("config.toml");
        let mut wl = make_whitelist(config_path);

        wl.add(&file).unwrap();
        assert!(wl.check(&file).is_ok());

        wl.remove(&file).unwrap();
        assert!(wl.check(&file).is_err());
    }

    #[test]
    fn remove_nonexistent_path_is_noop() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("data.csv");
        std::fs::write(&file, "").unwrap();

        let other = dir.path().join("other.csv");
        std::fs::write(&other, "").unwrap();

        let config_path = dir.path().join("config.toml");
        let mut wl = make_whitelist(config_path);

        wl.add(&file).unwrap();

        // Remove something not in the list - should be a no-op
        wl.remove(&other).unwrap();

        // file should still be accessible
        assert!(wl.check(&file).is_ok());
        assert_eq!(wl.config.paths.len(), 1);
    }

    #[test]
    fn remove_path_that_no_longer_exists_on_disk() {
        // After a whitelisted directory is deleted, remove() should still work (no-op vs disk,
        // but it removes the entry from the in-memory config and saves).
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let mut wl = Whitelist::load(config_path).unwrap();

        // Add a real path
        let target_dir = dir.path().join("myproject");
        std::fs::create_dir_all(&target_dir).unwrap();
        wl.add(&target_dir).unwrap();
        assert_eq!(wl.list().len(), 1);

        // Delete the directory from disk
        std::fs::remove_dir_all(&target_dir).unwrap();

        // remove() should succeed even though the path no longer exists
        wl.remove(&target_dir).unwrap();
        assert_eq!(wl.list().len(), 0);
    }

    #[test]
    fn list_returns_sorted_paths() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("aaa");
        let b = dir.path().join("bbb");
        let c = dir.path().join("ccc");
        for p in [&a, &b, &c] {
            std::fs::create_dir(p).unwrap();
        }

        let config_path = dir.path().join("config.toml");
        let mut wl = make_whitelist(config_path);

        // Add in reverse order
        wl.add(&c).unwrap();
        wl.add(&a).unwrap();
        wl.add(&b).unwrap();

        let listed = wl.list();
        assert_eq!(listed.len(), 3);
        assert!(listed[0] < listed[1]);
        assert!(listed[1] < listed[2]);
    }
}
