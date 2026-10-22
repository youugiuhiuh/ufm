pub mod archive;
pub mod error;
pub mod hash;
pub mod manifest;
pub mod plan;
pub mod transaction;
pub mod uninstall;

pub use archive::ArchiveExtractor;
pub use error::InstallerError;
pub use hash::{hash_file, sha256_bytes};
pub use manifest::{BackupRecord, InstallManifest, InstalledFileRecord, InstalledFileType};
pub use plan::{
    AddAction, BackupEntry, InstallPlan, InstallPlanBuilder, InstallSource, ModifyAction,
};
pub use transaction::TransactionExecutor;
pub use uninstall::{UninstallOptions, UninstallReport, Uninstaller};

pub fn is_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct TempTestDir {
        path: PathBuf,
    }

    impl TempTestDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("ufm_inst_test_{}_{}", name, std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempTestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_plan_generation_adds_and_modifies() {
        let temp = TempTestDir::new("plan_gen");
        let target = temp.path.join("game");
        let backup = temp.path.join("backups");
        fs::create_dir_all(&target).unwrap();

        // Existing file to modify
        let existing_file = target.join("version.txt");
        fs::write(&existing_file, "original_content").unwrap();

        let plan = InstallPlanBuilder::new("test-mod", "1.0.0", &target, &backup)
            .add_file_bytes("version.txt", b"new_content".to_vec())
            .unwrap()
            .add_file_bytes("plugins/test.dll", b"binary_data".to_vec())
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(plan.total_actions(), 2);
        assert_eq!(plan.adds.len(), 1);
        assert_eq!(plan.modifies.len(), 1);
        assert_eq!(plan.backups.len(), 1);

        assert_eq!(
            plan.adds[0].relative_path,
            PathBuf::from("plugins/test.dll")
        );
        assert_eq!(plan.modifies[0].relative_path, PathBuf::from("version.txt"));
        assert_eq!(plan.backups[0].relative_path, PathBuf::from("version.txt"));
    }

    #[test]
    fn test_atomic_transaction_execution() {
        let temp = TempTestDir::new("tx_exec");
        let target = temp.path.join("game");
        let backup = temp.path.join("backups");
        fs::create_dir_all(&target).unwrap();

        let existing_file = target.join("doorstop.ini");
        fs::write(&existing_file, "enabled=false").unwrap();

        let plan = InstallPlanBuilder::new("mod-core", "0.2.0", &target, &backup)
            .add_file_bytes("doorstop.ini", b"enabled=true".to_vec())
            .unwrap()
            .add_file_bytes("BepInEx/core/loader.dll", b"preloader_bytes".to_vec())
            .unwrap()
            .build()
            .unwrap();

        let manifest = TransactionExecutor::execute(&plan).expect("transaction should succeed");

        assert_eq!(manifest.installed_files.len(), 2);
        assert_eq!(manifest.backup_records.len(), 1);

        // Check new file exists
        assert_eq!(
            fs::read_to_string(target.join("doorstop.ini")).unwrap(),
            "enabled=true"
        );
        assert_eq!(
            fs::read(target.join("BepInEx/core/loader.dll")).unwrap(),
            b"preloader_bytes"
        );

        // Check backup exists with original content
        let backup_path = &manifest.backup_records[0].backup_path;
        assert!(backup_path.exists());
        assert_eq!(fs::read_to_string(backup_path).unwrap(), "enabled=false");
    }

    #[test]
    fn test_uninstall_restores_and_removes() {
        let temp = TempTestDir::new("uninstall");
        let target = temp.path.join("game");
        let backup = temp.path.join("backups");
        fs::create_dir_all(&target).unwrap();

        let existing_file = target.join("config.cfg");
        fs::write(&existing_file, "orig_cfg").unwrap();

        let plan = InstallPlanBuilder::new("pkg", "1.0", &target, &backup)
            .add_file_bytes("config.cfg", b"modified_cfg".to_vec())
            .unwrap()
            .add_file_bytes("sub/new_file.txt", b"new_file".to_vec())
            .unwrap()
            .build()
            .unwrap();

        let manifest = TransactionExecutor::execute(&plan).unwrap();
        assert_eq!(fs::read_to_string(&existing_file).unwrap(), "modified_cfg");
        assert!(target.join("sub/new_file.txt").exists());

        // Create an unrelated file that belongs to user
        let user_file = target.join("user_save.sav");
        fs::write(&user_file, "user_data").unwrap();

        let report =
            Uninstaller::uninstall(&manifest, &target, &UninstallOptions::default()).unwrap();
        assert!(report.is_clean_uninstall());
        assert_eq!(report.removed_files.len(), 1);
        assert_eq!(report.restored_files.len(), 1);

        // Target existing file must be restored
        assert_eq!(fs::read_to_string(&existing_file).unwrap(), "orig_cfg");
        // Added file must be removed
        assert!(!target.join("sub/new_file.txt").exists());
        // Unrelated user file must NEVER be touched!
        assert!(user_file.exists());
        assert_eq!(fs::read_to_string(&user_file).unwrap(), "user_data");
    }

    #[test]
    fn test_uninstall_refuses_to_delete_unconfirmed_file() {
        let temp = TempTestDir::new("unconfirmed");
        let target = temp.path.join("game");
        let backup = temp.path.join("backups");
        fs::create_dir_all(&target).unwrap();

        let plan = InstallPlanBuilder::new("pkg", "1.0", &target, &backup)
            .add_file_bytes("mod.dll", b"orig_mod_bytes".to_vec())
            .unwrap()
            .build()
            .unwrap();

        let manifest = TransactionExecutor::execute(&plan).unwrap();

        // User or external tool overwrote mod.dll
        let mod_file = target.join("mod.dll");
        fs::write(&mod_file, "tampered_bytes_from_user").unwrap();

        let report =
            Uninstaller::uninstall(&manifest, &target, &UninstallOptions::default()).unwrap();

        // Must NOT delete the file because ownership cannot be confirmed!
        assert!(!report.is_clean_uninstall());
        assert_eq!(report.skipped_files.len(), 1);
        assert!(mod_file.exists());
        assert_eq!(
            fs::read_to_string(&mod_file).unwrap(),
            "tampered_bytes_from_user"
        );
    }

    #[test]
    fn test_path_traversal_rejection() {
        let temp = TempTestDir::new("traversal");
        let target = temp.path.join("game");
        let backup = temp.path.join("backups");
        fs::create_dir_all(&target).unwrap();

        let builder = InstallPlanBuilder::new("bad", "1.0", &target, &backup);
        let res = builder.add_file_bytes("../escaped.txt", b"evil".to_vec());
        assert!(res.is_err());
    }

    #[test]
    fn test_atomic_rollback_on_failure() {
        let temp = TempTestDir::new("rollback");
        let target = temp.path.join("game");
        let backup = temp.path.join("backups");
        fs::create_dir_all(&target).unwrap();

        let original = target.join("file1.txt");
        fs::write(&original, "keep_me").unwrap();

        let plan = InstallPlanBuilder::new("failing", "1.0", &target, &backup)
            .add_file_bytes("file1.txt", b"overwritten".to_vec())
            .unwrap()
            .add_file_bytes("file_new.txt", b"brand_new".to_vec())
            .unwrap()
            .add_file_bytes("dir_blocking_file", b"blocked".to_vec())
            .unwrap()
            .build()
            .unwrap();

        // After plan is built, create a directory where dir_blocking_file was supposed to be written
        let conflict_dir = target.join("dir_blocking_file");
        fs::create_dir_all(&conflict_dir).unwrap();

        let result = TransactionExecutor::execute(&plan);
        assert!(result.is_err());

        // File1 should have been restored to "keep_me"
        assert_eq!(fs::read_to_string(&original).unwrap(), "keep_me");
        // Newly added file should have been removed
        assert!(!target.join("file_new.txt").exists());
    }

    #[test]
    fn test_manifest_save_and_load() {
        let temp = TempTestDir::new("manifest_io");
        let manifest_path = temp.path.join("sub/manifest.json");
        let manifest = InstallManifest {
            manifest_id: "test-pkg-1.0".into(),
            package_name: "test-pkg".into(),
            package_version: "1.0.0".into(),
            target_dir: PathBuf::from("/game"),
            backup_dir: PathBuf::from("/backup"),
            installed_files: vec![InstalledFileRecord {
                relative_path: PathBuf::from("mod.dll"),
                file_type: InstalledFileType::Added,
                installed_hash: "abcd".into(),
                file_size: 100,
            }],
            backup_records: vec![],
            timestamp: 123456,
        };

        manifest.save_to_file(&manifest_path).unwrap();
        let loaded = InstallManifest::load_from_file(&manifest_path).unwrap();
        assert_eq!(manifest, loaded);
        assert!(loaded.owns_relative_path(&PathBuf::from("mod.dll")));
        assert!(!loaded.owns_relative_path(&PathBuf::from("other.dll")));
    }
}
