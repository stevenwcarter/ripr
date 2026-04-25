use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RipError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("access denied: {0}")]
    AccessDenied(PathBuf),

    #[error("config error: {0}")]
    Config(String),

    #[error("parse error: {0}")]
    Parse(String),
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
