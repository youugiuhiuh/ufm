use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::InstallerError;
use crate::manifest::{BackupRecord, InstallManifest, InstalledFileRecord, InstalledFileType};
use crate::plan::{BackupEntry, InstallPlan};

pub struct TransactionExecutor;

impl TransactionExecutor {
    pub fn execute(plan: &InstallPlan) -> Result<InstallManifest, InstallerError> {
        if !plan.target_dir.exists() {
            return Err(InstallerError::TargetNotFound(plan.target_dir.clone()));
        }

        // Stage 1: Execute backups
        let mut completed_backups: Vec<BackupEntry> = Vec::new();
        for backup in &plan.backups {
            let src = plan.target_dir.join(&backup.relative_path);
            if !src.exists() {
                // If a file was supposed to exist for backup but disappeared
                rollback_backups(&completed_backups);
                return Err(InstallerError::BackupFailed(
                    src,
                    "Source file expected for backup was not found".into(),
                ));
            }

            if let Some(parent) = backup.backup_destination.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    rollback_backups(&completed_backups);
                    return Err(InstallerError::BackupFailed(
                        backup.backup_destination.clone(),
                        format!("Failed to create backup directory: {}", e),
                    ));
                }
            }

            if let Err(e) = std::fs::copy(&src, &backup.backup_destination) {
                rollback_backups(&completed_backups);
                return Err(InstallerError::BackupFailed(
                    src,
                    format!("Failed to copy file to backup: {}", e),
                ));
            }
            completed_backups.push(backup.clone());
        }

        // Stage 2: Write files (atomic additions and modifications)
        let mut created_files: Vec<PathBuf> = Vec::new();
        let mut overwritten_files: Vec<(PathBuf, PathBuf)> = Vec::new();

        // 2a. Add actions
        for add in &plan.adds {
            let dest = plan.target_dir.join(&add.relative_path);
            if let Err(e) = add.source.write_to(&dest) {
                let rb_ok = rollback_changes(&created_files, &overwritten_files, &plan.target_dir);
                return Err(InstallerError::TransactionAborted {
                    step: format!("Add file '{}'", add.relative_path.display()),
                    reason: e.to_string(),
                    rollback_succeeded: rb_ok,
                });
            }
            created_files.push(dest);
        }

        // 2b. Modify actions
        for modify in &plan.modifies {
            let dest = plan.target_dir.join(&modify.relative_path);
            // Record pair before overwriting
            overwritten_files.push((dest.clone(), modify.backup.backup_destination.clone()));
            if let Err(e) = modify.source.write_to(&dest) {
                let rb_ok = rollback_changes(&created_files, &overwritten_files, &plan.target_dir);
                return Err(InstallerError::TransactionAborted {
                    step: format!("Modify file '{}'", modify.relative_path.display()),
                    reason: e.to_string(),
                    rollback_succeeded: rb_ok,
                });
            }
        }

        // Stage 3: Assemble manifest
        let mut installed_files = Vec::new();
        for add in &plan.adds {
            installed_files.push(InstalledFileRecord {
                relative_path: add.relative_path.clone(),
                file_type: InstalledFileType::Added,
                installed_hash: add.expected_hash.clone(),
                file_size: add.expected_size,
            });
        }
        for modify in &plan.modifies {
            installed_files.push(InstalledFileRecord {
                relative_path: modify.relative_path.clone(),
                file_type: InstalledFileType::Modified,
                installed_hash: modify.new_hash.clone(),
                file_size: modify.new_size,
            });
        }

        let backup_records = plan
            .backups
            .iter()
            .map(|b| BackupRecord {
                relative_path: b.relative_path.clone(),
                backup_path: b.backup_destination.clone(),
                original_hash: b.original_hash.clone(),
                original_size: b.original_size,
            })
            .collect();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let manifest_id = format!(
            "{}-{}-{}",
            plan.package_name, plan.package_version, timestamp
        );

        Ok(InstallManifest {
            manifest_id,
            package_name: plan.package_name.clone(),
            package_version: plan.package_version.clone(),
            target_dir: plan.target_dir.clone(),
            backup_dir: plan.backup_dir.clone(),
            installed_files,
            backup_records,
            timestamp,
        })
    }
}

fn rollback_backups(backups: &[BackupEntry]) {
    for b in backups {
        let _ = std::fs::remove_file(&b.backup_destination);
    }
}

fn rollback_changes(
    created_files: &[PathBuf],
    overwritten_files: &[(PathBuf, PathBuf)],
    target_root: &Path,
) -> bool {
    let mut all_ok = true;

    // 1. Delete created files
    for created in created_files {
        if created.exists() {
            if std::fs::remove_file(created).is_err() {
                all_ok = false;
            } else {
                cleanup_empty_parents(created, target_root);
            }
        }
    }

    // 2. Restore overwritten files from backup
    for (dest, backup_src) in overwritten_files {
        if backup_src.exists() && std::fs::copy(backup_src, dest).is_err() {
            all_ok = false;
        }
    }

    all_ok
}

pub fn cleanup_empty_parents(file_path: &Path, root_dir: &Path) {
    let mut current = file_path.parent();
    while let Some(dir) = current {
        if dir == root_dir || !dir.starts_with(root_dir) {
            break;
        }
        // Only remove if directory is completely empty
        match std::fs::read_dir(dir) {
            Ok(mut entries) => {
                if entries.next().is_none() {
                    if std::fs::remove_dir(dir).is_err() {
                        break;
                    }
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
        current = dir.parent();
    }
}
