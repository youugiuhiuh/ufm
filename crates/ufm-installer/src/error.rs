use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstallerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid path: {0} ({1})")]
    InvalidPath(PathBuf, String),

    #[error("Target directory not found: {0}")]
    TargetNotFound(PathBuf),

    #[error("Backup failure for {0}: {1}")]
    BackupFailed(PathBuf, String),

    #[error(
        "Transaction failed at step '{step}': {reason}; rollback succeeded: {rollback_succeeded}"
    )]
    TransactionAborted {
        step: String,
        reason: String,
        rollback_succeeded: bool,
    },

    #[error("File ownership not confirmed for {path}: {reason}")]
    OwnershipUnconfirmed { path: PathBuf, reason: String },

    #[error("Manifest error: {0}")]
    ManifestError(String),
}
