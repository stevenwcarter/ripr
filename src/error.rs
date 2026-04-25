use std::path::PathBuf;

#[derive(Debug)]
pub enum RipError {
    Io(std::io::Error),
    AccessDenied(PathBuf),
    Config(String),
    Parse(String),
}

impl std::error::Error for RipError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RipError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for RipError {
    fn from(e: std::io::Error) -> Self {
        RipError::Io(e)
    }
}

impl std::fmt::Display for RipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RipError::Io(e) => write!(f, "I/O error: {e}"),
            RipError::AccessDenied(path) => {
                let s = path.to_string_lossy();
                // POSIX single-quoting: replace each ' with '\''
                let quoted = format!("'{}'", s.replace('\'', r"'\''"));
                write!(
                    f,
                    "access denied: {}\n      \u{2192} to allow access, run: ripr whitelist add {}",
                    path.display(),
                    quoted
                )
            }
            RipError::Config(msg) => write!(f, "config error: {msg}"),
            RipError::Parse(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl RipError {
    /// Map the error to a process exit code.
    ///
    /// | Code | Meaning          |
    /// |------|------------------|
    /// | 0    | success (unused) |
    /// | 1    | I/O error        |
    /// | 2    | access denied    |
    /// | 3    | config / parse   |
    pub fn exit_code(&self) -> i32 {
        match self {
            RipError::Io(_) => 1,
            RipError::AccessDenied(_) => 2,
            RipError::Config(_) | RipError::Parse(_) => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn access_denied_display_uses_single_quotes() {
        let path = PathBuf::from("/home/user/my project/file.txt");
        let err = RipError::AccessDenied(path);
        let msg = err.to_string();
        assert!(msg.contains("access denied: /home/user/my project/file.txt"));
        assert!(msg.contains("ripr whitelist add '/home/user/my project/file.txt'"));
        // Ensure double-quotes are NOT used (shell injection risk)
        assert!(!msg.contains('"'));
    }

    #[test]
    fn access_denied_display_escapes_single_quotes_in_path() {
        let path = PathBuf::from("/home/user/it's a trap/file.txt");
        let err = RipError::AccessDenied(path);
        let msg = err.to_string();
        // Single quotes in path must be escaped using '\'' POSIX idiom
        assert!(msg.contains(r"it'\''s"));
    }
}
