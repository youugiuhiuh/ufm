use std::path::{Path, PathBuf};

use crate::error::InstallerError;
use crate::hash::hash_file;
use crate::manifest::{InstallManifest, InstalledFileType};
use crate::plan::validate_relative_path;
use crate::transaction::cleanup_empty_parents;

#[derive(Debug, Clone)]
pub struct UninstallOptions {
    /// In strict mode, files that were modified externally (hash mismatch)
    /// will NOT be deleted to prevent accidental data loss.
    pub strict_ownership_check: bool,
    /// Whether to restore original files from backup if they were modified.
    pub restore_backups: bool,
    /// Whether to clean up empty parent directories left behind.
    pub remove_empty_dirs: bool,
}

impl Default for UninstallOptions {
    fn default() -> Self {
        Self {
            strict_ownership_check: true,
            restore_backups: true,
            remove_empty_dirs: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UninstallReport {
    pub package_name: String,
    pub removed_files: Vec<PathBuf>,
    pub restored_files: Vec<PathBuf>,
    pub skipped_files: Vec<(PathBuf, String)>,
    pub cleaned_directories: Vec<PathBuf>,
}

impl UninstallReport {
    pub fn is_clean_uninstall(&self) -> bool {
        self.skipped_files.is_empty()
    }
}

pub struct Uninstaller;

impl Uninstaller {
    /// Uninstalls an installation strictly according to its manifest.
    /// Files that are not confirmed to belong to this installation are NEVER deleted.
    pub fn uninstall(
        manifest: &InstallManifest,
        target_dir: &Path,
        options: &UninstallOptions,
    ) -> Result<UninstallReport, InstallerError> {
        let mut report = UninstallReport {
            package_name: manifest.package_name.clone(),
            ..Default::default()
        };

        if !target_dir.exists() {
            return Err(InstallerError::TargetNotFound(target_dir.to_path_buf()));
        }

        for record in &manifest.installed_files {
            // Guard against path traversal
            if let Err(e) = validate_relative_path(&record.relative_path) {
                report.skipped_files.push((
                    record.relative_path.clone(),
                    format!("Path validation failed: {}", e),
                ));
                continue;
            }

            let file_path = target_dir.join(&record.relative_path);
            if !file_path.exists() {
                // File already does not exist
                continue;
            }

            // Verify file ownership via hash check
            if options.strict_ownership_check {
                match hash_file(&file_path) {
                    Ok(current_hash) => {
                        if current_hash != record.installed_hash {
                            report.skipped_files.push((
                                file_path,
                                format!(
                                    "Ownership not confirmed: file hash changed from {} to {}",
                                    record.installed_hash, current_hash
                                ),
                            ));
                            continue;
                        }
                    }
                    Err(e) => {
                        report
                            .skipped_files
                            .push((file_path, format!("Failed to calculate hash: {}", e)));
                        continue;
                    }
                }
            }

            // At this point ownership is confirmed!
            match record.file_type {
                InstalledFileType::Added => {
                    // This was a newly created file. Safe to remove.
                    match std::fs::remove_file(&file_path) {
                        Ok(()) => {
                            if options.remove_empty_dirs {
                                cleanup_empty_parents(&file_path, target_dir);
                            }
                            report.removed_files.push(file_path);
                        }
                        Err(e) => {
                            report
                                .skipped_files
                                .push((file_path, format!("Failed to delete added file: {}", e)));
                        }
                    }
                }
                InstalledFileType::Modified => {
                    // This was an overwritten file. Check backup.
                    if options.restore_backups {
                        if let Some(backup) = manifest.get_backup_record(&record.relative_path) {
                            if backup.backup_path.exists() {
                                // Restore original file
                                match std::fs::copy(&backup.backup_path, &file_path) {
                                    Ok(_) => {
                                        let _ = std::fs::remove_file(&backup.backup_path);
                                        report.restored_files.push(file_path);
                                    }
                                    Err(e) => {
                                        report.skipped_files.push((
                                            file_path,
                                            format!("Failed to restore from backup: {}", e),
                                        ));
                                    }
                                }
                            } else {
                                report.skipped_files.push((
                                    file_path,
                                    format!(
                                        "Backup file '{}' is missing; kept current file",
                                        backup.backup_path.display()
                                    ),
                                ));
                            }
                        } else {
                            report.skipped_files.push((
                                file_path,
                                "No backup record found in manifest for modified file".into(),
                            ));
                        }
                    }
                }
            }
        }

        // Clean up empty backup directory if empty
        if manifest.backup_dir.exists() {
            cleanup_empty_parents(
                &manifest.backup_dir.join("dummy"),
                manifest.backup_dir.parent().unwrap_or(&manifest.backup_dir),
            );
        }

        Ok(report)
    }
}
