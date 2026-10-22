use std::path::Path;
use ufm_compat::filter_candidates;
use ufm_detector::detect;
use ufm_diagnostics::DiagnosticEngine;
use ufm_registry::BepInExRegistry;

#[tokio::main]
async fn main() {
    println!("=== UFM Desktop Host Engine ===");
    let sample = "/run/media/ub/系统/Users/asus/Downloads/Telegram Desktop/3D!かすたむオカズメーカー/3D!かすたむオカズメーカー/GameCore";
    if let Ok(report) = detect(Path::new(sample)) {
        println!("游戏检测就绪: {:?}", report.unity_version);
        let registry = BepInExRegistry::official_catalog();
        let compat = filter_candidates(&report, registry.builds());
        println!("推荐构建: {:?}", compat.recommended_build.map(|b| b.id));
        let engine = DiagnosticEngine::new();
        let diags = engine.analyze_with_env("Log snippet", Some(&report));
        println!("诊断系统就绪，发现潜在问题: {}", diags.total_issues);
    }
}
