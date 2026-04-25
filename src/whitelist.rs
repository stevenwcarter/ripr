use std::path::{Path, PathBuf};

use crate::RipError;
use crate::config::{Config, load_config, save_config};

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
    /// then checks if it matches any whitelisted entry:
    /// - Directory entry: target starts with the whitelisted dir path
    /// - File entry: target equals the whitelisted path exactly
    ///
    /// Returns Err(RipError::AccessDenied(target)) if not allowed.
    pub fn check(&self, target: &Path) -> Result<(), RipError> {
        let canonical = target
            .canonicalize()
            .map_err(|_| RipError::AccessDenied(target.to_path_buf()))?;
        for entry in &self.config.paths {
            let entry_path = Path::new(entry);
            if entry_path.is_dir() {
                // Directory: target must start with the whitelisted dir
                if canonical.starts_with(entry_path) {
                    return Ok(());
                }
            } else {
                // File (or unknown): exact match
                if canonical == entry_path {
                    return Ok(());
                }
            }
        }
        Err(RipError::AccessDenied(canonical))
    }

    /// Add a path to the whitelist. Canonicalizes the input path first.
    /// If the path is already in the whitelist, this is a no-op.
    /// Saves the config after adding.
    pub fn add(&mut self, path: &Path) -> Result<(), RipError> {
        let canonical = path.canonicalize().map_err(RipError::Io)?;
        let canonical_str = canonical.to_string_lossy().into_owned();

        if !self.config.paths.contains(&canonical_str) {
            self.config.paths.push(canonical_str);
            self.config.paths.sort();
            save_config(&self.config_path, &self.config)?;
        }

        Ok(())
    }

    /// Remove a path from the whitelist. Canonicalizes the input path first.
    /// If the path is not in the whitelist, this is a no-op (no error).
    /// Saves the config after removing.
    pub fn remove(&mut self, path: &Path) -> Result<(), RipError> {
        let canonical = path.canonicalize().map_err(RipError::Io)?;
        let canonical_str = canonical.to_string_lossy().into_owned();

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
