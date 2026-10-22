use std::path::Path;
use ufm_compat::filter_candidates;
use ufm_detector::detect;
use ufm_registry::BepInExRegistry;

fn main() {
    let target_path = "/run/media/ub/系统/Users/asus/Downloads/Telegram Desktop/3D!かすたむオカズメーカー/3D!かすたむオカズメーカー/GameCore";
    println!("=== 1. UFM 核心引擎正在扫描路径 ===");
    println!("路径: {}\n", target_path);

    let report = match detect(Path::new(target_path)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("检测失败: {:?}", e);
            return;
        }
    };

    println!("=== 2. 环境检测结果 (EnvironmentReport) ===");
    println!("游戏根目录: {}", report.game_path);
    println!("数据目录: {:?}", report.data_dir);
    println!(
        "Unity 版本: {}",
        report
            .unity_version
            .as_ref()
            .map(|v| v.raw.as_str())
            .unwrap_or("未知")
    );
    println!("运行时类型: {:?}", report.runtime_type);
    println!("程序架构: {:?}", report.architecture);
    println!("目标平台: {:?}", report.target_platform);
    println!("已安装框架: {:?}", report.installed_frameworks);
    println!("综合置信度: {:?}\n", report.overall_confidence);

    println!("=== 3. 详细检测证据链 (Evidences) ===");
    for (i, ev) in report.evidences.iter().enumerate() {
        println!(
            "[{}] {:?} | {} | 描述: {} | 路径: {:?}",
            i + 1,
            ev.confidence,
            ev.key,
            ev.description,
            ev.path
        );
    }
    println!();

    println!("=== 4. BepInEx 6 官方构建兼容性推荐与匹配 ===");
    let registry = BepInExRegistry::official_catalog();
    let compat_report = filter_candidates(&report, registry.builds());

    println!("全局警告:");
    if compat_report.global_warnings.is_empty() {
        println!("  (无)");
    } else {
        for w in &compat_report.global_warnings {
            println!("  - [{}] {}", w.code, w.message);
        }
    }

    println!("\n推荐构建:");
    if let Some(recommended) = &compat_report.recommended_build {
        println!("  ★ 最佳推荐构建: {}", recommended.id);
        println!("    - 版本: {}", recommended.version);
        println!("    - 渠道: {:?}", recommended.channel);
        println!("    - 目标运行时: {:?}", recommended.runtime_type);
        println!("    - 目标架构: {:?}", recommended.architecture);
        println!("    - 目标平台: {:?}", recommended.target_platform);
        println!("    - 下载 URL: {}", recommended.download_url);
    } else {
        println!("  未找到完全匹配的推荐构建！");
    }

    println!("\n候选评估列表 (总计 {} 项):", compat_report.assessments.len());
    for a in &compat_report.assessments {
        println!("  - 构建: {} (状态: {:?})", a.build.id, a.status);
        for w in &a.warnings {
            println!("      警告 [{}]: {}", w.code, w.message);
        }
        for r in &a.reasons {
            println!("      依据 [{}:{:?}]: {}", r.aspect, r.state, r.detail);
        }
    }
}
