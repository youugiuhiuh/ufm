use std::path::{Component, Path, PathBuf};

use crate::error::InstallerError;
use crate::hash::{hash_file, sha256_bytes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSource {
    Bytes(Vec<u8>),
    SourceFile(PathBuf),
}

impl InstallSource {
    pub fn compute_hash(&self) -> Result<String, InstallerError> {
        match self {
            InstallSource::Bytes(bytes) => Ok(sha256_bytes(bytes)),
            InstallSource::SourceFile(path) => hash_file(path).map_err(InstallerError::Io),
        }
    }

    pub fn size(&self) -> Result<u64, InstallerError> {
        match self {
            InstallSource::Bytes(bytes) => Ok(bytes.len() as u64),
            InstallSource::SourceFile(path) => {
                let meta = std::fs::metadata(path)?;
                Ok(meta.len())
            }
        }
    }

    pub fn write_to(&self, destination: &Path) -> Result<(), InstallerError> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match self {
            InstallSource::Bytes(bytes) => {
                std::fs::write(destination, bytes)?;
            }
            InstallSource::SourceFile(src) => {
                std::fs::copy(src, destination)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddAction {
    pub relative_path: PathBuf,
    pub source: InstallSource,
    pub expected_hash: String,
    pub expected_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupEntry {
    pub relative_path: PathBuf,
    pub backup_destination: PathBuf,
    pub original_hash: String,
    pub original_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifyAction {
    pub relative_path: PathBuf,
    pub source: InstallSource,
    pub new_hash: String,
    pub new_size: u64,
    pub backup: BackupEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    pub package_name: String,
    pub package_version: String,
    pub target_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub adds: Vec<AddAction>,
    pub modifies: Vec<ModifyAction>,
    pub backups: Vec<BackupEntry>,
}

impl InstallPlan {
    pub fn total_actions(&self) -> usize {
        self.adds.len() + self.modifies.len()
    }
}

pub fn validate_relative_path(path: &Path) -> Result<(), InstallerError> {
    if path.as_os_str().is_empty() {
        return Err(InstallerError::InvalidPath(
            path.to_path_buf(),
            "Path cannot be empty".into(),
        ));
    }
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                return Err(InstallerError::InvalidPath(
                    path.to_path_buf(),
                    "Path traversal ('..') is strictly prohibited".into(),
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(InstallerError::InvalidPath(
                    path.to_path_buf(),
                    "Absolute paths are not permitted in relative actions".into(),
                ));
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }
    Ok(())
}

pub struct InstallPlanBuilder {
    package_name: String,
    package_version: String,
    target_dir: PathBuf,
    backup_dir: PathBuf,
    items: Vec<(PathBuf, InstallSource)>,
}

impl InstallPlanBuilder {
    pub fn new<P: Into<PathBuf>, B: Into<PathBuf>>(
        package_name: impl Into<String>,
        package_version: impl Into<String>,
        target_dir: P,
        backup_dir: B,
    ) -> Self {
        Self {
            package_name: package_name.into(),
            package_version: package_version.into(),
            target_dir: target_dir.into(),
            backup_dir: backup_dir.into(),
            items: Vec::new(),
        }
    }

    pub fn add_file_bytes<P: Into<PathBuf>>(
        mut self,
        relative_path: P,
        bytes: Vec<u8>,
    ) -> Result<Self, InstallerError> {
        let p = relative_path.into();
        validate_relative_path(&p)?;
        self.items.push((p, InstallSource::Bytes(bytes)));
        Ok(self)
    }

    pub fn add_file_source<P: Into<PathBuf>, S: Into<PathBuf>>(
        mut self,
        relative_path: P,
        source_path: S,
    ) -> Result<Self, InstallerError> {
        let p = relative_path.into();
        validate_relative_path(&p)?;
        self.items
            .push((p, InstallSource::SourceFile(source_path.into())));
        Ok(self)
    }

    pub fn build(self) -> Result<InstallPlan, InstallerError> {
        let mut adds = Vec::new();
        let mut modifies = Vec::new();
        let mut backups = Vec::new();

        for (rel_path, source) in self.items {
            let target_file = self.target_dir.join(&rel_path);
            let new_hash = source.compute_hash()?;
            let new_size = source.size()?;

            if target_file.is_dir() {
                return Err(InstallerError::InvalidPath(
                    target_file,
                    "Target path is an existing directory, cannot overwrite with a file".into(),
                ));
            } else if target_file.is_file() {
                let orig_hash = hash_file(&target_file)?;
                let orig_size = std::fs::metadata(&target_file)?.len();
                let backup_dest = self.backup_dir.join(&rel_path);

                let backup_entry = BackupEntry {
                    relative_path: rel_path.clone(),
                    backup_destination: backup_dest,
                    original_hash: orig_hash,
                    original_size: orig_size,
                };
                backups.push(backup_entry.clone());

                modifies.push(ModifyAction {
                    relative_path: rel_path,
                    source,
                    new_hash,
                    new_size,
                    backup: backup_entry,
                });
            } else {
                adds.push(AddAction {
                    relative_path: rel_path,
                    source,
                    expected_hash: new_hash,
                    expected_size: new_size,
                });
            }
        }

        Ok(InstallPlan {
            package_name: self.package_name,
            package_version: self.package_version,
            target_dir: self.target_dir,
            backup_dir: self.backup_dir,
            adds,
            modifies,
            backups,
        })
    }
}
