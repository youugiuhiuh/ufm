#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use ufm_compat::{filter_candidates, CompatibilityReport};
use ufm_core::EnvironmentReport;
use ufm_detector::detect;
use ufm_diagnostics::{DiagnosticEngine, DiagnosticReportData};
use ufm_download::DownloadManager;
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

    TransactionExecutor::execute(&plan).map_err(|e| format!("执行原子安装事务失败: {e}"))?;

    tracing::info!(
        "Successfully completed atomic install of {} into {}",
        build_id,
        game_path
    );

    Ok(true)
}

#[tauri::command]
fn diagnose_log(
    log_content: String,
    report: Option<EnvironmentReport>,
) -> Result<DiagnosticReportData, String> {
    let engine = DiagnosticEngine::new();
    let mut diag = engine.analyze_with_env(&log_content, report.as_ref());
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
            diagnose_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
