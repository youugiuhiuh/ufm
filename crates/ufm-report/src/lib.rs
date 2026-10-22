pub mod error;
pub mod exporter;
pub mod markdown;
pub mod redactor;

pub use error::ReportError;
pub use exporter::ReportExporter;
pub use markdown::{
    render_combined_markdown, render_diagnostics_markdown, render_environment_markdown,
};
pub use redactor::{
    redact_ipv4_addresses, redact_tokens_and_secrets, redact_usernames_in_paths, Redactor,
};

pub fn is_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ufm_core::{
        Architecture, Confidence, EnvironmentReport, EvidenceItem, RuntimeType, TargetPlatform,
        UnityVersionInfo,
    };
    use ufm_diagnostics::{DiagnosticIssue, DiagnosticReportData, RiskLevel};

    fn sample_environment() -> EnvironmentReport {
        EnvironmentReport {
            game_path: r"C:\Users\SecretUser\Games\MyUnityGame".into(),
            data_dir: Some(r"C:\Users\SecretUser\Games\MyUnityGame\MyGame_Data".into()),
            unity_version: Some(UnityVersionInfo {
                raw: "2021.3.16f1".into(),
                major: 2021,
                minor: 3,
                patch: 16,
                suffix: Some("f1".into()),
            }),
            runtime_type: RuntimeType::Mono,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec!["BepInEx-5.4.21".into()],
            evidences: vec![EvidenceItem {
                key: "managed_dir".into(),
                description: "Managed folder found".into(),
                confidence: Confidence::Confirmed,
                path: Some(r"C:\Users\SecretUser\Games\MyUnityGame\MyGame_Data\Managed".into()),
            }],
            overall_confidence: Confidence::Confirmed,
        }
    }

    fn sample_diagnostics() -> DiagnosticReportData {
        DiagnosticReportData::new(vec![DiagnosticIssue {
            rule_id: "RULE_DLL_NOT_FOUND".into(),
            title: "Missing DLL".into(),
            risk_level: RiskLevel::High,
            description: "Unable to load DLL at /home/developer/games/mod.so with token=mysecret123 on 192.168.1.50".into(),
            matched_lines: vec![
                "DllNotFoundException: /home/developer/games/mod.so".into(),
                "Debug token=mysecret123 ip=192.168.1.50".into(),
            ],
            suggested_fix: "Install required dependency".into(),
            references: vec!["https://example.com/fix".into()],
        }])
    }

    #[test]
    fn test_environment_json_redaction() {
        let env = sample_environment();
        let exporter = ReportExporter::new();

        let json_redacted = exporter.export_environment_json(&env, true).unwrap();
        assert!(!json_redacted.contains("SecretUser"));
        assert!(json_redacted.contains("[USERNAME]"));

        let json_unredacted = exporter.export_environment_json(&env, false).unwrap();
        assert!(json_unredacted.contains("SecretUser"));
    }

    #[test]
    fn test_environment_markdown_redaction() {
        let env = sample_environment();
        let exporter = ReportExporter::new();

        let md_redacted = exporter.export_environment_markdown(&env, true).unwrap();
        assert!(!md_redacted.contains("SecretUser"));
        assert!(md_redacted.contains("[USERNAME]"));
        assert!(md_redacted.contains("| **Game Path** |"));
        assert!(md_redacted.contains("2021.3.16f1"));

        let md_unredacted = exporter.export_environment_markdown(&env, false).unwrap();
        assert!(md_unredacted.contains("SecretUser"));
    }

    #[test]
    fn test_diagnostics_export_redaction() {
        let diag = sample_diagnostics();
        let exporter = ReportExporter::new();

        let md = exporter.export_diagnostic_markdown(&diag, true).unwrap();
        assert!(!md.contains("developer"));
        assert!(!md.contains("mysecret123"));
        assert!(!md.contains("192.168.1.50"));
        assert!(md.contains("[USERNAME]"));
        assert!(md.contains("[REDACTED]"));
        assert!(md.contains("[REDACTED_IP]"));
        assert!(md.contains("### 1. [HIGH] Missing DLL"));

        let json = exporter.export_diagnostic_json(&diag, true).unwrap();
        assert!(!json.contains("developer"));
        assert!(!json.contains("mysecret123"));
        assert!(!json.contains("192.168.1.50"));
    }

    #[test]
    fn test_combined_export() {
        let env = sample_environment();
        let diag = sample_diagnostics();
        let exporter = ReportExporter::new();

        let md = exporter
            .export_combined_markdown(Some(&env), Some(&diag), true)
            .unwrap();
        assert!(md.contains("Comprehensive Report"));
        assert!(md.contains("Detection Evidence"));
        assert!(md.contains("Issues & Remediation"));
        assert!(!md.contains("SecretUser"));
        assert!(!md.contains("developer"));

        let json = exporter
            .export_combined_json(Some(&env), Some(&diag), true)
            .unwrap();
        assert!(json.contains("\"environment\""));
        assert!(json.contains("\"diagnostics\""));
    }
}
