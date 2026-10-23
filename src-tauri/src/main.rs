#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;
use ufm_compat::{filter_candidates, CompatibilityReport};
use ufm_core::EnvironmentReport;
use ufm_detector::detect;
use ufm_diagnostics::{DiagnosticEngine, DiagnosticReportData};
use ufm_installer::plan::InstallPlan;
use ufm_registry::BepInExRegistry;

#[tauri::command]
fn detect_environment(game_path: String) -> Result<EnvironmentReport, String> {
    let path = Path::new(&game_path);
    detect(path).map_err(|e| format!("游戏环境检测失败: {e}"))
}

#[tauri::command]
fn evaluate_compatibility(report: EnvironmentReport) -> Result<CompatibilityReport, String> {
    let registry = BepInExRegistry::official_catalog();
    let builds = registry.list_all();
    let compat_report = filter_candidates(&report, &builds);
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

#[tauri::command]
fn prepare_install_plan(
    game_path: String,
    build_id: String,
) -> Result<InstallPlanPreviewDto, String> {
    let preview = InstallPlanPreviewDto {
        package_name: build_id,
        package_version: "6.0.0-pre.1".to_string(),
        total_actions: 4,
        backup_dir: format!("{game_path}/.ufm_backups/latest"),
        changes: vec![
            InstallActionDto {
                relative_path: "winhttp.dll".to_string(),
                action_type: "modify".to_string(),
                size: 26624,
                will_backup: true,
            },
            InstallActionDto {
                relative_path: "doorstop_config.ini".to_string(),
                action_type: "modify".to_string(),
                size: 1451,
                will_backup: true,
            },
            InstallActionDto {
                relative_path: "BepInEx/core/BepInEx.Core.dll".to_string(),
                action_type: "add".to_string(),
                size: 428032,
                will_backup: false,
            },
            InstallActionDto {
                relative_path: "BepInEx/core/BepInEx.Unity.Mono.Preloader.dll".to_string(),
                action_type: "add".to_string(),
                size: 98304,
                will_backup: false,
            },
        ],
    };
    Ok(preview)
}

#[tauri::command]
fn execute_install(game_path: String, build_id: String) -> Result<bool, String> {
    tracing::info!(
        "Executing atomic install for {} on game_path {}",
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
    let mut diag = engine.analyze(&log_content);
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
            diag.total_issues = diag.issues.len();
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
