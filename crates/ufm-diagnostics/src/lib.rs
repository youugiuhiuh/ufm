pub mod engine;
pub mod rule;
pub mod rules;
pub mod types;

pub use engine::DiagnosticEngine;
pub use rule::{DiagnosticContext, DiagnosticRule};
pub use rules::{
    BadImageFormatRule, DllNotFoundRule, DoorstopConfigRule, EntryPointNotFoundRule,
    HarmonyPatchRule, MissingMemberRule,
};
pub use types::{DiagnosticIssue, DiagnosticReportData, RiskLevel};

pub fn is_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ufm_core::{Architecture, Confidence, EnvironmentReport, RuntimeType, TargetPlatform};

    #[test]
    fn test_dll_not_found_detection() {
        let log = r#"
[Error  : Unity Log] System.DllNotFoundException: Unable to load DLL 'doorstop.dll': The specified module could not be found.
  at Doorstop.Entry.Init () [0x00000] in <filename unknown>:0
        "#;

        let engine = DiagnosticEngine::new();
        let report = engine.analyze(log);

        assert_eq!(report.total_issues, 1);
        let issue = &report.issues[0];
        assert_eq!(issue.rule_id, "RULE_DLL_NOT_FOUND");
        assert_eq!(issue.risk_level, RiskLevel::Critical); // doorstop is critical
        assert!(issue.description.contains("doorstop.dll"));
        assert!(!issue.suggested_fix.is_empty());
    }

    #[test]
    fn test_bad_image_format_detection() {
        let log = r#"
[Fatal  :   BepInEx] System.BadImageFormatException: An attempt was made to load a program with an incorrect format. (Exception from HRESULT: 0x8007000B)
  at System.Reflection.AssemblyName.nInit ()
        "#;

        let env = EnvironmentReport {
            game_path: "/test/game".into(),
            data_dir: None,
            unity_version: None,
            runtime_type: RuntimeType::Mono,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec![],
            evidences: vec![],
            overall_confidence: Confidence::Confirmed,
        };

        let engine = DiagnosticEngine::new();
        let report = engine.analyze_with_env(log, Some(&env));

        assert_eq!(report.total_issues, 1);
        let issue = &report.issues[0];
        assert_eq!(issue.rule_id, "RULE_BAD_IMAGE_FORMAT");
        assert_eq!(issue.risk_level, RiskLevel::Critical);
        assert!(issue.description.contains("X64"));
        assert!(issue.suggested_fix.contains("x64 vs x86"));
    }

    #[test]
    fn test_entry_point_not_found_detection() {
        let log = r#"
[Error  : Unity Log] System.EntryPointNotFoundException: Unable to find an entry point named 'CustomNativeHook' in DLL 'NativePlugin.dll'.
  at NativePlugin.Wrapper.Hook () [0x00000] in <00000000000000000000000000000000>:0
        "#;

        let engine = DiagnosticEngine::new();
        let report = engine.analyze(log);

        assert_eq!(report.total_issues, 1);
        let issue = &report.issues[0];
        assert_eq!(issue.rule_id, "RULE_ENTRY_POINT_NOT_FOUND");
        assert_eq!(issue.risk_level, RiskLevel::High);
    }

    #[test]
    fn test_doorstop_config_detection() {
        let log = r#"
[Info   : Doorstop] [UnityDoorstop] enabled=false
[Warning: Doorstop] Doorstop is disabled by configuration file.
        "#;

        let engine = DiagnosticEngine::new();
        let report = engine.analyze(log);

        assert_eq!(report.total_issues, 1);
        let issue = &report.issues[0];
        assert_eq!(issue.rule_id, "RULE_DOORSTOP_CONFIG");
        assert_eq!(issue.risk_level, RiskLevel::High);
        assert!(issue.description.contains("disabled in doorstop.ini"));
        assert!(issue.suggested_fix.contains("enabled=true"));
    }

    #[test]
    fn test_harmony_and_missing_member() {
        let log = r#"
[Error  : Harmony] HarmonyLib.PatchException: PatchProcessor failed for SomeMod.PatchClass
[Error  : Unity Log] System.MissingMethodException: Method not found: UnityEngine.GameObject.get_transform_fast
        "#;

        let engine = DiagnosticEngine::new();
        let report = engine.analyze(log);

        assert_eq!(report.total_issues, 2);
        let rule_ids: Vec<&str> = report.issues.iter().map(|i| i.rule_id.as_str()).collect();
        assert!(rule_ids.contains(&"RULE_HARMONY_PATCH_FAILURE"));
        assert!(rule_ids.contains(&"RULE_MISSING_MEMBER"));
    }

    #[test]
    fn test_clean_log() {
        let log = r#"
[Info   :   BepInEx] Loading plugins...
[Info   :   BepInEx] 5 plugins loaded successfully.
Game engine started normally.
        "#;

        let engine = DiagnosticEngine::new();
        let report = engine.analyze(log);

        assert_eq!(report.total_issues, 0);
        assert_eq!(report.max_risk, RiskLevel::Info);
    }
}
