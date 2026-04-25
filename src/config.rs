use std::path::{Path, PathBuf};

use etcetera::{AppStrategy, AppStrategyArgs, choose_app_strategy};
use serde::{Deserialize, Serialize};

use crate::RipError;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub paths: Vec<String>,
}

/// Resolve the config file path using this priority:
/// 1. `--config <path>` argument (passed as `cli_override`)
/// 2. `RIPR_CONFIG` environment variable
/// 3. Platform config dir via etcetera:
///    Linux:   ~/.config/ripr/config.toml
///    macOS:   ~/Library/Application Support/ripr/config.toml
///    Windows: %APPDATA%\ripr\config.toml
pub fn resolve_config_path(cli_override: Option<&Path>) -> Result<PathBuf, RipError> {
    if let Some(p) = cli_override {
        return Ok(p.to_path_buf());
    }

    if let Ok(val) = std::env::var("RIPR_CONFIG")
        && !val.is_empty()
    {
        return Ok(PathBuf::from(val));
    }

    let strategy = choose_app_strategy(AppStrategyArgs {
        top_level_domain: "com".to_string(),
        author: "ripr".to_string(),
        app_name: "ripr".to_string(),
    })
    .map_err(|e| RipError::Config(e.to_string()))?;

    Ok(strategy.config_dir().join("config.toml"))
}

/// Load config from disk. If the file does not exist, return an empty Config (not an error).
pub fn load_config(path: &Path) -> Result<Config, RipError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents).map_err(|e| {
            RipError::Config(format!(
                "failed to parse config at {}: {}",
                path.display(),
                e
            ))
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(RipError::Io(e)),
    }
}

/// Save config to disk. Creates the parent directory if it doesn't exist.
pub fn save_config(path: &Path, config: &Config) -> Result<(), RipError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let contents = toml::to_string(config)
        .map_err(|e| RipError::Config(format!("failed to serialize config: {}", e)))?;
    std::fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_config_nonexistent_returns_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let config = load_config(&path).unwrap();
        assert!(config.paths.is_empty());
    }

    #[test]
    fn load_config_valid_toml_parses_correctly() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"paths = ["/home/user/src/myproject", "/home/user/data.csv"]"#,
        )
        .unwrap();

        let config = load_config(&path).unwrap();
        assert_eq!(config.paths.len(), 2);
        assert_eq!(config.paths[0], "/home/user/src/myproject");
        assert_eq!(config.paths[1], "/home/user/data.csv");
    }

    #[test]
    fn load_config_invalid_toml_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is not valid toml = = =").unwrap();

        let result = load_config(&path);
        assert!(result.is_err());
        match result.unwrap_err() {
            RipError::Config(msg) => assert!(msg.contains("failed to parse config at")),
            other => panic!("expected RipError::Config, got {:?}", other),
        }
    }

    #[test]
    fn save_and_load_config_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("subdir").join("config.toml");

        let config = Config {
            paths: vec![
                "/home/user/src/myproject".to_string(),
                "/home/user/data.csv".to_string(),
            ],
        };

        save_config(&path, &config).unwrap();

        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.paths, config.paths);
    }
}
