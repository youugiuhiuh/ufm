#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use ufm_compat::{filter_candidates, CompatibilityReport};
use ufm_core::EnvironmentReport;
use ufm_detector::detect;
use ufm_diagnostics::{DiagnosticEngine, DiagnosticReportData};
use ufm_download::DownloadManager;
use ufm_installer::manifest::InstallManifest;
use ufm_installer::uninstall::{UninstallOptions, Uninstaller};
use ufm_installer::{ArchiveExtractor, TransactionExecutor};
use ufm_registry::BepInExRegistry;

#[tauri::command]
fn detect_environment(game_path: String) -> Result<EnvironmentReport, String> {
    let path = Path::new(&game_path);
    detect(path).map_err(|e| format!("游戏环境检测失败: {e}"))
}

#[tauri::command]
fn evaluate_compatibility(report: EnvironmentReport) -> Result<CompatibilityReport, String> {
    let registry = BepInExRegistry::official_catalog();
    let builds = registry.builds();
    let compat_report = filter_candidates(&report, builds);
    Ok(compat_report)
}

#[derive(serde::Serialize)]
struct InstallPlanPreviewDto {
    package_name: String,
    package_version: String,
    total_actions: usize,
    backup_dir: String,
    changes: Vec<InstallActionDto>,
}

#[derive(serde::Serialize)]
struct InstallActionDto {
    relative_path: String,
    action_type: String,
    size: u64,
    will_backup: bool,
}

fn get_history_dir() -> PathBuf {
    std::env::temp_dir().join("ufm_history")
}

fn get_cache_dir() -> PathBuf {
    std::env::temp_dir().join("ufm_cache")
}

fn make_backup_dir(target_dir: &Path) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    target_dir.join(".ufm_backups").join(format!("backup_{ts}"))
}

async fn download_build_zip(build_id: &str) -> Result<(String, String, PathBuf), String> {
    let registry = BepInExRegistry::official_catalog();
    let build = registry
        .get(build_id)
        .ok_or_else(|| format!("未在目录库中找到目标构建: {build_id}"))?;

    let cache_dir = get_cache_dir();
    let downloader =
        DownloadManager::new(&cache_dir).map_err(|e| format!("初始化下载缓存目录失败: {e}"))?;

    let filename = build
        .filename
        .clone()
        .unwrap_or_else(|| format!("{build_id}.zip"));

    let zip_path = downloader
        .fetch_with_verify(&build.download_url, &build.sha256, &filename)
        .await
        .map_err(|e| format!("下载或加载构建包失败: {e}"))?;

    Ok((build.id.clone(), build.version.clone(), zip_path))
}

#[tauri::command]
async fn prepare_install_plan(
    game_path: String,
    build_id: String,
) -> Result<InstallPlanPreviewDto, String> {
    let (pkg_name, pkg_ver, zip_path) = download_build_zip(&build_id).await?;
    let target_dir = Path::new(&game_path);
    if !target_dir.exists() {
        return Err(format!("游戏目录不存在: {game_path}"));
    }

    let backup_dir = make_backup_dir(target_dir);
    let plan = ArchiveExtractor::build_plan_from_zip(
        &zip_path,
        &pkg_name,
        &pkg_ver,
        target_dir,
        &backup_dir,
    )
    .map_err(|e| format!("解压构建包并生成安装计划失败: {e}"))?;

    let mut changes = Vec::new();
    for m in &plan.modifies {
        changes.push(InstallActionDto {
            relative_path: m.relative_path.to_string_lossy().to_string(),
            action_type: "modify".to_string(),
            size: m.new_size,
            will_backup: true,
        });
    }
    for a in &plan.adds {
        changes.push(InstallActionDto {
            relative_path: a.relative_path.to_string_lossy().to_string(),
            action_type: "add".to_string(),
            size: a.expected_size,
            will_backup: false,
        });
    }

    let total_actions = plan.total_actions();
    Ok(InstallPlanPreviewDto {
        package_name: plan.package_name,
        package_version: plan.package_version,
        total_actions,
        backup_dir: backup_dir.to_string_lossy().to_string(),
        changes,
    })
}

#[tauri::command]
async fn execute_install(game_path: String, build_id: String) -> Result<bool, String> {
    let (pkg_name, pkg_ver, zip_path) = download_build_zip(&build_id).await?;
    let target_dir = Path::new(&game_path);
    if !target_dir.exists() {
        return Err(format!("游戏目录不存在: {game_path}"));
    }

    let backup_dir = make_backup_dir(target_dir);
    let plan = ArchiveExtractor::build_plan_from_zip(
        &zip_path,
        &pkg_name,
        &pkg_ver,
        target_dir,
        &backup_dir,
    )
    .map_err(|e| format!("解压构建包失败: {e}"))?;

    let manifest =
        TransactionExecutor::execute(&plan).map_err(|e| format!("执行原子安装事务失败: {e}"))?;

    // 保存清单到备份目录与全局历史记录
    let _ = manifest.save_to_file(backup_dir.join("manifest.json"));
    let history_dir = get_history_dir();
    let _ = std::fs::create_dir_all(&history_dir);
    let _ = manifest.save_to_file(history_dir.join(format!("{}.json", manifest.manifest_id)));
    let _ = manifest.save_to_file(
        target_dir
            .join(".ufm_backups")
            .join(format!("manifest_{}.json", manifest.manifest_id)),
    );

    tracing::info!(
        "Successfully completed atomic install of {} into {}",
        build_id,
        game_path
    );

    Ok(true)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct InstallHistoryItemDto {
    manifest_id: String,
    package_name: String,
    package_version: String,
    timestamp: u64,
    target_dir: String,
    backup_dir: String,
    installed_files_count: usize,
    backup_records_count: usize,
    can_rollback: bool,
}

#[tauri::command]
fn get_install_history(game_path: Option<String>) -> Result<Vec<InstallHistoryItemDto>, String> {
    let mut items = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    let mut search_dirs = Vec::new();
    if let Some(ref gp) = game_path {
        let p = Path::new(gp);
        search_dirs.push(p.join(".ufm_backups"));
    }
    search_dirs.push(get_history_dir());

    for dir in search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(manifest) = InstallManifest::load_from_file(&path) {
                        if seen_ids.insert(manifest.manifest_id.clone()) {
                            let can_rollback =
                                manifest.backup_dir.is_dir() || manifest.backup_records.is_empty();
                            items.push(InstallHistoryItemDto {
                                manifest_id: manifest.manifest_id,
                                package_name: manifest.package_name,
                                package_version: manifest.package_version,
                                timestamp: manifest.timestamp,
                                target_dir: manifest.target_dir.to_string_lossy().to_string(),
                                backup_dir: manifest.backup_dir.to_string_lossy().to_string(),
                                installed_files_count: manifest.installed_files.len(),
                                backup_records_count: manifest.backup_records.len(),
                                can_rollback,
                            });
                        }
                    }
                } else if path.is_dir() {
                    let sub_manifest = path.join("manifest.json");
                    if sub_manifest.is_file() {
                        if let Ok(manifest) = InstallManifest::load_from_file(&sub_manifest) {
                            if seen_ids.insert(manifest.manifest_id.clone()) {
                                let can_rollback = manifest.backup_dir.is_dir()
                                    || manifest.backup_records.is_empty();
                                items.push(InstallHistoryItemDto {
                                    manifest_id: manifest.manifest_id,
                                    package_name: manifest.package_name,
                                    package_version: manifest.package_version,
                                    timestamp: manifest.timestamp,
                                    target_dir: manifest.target_dir.to_string_lossy().to_string(),
                                    backup_dir: manifest.backup_dir.to_string_lossy().to_string(),
                                    installed_files_count: manifest.installed_files.len(),
                                    backup_records_count: manifest.backup_records.len(),
                                    can_rollback,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(items)
}

#[tauri::command]
fn rollback_installation(manifest_id: String, game_path: String) -> Result<bool, String> {
    let target_dir = Path::new(&game_path);
    if !target_dir.exists() {
        return Err(format!("游戏目录不存在: {game_path}"));
    }

    let mut manifest_path = None;
    let candidates = [
        target_dir
            .join(".ufm_backups")
            .join(format!("manifest_{manifest_id}.json")),
        get_history_dir().join(format!("{manifest_id}.json")),
    ];

    for c in candidates {
        if c.is_file() {
            manifest_path = Some(c);
            break;
        }
    }

    if manifest_path.is_none() {
        if let Ok(entries) = std::fs::read_dir(target_dir.join(".ufm_backups")) {
            for entry in entries.flatten() {
                let sub = entry.path().join("manifest.json");
                if sub.is_file() {
                    if let Ok(m) = InstallManifest::load_from_file(&sub) {
                        if m.manifest_id == manifest_id {
                            manifest_path = Some(sub);
                            break;
                        }
                    }
                }
            }
        }
    }

    let m_path =
        manifest_path.ok_or_else(|| format!("未找到清单 ID 为 {manifest_id} 的备份记录"))?;
    let manifest =
        InstallManifest::load_from_file(&m_path).map_err(|e| format!("读取清单失败: {e}"))?;

    let options = UninstallOptions {
        strict_ownership_check: false,
        restore_backups: true,
        remove_empty_dirs: true,
    };

    let report = Uninstaller::uninstall(&manifest, target_dir, &options)
        .map_err(|e| format!("回滚事务失败: {e}"))?;

    tracing::info!(
        "Rollback completed: removed {} files, restored {} files",
        report.removed_files.len(),
        report.restored_files.len()
    );

    Ok(true)
}

#[tauri::command]
fn diagnose_log(
    game_path: Option<String>,
    log_content: Option<String>,
    report: Option<EnvironmentReport>,
) -> Result<DiagnosticReportData, String> {
    let mut combined_logs = String::new();

    // 1. 如果用户手动传入了日志内容，优先加入
    if let Some(content) = log_content {
        if !content.trim().is_empty() {
            combined_logs.push_str(&content);
            combined_logs.push('\n');
        }
    }

    // 2. 如果提供了游戏路径，自动扫描并提取常见的 Unity / BepInEx 日志与配置文件
    let target_game_path = game_path.or_else(|| report.as_ref().map(|r| r.game_path.clone()));
    if let Some(gp) = target_game_path {
        let root = Path::new(&gp);
        let candidate_files = [
            root.join("BepInEx").join("LogOutput.log"),
            root.join("doorstop_latest.log"),
            root.join("doorstop.log"),
            root.join("MelonLoader").join("Latest.log"),
            root.join("output_log.txt"),
            root.join("doorstop_config.ini"),
        ];

        for f in candidate_files {
            if f.is_file() {
                if let Ok(c) = std::fs::read_to_string(&f) {
                    combined_logs.push_str(&format!("\n--- File: {} ---\n", f.display()));
                    let tail: Vec<&str> = c.lines().rev().take(500).collect();
                    for line in tail.into_iter().rev() {
                        combined_logs.push_str(line);
                        combined_logs.push('\n');
                    }
                }
            }
        }
    }

    let engine = DiagnosticEngine::new();
    let mut diag = engine.analyze_with_env(&combined_logs, report.as_ref());
    if let Some(r) = report {
        let has_melon = r.installed_frameworks.iter().any(|f| f == "MelonLoader");
        let has_bepinex = r.installed_frameworks.iter().any(|f| f == "BepInEx");
        if has_melon && has_bepinex {
            diag.issues.push(ufm_diagnostics::DiagnosticIssue {
                rule_id: "LOADER_CONFLICT_MELON_BEPINEX".to_string(),
                title: "MelonLoader 与 BepInEx 混装共存".to_string(),
                risk_level: ufm_diagnostics::RiskLevel::Medium,
                description: "目录中同时检测到 MelonLoader 和 BepInEx 运行库，两者可能竞争劫持 Unity 运行时导致崩溃。".to_string(),
                matched_lines: vec!["Found MelonLoader runtime".to_string(), "Found BepInEx runtime".to_string()],
                suggested_fix: "建议仅保留一套框架。若使用 BepInEx，请移除或备份重命名 MelonLoader 目录。".to_string(),
                references: vec!["https://bepinex.github.io/bepinex_docs/master/articles/advanced/troubleshooting.html".to_string()],
            });
            diag = DiagnosticReportData::new(diag.issues);
        }
    }
    Ok(diag)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            detect_environment,
            evaluate_compatibility,
            prepare_install_plan,
            execute_install,
            diagnose_log,
            get_install_history,
            rollback_installation
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
