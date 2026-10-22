use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::InstallerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstalledFileType {
    Added,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledFileRecord {
    pub relative_path: PathBuf,
    pub file_type: InstalledFileType,
    pub installed_hash: String,
    pub file_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRecord {
    pub relative_path: PathBuf,
    pub backup_path: PathBuf,
    pub original_hash: String,
    pub original_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallManifest {
    pub manifest_id: String,
    pub package_name: String,
    pub package_version: String,
    pub target_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub installed_files: Vec<InstalledFileRecord>,
    pub backup_records: Vec<BackupRecord>,
    pub timestamp: u64,
}

impl InstallManifest {
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), InstallerError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| InstallerError::ManifestError(e.to_string()))?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, InstallerError> {
        let content = std::fs::read_to_string(path)?;
        let manifest = serde_json::from_str(&content)
            .map_err(|e| InstallerError::ManifestError(e.to_string()))?;
        Ok(manifest)
    }

    pub fn owns_relative_path(&self, rel_path: &Path) -> bool {
        self.installed_files
            .iter()
            .any(|f| f.relative_path == rel_path)
    }

    pub fn get_file_record(&self, rel_path: &Path) -> Option<&InstalledFileRecord> {
        self.installed_files
            .iter()
            .find(|f| f.relative_path == rel_path)
    }

    pub fn get_backup_record(&self, rel_path: &Path) -> Option<&BackupRecord> {
        self.backup_records
            .iter()
            .find(|b| b.relative_path == rel_path)
    }
}
