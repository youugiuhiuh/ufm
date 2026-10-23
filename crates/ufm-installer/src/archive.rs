//! 安全的 Zip 归档解压与安装计划分发器，严防 Zip Slip 路径穿越与恶意绝对路径

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use crate::error::InstallerError;
use crate::plan::{validate_relative_path, InstallPlan, InstallPlanBuilder};

pub struct ArchiveExtractor;

impl ArchiveExtractor {
    /// 从原生 Zip 文件中安全提取条目并构造原子安装计划
    pub fn build_plan_from_zip<P: AsRef<Path>, T: AsRef<Path>, B: AsRef<Path>>(
        zip_path: P,
        package_name: &str,
        package_version: &str,
        target_dir: T,
        backup_dir: B,
    ) -> Result<InstallPlan, InstallerError> {
        let file = File::open(zip_path.as_ref())?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| InstallerError::General(format!("无法读取 Zip 压缩包: {e}")))?;

        let mut builder = InstallPlanBuilder::new(
            package_name,
            package_version,
            target_dir.as_ref(),
            backup_dir.as_ref(),
        );

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| InstallerError::General(format!("读取压缩包条目失败: {e}")))?;

            // 忽略纯目录条目
            if entry.is_dir() {
                continue;
            }

            // 安全防护：规范化并严格校验相对路径，防止 Zip Slip
            let raw_name = entry.name();
            let relative_path = PathBuf::from(raw_name);

            // 严格检查是否存在 ../ 逃逸或绝对根路径
            validate_relative_path(&relative_path)?;

            // 读取解压字节
            let mut buffer = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buffer)?;

            builder = builder.add_file_bytes(relative_path, buffer)?;
        }

        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_zip(dest: &Path, files: &[(&str, &[u8])]) {
        let file = File::create(dest).unwrap();
        let mut zip = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (name, content) in files {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn test_build_plan_from_valid_zip() {
        let temp_dir = std::env::temp_dir().join(format!("ufm_zip_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let zip_file = temp_dir.join("test.zip");
        create_test_zip(
            &zip_file,
            &[
                ("winhttp.dll", b"fake_winhttp_proxy"),
                ("BepInEx/core/BepInEx.Core.dll", b"fake_bepinex_core_binary"),
            ],
        );

        let target_dir = temp_dir.join("game");
        let backup_dir = temp_dir.join("backups");
        std::fs::create_dir_all(&target_dir).unwrap();

        // 模拟已存在 winhttp.dll 以便测试自动备份标记
        std::fs::write(target_dir.join("winhttp.dll"), b"old_proxy").unwrap();

        let plan = ArchiveExtractor::build_plan_from_zip(
            &zip_file,
            "bepinex-v6",
            "6.0.0-pre.1",
            &target_dir,
            &backup_dir,
        )
        .unwrap();

        assert_eq!(plan.total_actions(), 2);
        assert_eq!(plan.modifies.len(), 1); // winhttp.dll 需修改/备份
        assert_eq!(plan.adds.len(), 1); // BepInEx.Core.dll 纯新增
        assert_eq!(plan.backups.len(), 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_zip_slip_prevention() {
        let temp_dir = std::env::temp_dir().join(format!("ufm_zip_slip_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let zip_file = temp_dir.join("malicious.zip");
        create_test_zip(&zip_file, &[("../../etc/evil.sh", b"malicious payload")]);

        let target_dir = temp_dir.join("game");
        let backup_dir = temp_dir.join("backups");

        let result = ArchiveExtractor::build_plan_from_zip(
            &zip_file,
            "evil-pack",
            "1.0.0",
            &target_dir,
            &backup_dir,
        );

        assert!(result.is_err(), "必须拒绝带路径穿越的恶意 Zip 文件");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
