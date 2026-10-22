use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Confirmed,
    HighProbability,
    Unknown,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    Mono,
    Il2Cpp,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    X86,
    X64,
    Arm64,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetPlatform {
    Windows,
    Linux,
    MacOs,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub key: String,
    pub description: String,
    pub confidence: Confidence,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityVersionInfo {
    pub raw: String,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub suffix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentReport {
    pub game_path: String,
    pub data_dir: Option<String>,
    pub unity_version: Option<UnityVersionInfo>,
    pub runtime_type: RuntimeType,
    pub architecture: Architecture,
    pub target_platform: TargetPlatform,
    pub installed_frameworks: Vec<String>,
    pub evidences: Vec<EvidenceItem>,
    pub overall_confidence: Confidence,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_serialization() {
        let report = EnvironmentReport {
            game_path: "/test/game".into(),
            data_dir: Some("/test/game/Data".into()),
            unity_version: None,
            runtime_type: RuntimeType::Mono,
            architecture: Architecture::X64,
            target_platform: TargetPlatform::Linux,
            installed_frameworks: vec![],
            evidences: vec![EvidenceItem {
                key: "managed_dir".into(),
                description: "Managed folder present".into(),
                confidence: Confidence::Confirmed,
                path: Some("/test/game/Data/Managed".into()),
            }],
            overall_confidence: Confidence::Confirmed,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("mono"));
    }
}
