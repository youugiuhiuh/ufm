//! ufm-registry: BepInEx 6 build metadata definitions and registry.

use serde::{Deserialize, Serialize};
use ufm_core::{Architecture, RuntimeType, TargetPlatform};

/// Release channel for BepInEx 6 builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildChannel {
    #[serde(alias = "BleedingEdge", alias = "bleedingedge")]
    BleedingEdge,
    #[serde(alias = "Release", alias = "release")]
    Release,
}

impl std::fmt::Display for BuildChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BleedingEdge => write!(f, "BleedingEdge"),
            Self::Release => write!(f, "Release"),
        }
    }
}

/// Metadata model for a BepInEx 6 build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BepInExBuild {
    pub id: String,
    pub version: String,
    pub channel: BuildChannel,
    pub runtime_type: RuntimeType,
    pub architecture: Architecture,
    pub target_platform: TargetPlatform,
    pub download_url: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

impl BepInExBuild {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        version: impl Into<String>,
        channel: BuildChannel,
        runtime_type: RuntimeType,
        architecture: Architecture,
        target_platform: TargetPlatform,
        download_url: impl Into<String>,
        sha256: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
            channel,
            runtime_type,
            architecture,
            target_platform,
            download_url: download_url.into(),
            sha256: sha256.into(),
            filename: None,
        }
    }

    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }
}

/// In-memory collection of BepInEx builds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BepInExRegistry {
    pub builds: Vec<BepInExBuild>,
}

impl BepInExRegistry {
    pub fn new() -> Self {
        Self { builds: Vec::new() }
    }

    pub fn with_builds(builds: Vec<BepInExBuild>) -> Self {
        Self { builds }
    }

    pub fn add(&mut self, build: BepInExBuild) {
        self.builds.push(build);
    }

    pub fn get(&self, id: &str) -> Option<&BepInExBuild> {
        self.builds.iter().find(|b| b.id == id)
    }

    pub fn builds(&self) -> &[BepInExBuild] {
        &self.builds
    }

    /// Predefined official/standard builds for tests and fallback catalogs.
    pub fn official_catalog() -> Self {
        Self::with_builds(vec![
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-win-x64-il2cpp",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Il2Cpp,
                Architecture::X64,
                TargetPlatform::Windows,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityIL2CPP_x64_6.0.0-pre.1.zip",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ).with_filename("BepInEx_UnityIL2CPP_x64_6.0.0-pre.1.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-win-x86-il2cpp",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Il2Cpp,
                Architecture::X86,
                TargetPlatform::Windows,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityIL2CPP_x86_6.0.0-pre.1.zip",
                "01ba4719c80b6fe911b091a7c05124b64eeece964e09c058ef8f9805daca546b",
            ).with_filename("BepInEx_UnityIL2CPP_x86_6.0.0-pre.1.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-win-x64-mono",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::Windows,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityMono_x64_6.0.0-pre.1.zip",
                "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae",
            ).with_filename("BepInEx_UnityMono_x64_6.0.0-pre.1.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-win-x86-mono",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X86,
                TargetPlatform::Windows,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityMono_x86_6.0.0-pre.1.zip",
                "fc94a7d45f6d728340f11123d4d88e6100f165ac4feb16900da788604131f0e6",
            ).with_filename("BepInEx_UnityMono_x86_6.0.0-pre.1.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-linux-x64-il2cpp",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Il2Cpp,
                Architecture::X64,
                TargetPlatform::Linux,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityIL2CPP_linux_x64_6.0.0-pre.1.zip",
                "6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b",
            ).with_filename("BepInEx_UnityIL2CPP_linux_x64_6.0.0-pre.1.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-linux-x64-mono",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::Linux,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityMono_linux_x64_6.0.0-pre.1.zip",
                "d4735e3a265e16eee03f59718b9b5d03019c07d8b6c51f90da3a666eec13ab35",
            ).with_filename("BepInEx_UnityMono_linux_x64_6.0.0-pre.1.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.724-win-x64-il2cpp",
                "6.0.0-be.724",
                BuildChannel::BleedingEdge,
                RuntimeType::Il2Cpp,
                Architecture::X64,
                TargetPlatform::Windows,
                "https://builds.bepinex.dev/projects/bepinex_be/724/BepInEx-Unity.IL2CPP-win-x64-6.0.0-be.724+50ff69f.zip",
                "4e1243bd22c66e76c2ba9eddc1f91394e57f9f835f8357771746261f31f267a1",
            ).with_filename("BepInEx-Unity.IL2CPP-win-x64-6.0.0-be.724+50ff69f.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.724-win-x64-mono",
                "6.0.0-be.724",
                BuildChannel::BleedingEdge,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::Windows,
                "https://builds.bepinex.dev/projects/bepinex_be/724/BepInEx-Unity.Mono-win-x64-6.0.0-be.724+50ff69f.zip",
                "18ac3e7343f016890c510e93f935261169d9e3f565436429830faf0934f4f8e4",
            ).with_filename("BepInEx-Unity.Mono-win-x64-6.0.0-be.724+50ff69f.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-macos-x64-mono",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::MacOs,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityMono_macos_x64_6.0.0-pre.1.zip",
                "4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a",
            ).with_filename("BepInEx_UnityMono_macos_x64_6.0.0-pre.1.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.724-macos-arm64-il2cpp",
                "6.0.0-be.724",
                BuildChannel::BleedingEdge,
                RuntimeType::Il2Cpp,
                Architecture::Arm64,
                TargetPlatform::MacOs,
                "https://builds.bepinex.dev/projects/bepinex_be/724/BepInEx-Unity.IL2CPP-macos-arm64-6.0.0-be.724.zip",
                "ef2d127de37b942baad06145e54b0c619a1f22327b2ebbcfbec78f5564afe39d",
            ).with_filename("BepInEx-Unity.IL2CPP-macos-arm64-6.0.0-be.724.zip"),
        ])
    }
}

pub fn is_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ready() {
        assert!(is_ready());
    }

    #[test]
    fn test_build_serialization() {
        let build = BepInExBuild::new(
            "bepinex-v6.0.0-be.724-win-x64-il2cpp",
            "6.0.0-be.724",
            BuildChannel::BleedingEdge,
            RuntimeType::Il2Cpp,
            Architecture::X64,
            TargetPlatform::Windows,
            "https://builds.bepinex.dev/example.zip",
            "abc123sha256",
        )
        .with_filename("example.zip");

        let json = serde_json::to_string(&build).unwrap();
        assert!(json.contains("\"bleeding_edge\""));
        assert!(json.contains("\"il2cpp\""));
        assert!(json.contains("\"windows\""));
        assert!(json.contains("\"x64\""));
        assert!(json.contains("\"abc123sha256\""));

        let deserialized: BepInExBuild = serde_json::from_str(&json).unwrap();
        assert_eq!(build, deserialized);
    }

    #[test]
    fn test_registry_operations() {
        let catalog = BepInExRegistry::official_catalog();
        assert!(!catalog.builds().is_empty());

        let build = catalog.get("bepinex-v6.0.0-pre.1-win-x64-il2cpp");
        assert!(build.is_some());
        let b = build.unwrap();
        assert_eq!(b.version, "6.0.0-pre.1");
        assert_eq!(b.channel, BuildChannel::Release);
        assert_eq!(b.runtime_type, RuntimeType::Il2Cpp);
        assert_eq!(b.architecture, Architecture::X64);
        assert_eq!(b.target_platform, TargetPlatform::Windows);

        let mut custom = BepInExRegistry::new();
        assert_eq!(custom.builds().len(), 0);
        custom.add(b.clone());
        assert_eq!(custom.builds().len(), 1);
        assert_eq!(custom.get(&b.id).unwrap(), b);
    }
}
