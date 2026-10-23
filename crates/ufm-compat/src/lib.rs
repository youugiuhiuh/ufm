//! ufm-compat: Compatibility evaluation and candidate build filtering.

use serde::{Deserialize, Serialize};
use ufm_core::{
    Architecture, Confidence, EnvironmentReport, RuntimeType, TargetPlatform, UnityVersionInfo,
};
use ufm_registry::{BepInExBuild, BuildChannel};

/// Overall compatibility status of a candidate build against an environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompatibilityStatus {
    Compatible,
    PartiallyCompatible,
    Incompatible,
    Unknown,
}

/// The state of an individual matching dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchState {
    Matched,
    Mismatched,
    Unknown,
}

/// Detailed match evidence for a specific dimension (e.g. runtime, platform).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchReason {
    pub aspect: String,
    pub state: MatchState,
    pub detail: String,
}

impl MatchReason {
    pub fn new(aspect: impl Into<String>, state: MatchState, detail: impl Into<String>) -> Self {
        Self {
            aspect: aspect.into(),
            state,
            detail: detail.into(),
        }
    }

    pub fn is_matched(&self) -> bool {
        self.state == MatchState::Matched
    }
}

/// Severity level for compatibility warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WarningLevel {
    Info,
    Warning,
    Risk,
}

/// Warning or risk note surfaced during compatibility evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityWarning {
    pub level: WarningLevel,
    pub code: String,
    pub message: String,
}

impl CompatibilityWarning {
    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: WarningLevel::Info,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: WarningLevel::Warning,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn risk(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: WarningLevel::Risk,
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Assessment output for a candidate build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityAssessment {
    pub build: BepInExBuild,
    pub status: CompatibilityStatus,
    pub reasons: Vec<MatchReason>,
    pub warnings: Vec<CompatibilityWarning>,
}

impl CompatibilityAssessment {
    pub fn is_compatible(&self) -> bool {
        self.status == CompatibilityStatus::Compatible
    }

    pub fn is_usable(&self) -> bool {
        matches!(
            self.status,
            CompatibilityStatus::Compatible | CompatibilityStatus::PartiallyCompatible
        )
    }
}

/// Full compatibility report for an environment against a candidate catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub assessments: Vec<CompatibilityAssessment>,
    pub recommended_build: Option<BepInExBuild>,
    pub global_warnings: Vec<CompatibilityWarning>,
}

/// Evaluates a single candidate build against the environment report.
pub fn evaluate_build(report: &EnvironmentReport, build: &BepInExBuild) -> CompatibilityAssessment {
    let mut reasons = Vec::new();
    let mut warnings = Vec::new();

    // 1. Evaluate RuntimeType
    let runtime_state = match report.runtime_type {
        RuntimeType::Unknown => {
            warnings.push(CompatibilityWarning::risk(
                "RISK_UNKNOWN_RUNTIME",
                "Runtime type is unknown. Installing Mono or IL2CPP without confirmation may crash on startup.",
            ));
            MatchReason::new(
                "runtime",
                MatchState::Unknown,
                "Game runtime type could not be determined (Unknown).",
            )
        }
        detected if detected == build.runtime_type => MatchReason::new(
            "runtime",
            MatchState::Matched,
            format!("Runtime matches detected {:?}.", detected),
        ),
        detected => MatchReason::new(
            "runtime",
            MatchState::Mismatched,
            format!(
                "Runtime mismatch: build targets {:?}, but game runtime is {:?}.",
                build.runtime_type, detected
            ),
        ),
    };
    reasons.push(runtime_state);

    // 2. Evaluate Architecture
    let arch_state = match report.architecture {
        Architecture::Unknown => {
            warnings.push(CompatibilityWarning::risk(
                "RISK_UNKNOWN_ARCHITECTURE",
                "Target architecture is unknown. Architecture mismatch causes DLL load errors.",
            ));
            MatchReason::new(
                "architecture",
                MatchState::Unknown,
                "Game architecture could not be determined (Unknown).",
            )
        }
        detected if detected == build.architecture => MatchReason::new(
            "architecture",
            MatchState::Matched,
            format!("Architecture matches detected {:?}.", detected),
        ),
        detected => MatchReason::new(
            "architecture",
            MatchState::Mismatched,
            format!(
                "Architecture mismatch: build is {:?}, but game is {:?}.",
                build.architecture, detected
            ),
        ),
    };
    reasons.push(arch_state);

    // 3. Evaluate TargetPlatform
    let platform_state = match report.target_platform {
        TargetPlatform::Unknown => {
            warnings.push(CompatibilityWarning::risk(
                "RISK_UNKNOWN_PLATFORM",
                "Target operating system platform is unknown. Binary compatibility cannot be guaranteed.",
            ));
            MatchReason::new(
                "platform",
                MatchState::Unknown,
                "Game target platform could not be determined (Unknown).",
            )
        }
        detected if detected == build.target_platform => MatchReason::new(
            "platform",
            MatchState::Matched,
            format!("Platform matches detected {:?}.", detected),
        ),
        detected => MatchReason::new(
            "platform",
            MatchState::Mismatched,
            format!(
                "Platform mismatch: build is for {:?}, but game runs on {:?}.",
                build.target_platform, detected
            ),
        ),
    };
    reasons.push(platform_state);

    // 4. Evaluate UnityVersion
    let unity_state = evaluate_unity_version(report.unity_version.as_ref(), build, &mut warnings);
    reasons.push(unity_state);

    // 5. Channel & runtime specific caveats
    if build.channel == BuildChannel::BleedingEdge {
        warnings.push(CompatibilityWarning::warning(
            "BLEEDING_EDGE_BUILD",
            "Bleeding Edge (builds.bepinex.dev 夜间构建)：包含最新特性与修复，但可能包含未完全稳定的变更。",
        ));
    }
    if build.version.starts_with("5.") {
        warnings.push(CompatibilityWarning::info(
            "BEPINEX_5_STABLE",
            "BepInEx 5 (经典稳定版)：针对 Unity Mono 游戏兼容性最佳，拥有成熟 Mod 插件生态。若 BepInEx 6 存在异常或老插件无法加载，强烈建议选用此版本。",
        ));
    }
    if build.runtime_type == RuntimeType::Il2Cpp {
        warnings.push(CompatibilityWarning::info(
            "IL2CPP_NOTICE",
            "IL2CPP requires dummy assembly generation / unstripped metadata upon first launch.",
        ));
    }

    // 6. Overall confidence check from report
    if report.overall_confidence == Confidence::Conflict {
        warnings.push(CompatibilityWarning::risk(
            "CONFLICTING_DETECTION_EVIDENCE",
            "Environment detection report contains conflicting evidence.",
        ));
    } else if report.overall_confidence == Confidence::Unknown {
        warnings.push(CompatibilityWarning::risk(
            "UNKNOWN_DETECTION_EVIDENCE",
            "Environment detection report has unknown confidence level.",
        ));
    }

    // 7. Determine overall compatibility status
    let is_conflict = report.overall_confidence == Confidence::Conflict;
    let has_mismatch = reasons.iter().any(|r| r.state == MatchState::Mismatched);
    let has_unknown = reasons.iter().any(|r| r.state == MatchState::Unknown);

    let status = if is_conflict {
        // Conflicting detection means ground truth cannot be established
        CompatibilityStatus::Unknown
    } else if has_mismatch {
        CompatibilityStatus::Incompatible
    } else if has_unknown || report.overall_confidence == Confidence::Unknown {
        let core_dimension_unknown = reasons.iter().any(|r| {
            (r.aspect == "runtime" || r.aspect == "architecture" || r.aspect == "platform")
                && r.state == MatchState::Unknown
        });
        if core_dimension_unknown
            || (build.runtime_type == RuntimeType::Il2Cpp && report.unity_version.is_none())
            || report.overall_confidence == Confidence::Unknown
        {
            CompatibilityStatus::Unknown
        } else {
            CompatibilityStatus::PartiallyCompatible
        }
    } else {
        CompatibilityStatus::Compatible
    };

    CompatibilityAssessment {
        build: build.clone(),
        status,
        reasons,
        warnings,
    }
}

fn evaluate_unity_version(
    version: Option<&UnityVersionInfo>,
    build: &BepInExBuild,
    warnings: &mut Vec<CompatibilityWarning>,
) -> MatchReason {
    match version {
        None => {
            if build.runtime_type == RuntimeType::Il2Cpp {
                warnings.push(CompatibilityWarning::risk(
                    "RISK_MISSING_UNITY_VERSION_IL2CPP",
                    "Unity version evidence is missing. BepInEx IL2CPP requires exact Unity version for interop generation.",
                ));
            } else {
                warnings.push(CompatibilityWarning::warning(
                    "WARN_MISSING_UNITY_VERSION",
                    "Unity version evidence is missing. Compatibility cannot be verified for version-specific hooks.",
                ));
            }
            MatchReason::new(
                "unity_version",
                MatchState::Unknown,
                "Unity engine version could not be detected from environment.",
            )
        }
        Some(info) => {
            if info.major < 2018 && build.runtime_type == RuntimeType::Il2Cpp {
                warnings.push(CompatibilityWarning::risk(
                    "UNSUPPORTED_LEGACY_UNITY",
                    format!(
                        "Unity version {} (< 2018) does not support BepInEx 6 IL2CPP.",
                        info.raw
                    ),
                ));
                MatchReason::new(
                    "unity_version",
                    MatchState::Mismatched,
                    format!("Unity {} is too old for BepInEx 6 IL2CPP.", info.raw),
                )
            } else if info.major >= 6000 {
                warnings.push(CompatibilityWarning::warning(
                    "UNITY_6_EXPERIMENTAL",
                    format!(
                        "Unity 6 detected ({}). BepInEx 6 support for Unity 6 is experimental.",
                        info.raw
                    ),
                ));
                MatchReason::new(
                    "unity_version",
                    MatchState::Matched,
                    format!("Unity 6 ({}) supported experimentally.", info.raw),
                )
            } else {
                MatchReason::new(
                    "unity_version",
                    MatchState::Matched,
                    format!("Unity {} is supported.", info.raw),
                )
            }
        }
    }
}

/// Evaluates all candidates and generates a compatibility report with recommendations.
pub fn filter_candidates(
    report: &EnvironmentReport,
    builds: &[BepInExBuild],
) -> CompatibilityReport {
    let mut global_warnings = Vec::new();

    if report.overall_confidence == Confidence::Conflict {
        global_warnings.push(CompatibilityWarning::risk(
            "REPORT_CONFIDENCE_CONFLICT",
            "Environment detection produced conflicting indicators. Manual verification is advised.",
        ));
    } else if report.overall_confidence == Confidence::Unknown {
        global_warnings.push(CompatibilityWarning::risk(
            "REPORT_CONFIDENCE_UNKNOWN",
            "Environment detection confidence is Unknown. Some attributes could not be confirmed.",
        ));
    }

    let assessments: Vec<CompatibilityAssessment> = builds
        .iter()
        .map(|build| evaluate_build(report, build))
        .collect();

    // Recommendation strategy:
    // 默认优先推荐 builds.bepinex.dev 上的最新 Bleeding Edge 构建（修复了海量游戏崩溃与现代 Unity 适配问题），
    // 随后是 BepInEx 6 Release，再随后是 BepInEx 5 经典稳定版。
    let build_score = |build: &BepInExBuild| -> (u8, u32) {
        let channel_tier = match build.channel {
            BuildChannel::BleedingEdge => 3,
            BuildChannel::Release if build.version.starts_with("6.") => 2,
            BuildChannel::Release => 1,
        };
        let be_num = if let Some(idx) = build.version.find("be.") {
            build.version[idx + 3..].parse::<u32>().unwrap_or(0)
        } else {
            0
        };
        (channel_tier, be_num)
    };

    let recommended_build = assessments
        .iter()
        .filter(|a| a.status == CompatibilityStatus::Compatible)
        .max_by_key(|a| (build_score(&a.build), &a.build.version))
        .or_else(|| {
            assessments
                .iter()
                .filter(|a| a.status == CompatibilityStatus::PartiallyCompatible)
                .max_by_key(|a| (build_score(&a.build), &a.build.version))
        })
        .map(|a| a.build.clone());

    CompatibilityReport {
        assessments,
        recommended_build,
        global_warnings,
    }
}

/// Quick helper returning only builds with `Compatible` status.
pub fn get_compatible_builds(
    report: &EnvironmentReport,
    builds: &[BepInExBuild],
) -> Vec<BepInExBuild> {
    filter_candidates(report, builds)
        .assessments
        .into_iter()
        .filter(|a| a.status == CompatibilityStatus::Compatible)
        .map(|a| a.build)
        .collect()
}

pub fn is_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ufm_core::EvidenceItem;
    use ufm_registry::BepInExRegistry;

    fn sample_evidence() -> Vec<EvidenceItem> {
        vec![EvidenceItem {
            key: "test".into(),
            description: "test evidence".into(),
            confidence: Confidence::Confirmed,
            path: None,
        }]
    }

    #[test]
    fn test_ready() {
        assert!(is_ready());
    }

    #[test]
    fn test_compatible_mono_windows() {
        let catalog = BepInExRegistry::official_catalog();
        let report = EnvironmentReport {
            game_path: "C:\\Games\\TestGame".into(),
            data_dir: Some("C:\\Games\\TestGame\\TestGame_Data".into()),
            unity_version: Some(UnityVersionInfo {
                raw: "2020.3.30f1".into(),
                major: 2020,
                minor: 3,
                patch: 30,
                suffix: Some("f1".into()),
            }),
            runtime_type: RuntimeType::Mono,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec![],
            evidences: sample_evidence(),
            overall_confidence: Confidence::Confirmed,
        };

        let result = filter_candidates(&report, catalog.builds());
        let compatible: Vec<_> = result
            .assessments
            .iter()
            .filter(|a| a.status == CompatibilityStatus::Compatible)
            .collect();

        assert!(!compatible.is_empty());
        for c in &compatible {
            assert_eq!(c.build.runtime_type, RuntimeType::Mono);
            assert_eq!(c.build.architecture, Architecture::X64);
            assert_eq!(c.build.target_platform, TargetPlatform::Windows);
            assert!(c.reasons.iter().all(|r| r.state == MatchState::Matched));
        }

        // Recommended build defaults to the latest BleedingEdge build from builds.bepinex.dev
        let recommended = result.recommended_build.expect("recommended build");
        assert_eq!(recommended.channel, BuildChannel::BleedingEdge);
        assert_eq!(recommended.runtime_type, RuntimeType::Mono);
    }

    #[test]
    fn test_compatible_il2cpp_windows() {
        let catalog = BepInExRegistry::official_catalog();
        let report = EnvironmentReport {
            game_path: "C:\\Games\\TestIL2CPP".into(),
            data_dir: Some("C:\\Games\\TestIL2CPP\\TestGame_Data".into()),
            unity_version: Some(UnityVersionInfo {
                raw: "2021.3.15f1".into(),
                major: 2021,
                minor: 3,
                patch: 15,
                suffix: Some("f1".into()),
            }),
            runtime_type: RuntimeType::Il2Cpp,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec![],
            evidences: sample_evidence(),
            overall_confidence: Confidence::Confirmed,
        };

        let result = filter_candidates(&report, catalog.builds());
        let compatible: Vec<_> = result
            .assessments
            .iter()
            .filter(|a| a.status == CompatibilityStatus::Compatible)
            .collect();

        assert!(!compatible.is_empty());
        for c in &compatible {
            assert_eq!(c.build.runtime_type, RuntimeType::Il2Cpp);
            assert_eq!(c.build.architecture, Architecture::X64);
            assert_eq!(c.build.target_platform, TargetPlatform::Windows);
            // IL2CPP notice should be present
            assert!(c.warnings.iter().any(|w| w.code == "IL2CPP_NOTICE"));
        }
    }

    #[test]
    fn test_linux_and_macos_filtering() {
        let catalog = BepInExRegistry::official_catalog();
        let linux_report = EnvironmentReport {
            game_path: "/games/linuxgame".into(),
            data_dir: Some("/games/linuxgame/data".into()),
            unity_version: Some(UnityVersionInfo {
                raw: "2019.4.40f1".into(),
                major: 2019,
                minor: 4,
                patch: 40,
                suffix: None,
            }),
            runtime_type: RuntimeType::Mono,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Linux,
            installed_frameworks: vec![],
            evidences: sample_evidence(),
            overall_confidence: Confidence::Confirmed,
        };

        let compatible = get_compatible_builds(&linux_report, catalog.builds());
        assert!(!compatible.is_empty());
        for b in &compatible {
            assert_eq!(b.target_platform, TargetPlatform::Linux);
        }

        let macos_report = EnvironmentReport {
            game_path: "/Applications/MacGame.app".into(),
            data_dir: None,
            unity_version: Some(UnityVersionInfo {
                raw: "2022.3.10f1".into(),
                major: 2022,
                minor: 3,
                patch: 10,
                suffix: None,
            }),
            runtime_type: RuntimeType::Il2Cpp,
            architecture: Architecture::Arm64,
            target_platform: TargetPlatform::MacOs,
            installed_frameworks: vec![],
            evidences: sample_evidence(),
            overall_confidence: Confidence::Confirmed,
        };

        let mac_compatible = get_compatible_builds(&macos_report, catalog.builds());
        assert_eq!(mac_compatible.len(), 1);
        assert_eq!(mac_compatible[0].architecture, Architecture::Arm64);
        assert_eq!(mac_compatible[0].target_platform, TargetPlatform::MacOs);
    }

    #[test]
    fn test_missing_runtime_evidence_marks_unknown_and_surfaces_risk() {
        let catalog = BepInExRegistry::official_catalog();
        let report = EnvironmentReport {
            game_path: "C:\\Games\\UnknownGame".into(),
            data_dir: None,
            unity_version: Some(UnityVersionInfo {
                raw: "2021.3.0f1".into(),
                major: 2021,
                minor: 3,
                patch: 0,
                suffix: None,
            }),
            runtime_type: RuntimeType::Unknown, // missing runtime!
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec![],
            evidences: vec![],
            overall_confidence: Confidence::HighProbability,
        };

        let result = filter_candidates(&report, catalog.builds());

        // For builds that match platform (Windows) and arch (x64), status must be Unknown due to missing runtime evidence
        let matching_win_x64: Vec<_> = result
            .assessments
            .iter()
            .filter(|a| {
                a.build.target_platform == TargetPlatform::Windows
                    && a.build.architecture == Architecture::X64
            })
            .collect();

        assert!(!matching_win_x64.is_empty());
        for assessment in matching_win_x64 {
            assert_eq!(assessment.status, CompatibilityStatus::Unknown);
            assert!(assessment
                .warnings
                .iter()
                .any(|w| w.code == "RISK_UNKNOWN_RUNTIME" && w.level == WarningLevel::Risk));
            let runtime_reason = assessment
                .reasons
                .iter()
                .find(|r| r.aspect == "runtime")
                .unwrap();
            assert_eq!(runtime_reason.state, MatchState::Unknown);
        }

        // Builds with wrong architecture or platform should still be Incompatible
        let mismatched: Vec<_> = result
            .assessments
            .iter()
            .filter(|a| a.build.target_platform != TargetPlatform::Windows)
            .collect();
        for assessment in mismatched {
            assert_eq!(assessment.status, CompatibilityStatus::Incompatible);
        }

        // Zero builds should be fully compatible
        assert_eq!(get_compatible_builds(&report, catalog.builds()).len(), 0);
    }

    #[test]
    fn test_missing_architecture_marks_unknown_and_surfaces_risk() {
        let catalog = BepInExRegistry::official_catalog();
        let report = EnvironmentReport {
            game_path: "C:\\Games\\Game".into(),
            data_dir: None,
            unity_version: Some(UnityVersionInfo {
                raw: "2021.3.0f1".into(),
                major: 2021,
                minor: 3,
                patch: 0,
                suffix: None,
            }),
            runtime_type: RuntimeType::Mono,
            architecture: Architecture::Unknown, // missing arch!
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec![],
            evidences: vec![],
            overall_confidence: Confidence::HighProbability,
        };

        let result = filter_candidates(&report, catalog.builds());
        let matching_win_mono: Vec<_> = result
            .assessments
            .iter()
            .filter(|a| {
                a.build.target_platform == TargetPlatform::Windows
                    && a.build.runtime_type == RuntimeType::Mono
            })
            .collect();

        assert!(!matching_win_mono.is_empty());
        for assessment in matching_win_mono {
            assert_eq!(assessment.status, CompatibilityStatus::Unknown);
            assert!(assessment
                .warnings
                .iter()
                .any(|w| w.code == "RISK_UNKNOWN_ARCHITECTURE" && w.level == WarningLevel::Risk));
            let arch_reason = assessment
                .reasons
                .iter()
                .find(|r| r.aspect == "architecture")
                .unwrap();
            assert_eq!(arch_reason.state, MatchState::Unknown);
        }

        assert_eq!(get_compatible_builds(&report, catalog.builds()).len(), 0);
    }

    #[test]
    fn test_missing_target_platform_marks_unknown_and_surfaces_risk() {
        let catalog = BepInExRegistry::official_catalog();
        let report = EnvironmentReport {
            game_path: "C:\\Games\\Game".into(),
            data_dir: None,
            unity_version: Some(UnityVersionInfo {
                raw: "2021.3.0f1".into(),
                major: 2021,
                minor: 3,
                patch: 0,
                suffix: None,
            }),
            runtime_type: RuntimeType::Mono,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Unknown, // missing platform!
            installed_frameworks: vec![],
            evidences: vec![],
            overall_confidence: Confidence::HighProbability,
        };

        let result = filter_candidates(&report, catalog.builds());
        let matching_x64_mono: Vec<_> = result
            .assessments
            .iter()
            .filter(|a| {
                a.build.architecture == Architecture::X64
                    && a.build.runtime_type == RuntimeType::Mono
            })
            .collect();

        assert!(!matching_x64_mono.is_empty());
        for assessment in matching_x64_mono {
            assert_eq!(assessment.status, CompatibilityStatus::Unknown);
            assert!(assessment
                .warnings
                .iter()
                .any(|w| w.code == "RISK_UNKNOWN_PLATFORM" && w.level == WarningLevel::Risk));
            let plat_reason = assessment
                .reasons
                .iter()
                .find(|r| r.aspect == "platform")
                .unwrap();
            assert_eq!(plat_reason.state, MatchState::Unknown);
        }

        assert_eq!(get_compatible_builds(&report, catalog.builds()).len(), 0);
    }

    #[test]
    fn test_missing_unity_version_effects() {
        let catalog = BepInExRegistry::official_catalog();
        // Case 1: IL2CPP without unity version -> Unknown + Risk
        let il2cpp_report = EnvironmentReport {
            game_path: "C:\\Games\\Game".into(),
            data_dir: None,
            unity_version: None,
            runtime_type: RuntimeType::Il2Cpp,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec![],
            evidences: sample_evidence(),
            overall_confidence: Confidence::Confirmed,
        };
        let il2cpp_result = filter_candidates(&il2cpp_report, catalog.builds());
        let il2cpp_win_x64 = il2cpp_result
            .assessments
            .iter()
            .find(|a| {
                a.build.runtime_type == RuntimeType::Il2Cpp
                    && a.build.architecture == Architecture::X64
                    && a.build.target_platform == TargetPlatform::Windows
            })
            .unwrap();
        assert_eq!(il2cpp_win_x64.status, CompatibilityStatus::Unknown);
        assert!(il2cpp_win_x64.warnings.iter().any(|w| w.code
            == "RISK_MISSING_UNITY_VERSION_IL2CPP"
            && w.level == WarningLevel::Risk));

        // Case 2: Mono without unity version -> PartiallyCompatible + Warning
        let mono_report = EnvironmentReport {
            game_path: "C:\\Games\\Game".into(),
            data_dir: None,
            unity_version: None,
            runtime_type: RuntimeType::Mono,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec![],
            evidences: sample_evidence(),
            overall_confidence: Confidence::Confirmed,
        };
        let mono_result = filter_candidates(&mono_report, catalog.builds());
        let mono_win_x64 = mono_result
            .assessments
            .iter()
            .find(|a| {
                a.build.runtime_type == RuntimeType::Mono
                    && a.build.architecture == Architecture::X64
                    && a.build.target_platform == TargetPlatform::Windows
            })
            .unwrap();
        assert_eq!(
            mono_win_x64.status,
            CompatibilityStatus::PartiallyCompatible
        );
        assert!(mono_win_x64
            .warnings
            .iter()
            .any(|w| w.code == "WARN_MISSING_UNITY_VERSION" && w.level == WarningLevel::Warning));
    }

    #[test]
    fn test_unity_version_extremes() {
        let catalog = BepInExRegistry::official_catalog();

        // Legacy Unity 5 with IL2CPP -> Incompatible
        let legacy_report = EnvironmentReport {
            game_path: "C:\\Games\\LegacyGame".into(),
            data_dir: None,
            unity_version: Some(UnityVersionInfo {
                raw: "5.6.7f1".into(),
                major: 5,
                minor: 6,
                patch: 7,
                suffix: None,
            }),
            runtime_type: RuntimeType::Il2Cpp,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec![],
            evidences: sample_evidence(),
            overall_confidence: Confidence::Confirmed,
        };
        let legacy_res = filter_candidates(&legacy_report, catalog.builds());
        let il2cpp_legacy = legacy_res
            .assessments
            .iter()
            .find(|a| {
                a.build.runtime_type == RuntimeType::Il2Cpp
                    && a.build.target_platform == TargetPlatform::Windows
            })
            .unwrap();
        assert_eq!(il2cpp_legacy.status, CompatibilityStatus::Incompatible);
        assert!(il2cpp_legacy
            .warnings
            .iter()
            .any(|w| w.code == "UNSUPPORTED_LEGACY_UNITY"));

        // Unity 6 with experimental warning
        let unity6_report = EnvironmentReport {
            game_path: "C:\\Games\\ModernGame".into(),
            data_dir: None,
            unity_version: Some(UnityVersionInfo {
                raw: "6000.0.1f1".into(),
                major: 6000,
                minor: 0,
                patch: 1,
                suffix: None,
            }),
            runtime_type: RuntimeType::Mono,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec![],
            evidences: sample_evidence(),
            overall_confidence: Confidence::Confirmed,
        };
        let u6_res = filter_candidates(&unity6_report, catalog.builds());
        let u6_mono = u6_res
            .assessments
            .iter()
            .find(|a| {
                a.build.runtime_type == RuntimeType::Mono
                    && a.build.architecture == Architecture::X64
                    && a.build.target_platform == TargetPlatform::Windows
            })
            .unwrap();
        assert_eq!(u6_mono.status, CompatibilityStatus::Compatible);
        assert!(u6_mono
            .warnings
            .iter()
            .any(|w| w.code == "UNITY_6_EXPERIMENTAL"));
    }

    #[test]
    fn test_conflict_overall_confidence() {
        let catalog = BepInExRegistry::official_catalog();
        let report = EnvironmentReport {
            game_path: "C:\\Games\\ConflictedGame".into(),
            data_dir: None,
            unity_version: Some(UnityVersionInfo {
                raw: "2021.3.0f1".into(),
                major: 2021,
                minor: 3,
                patch: 0,
                suffix: None,
            }),
            runtime_type: RuntimeType::Mono,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Windows,
            installed_frameworks: vec![],
            evidences: sample_evidence(),
            overall_confidence: Confidence::Conflict, // conflict!
        };

        let result = filter_candidates(&report, catalog.builds());
        assert!(result
            .global_warnings
            .iter()
            .any(|w| w.code == "REPORT_CONFIDENCE_CONFLICT"));
        for assessment in &result.assessments {
            assert_eq!(assessment.status, CompatibilityStatus::Unknown);
            assert!(assessment
                .warnings
                .iter()
                .any(|w| w.code == "CONFLICTING_DETECTION_EVIDENCE"));
        }
    }
}
