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

    /// Returns a slice of all registered BepInEx builds.
    pub fn list_all(&self) -> &[BepInExBuild] {
        &self.builds
    }

    /// 从 builds.bepinex.dev HTML 文本中解析提取 Bleeding Edge 构建
    pub fn parse_bepinex_be_html(html: &str) -> Vec<BepInExBuild> {
        let mut list = Vec::new();
        for line in html.lines() {
            if !line.contains("/projects/bepinex_be/") || !line.contains(".zip") {
                continue;
            }
            if let Some(start_idx) = line.find("href=\"/projects/bepinex_be/") {
                let rest = &line[start_idx + 6..];
                if let Some(end_idx) = rest.find('"') {
                    let href = &rest[..end_idx];
                    let parts: Vec<&str> = href.split('/').collect();
                    if parts.len() >= 5 {
                        let build_num = parts[3];
                        let raw_filename = parts[4];
                        let decoded_filename = raw_filename.replace("%2B", "+");

                        let is_mono = decoded_filename.contains("Unity.Mono");
                        let is_il2cpp = decoded_filename.contains("Unity.IL2CPP");
                        if !is_mono && !is_il2cpp {
                            continue;
                        }

                        let runtime_type = if is_mono {
                            RuntimeType::Mono
                        } else {
                            RuntimeType::Il2Cpp
                        };

                        let target_platform = if decoded_filename.contains("-win-") {
                            TargetPlatform::Windows
                        } else if decoded_filename.contains("-linux-") {
                            TargetPlatform::Linux
                        } else if decoded_filename.contains("-macos-") {
                            TargetPlatform::MacOs
                        } else {
                            continue;
                        };

                        let architecture = if decoded_filename.contains("-x64-") {
                            Architecture::X64
                        } else if decoded_filename.contains("-x86-") {
                            Architecture::X86
                        } else if decoded_filename.contains("-arm64-") {
                            Architecture::Arm64
                        } else {
                            continue;
                        };

                        let version = format!("6.0.0-be.{build_num}");
                        let id = format!(
                            "bepinex-v6.0.0-be.{}-{:?}-{:?}-{:?}",
                            build_num, target_platform, architecture, runtime_type
                        )
                        .to_lowercase();

                        let download_url = format!("https://builds.bepinex.dev{href}");
                        let build = BepInExBuild::new(
                            id,
                            version,
                            BuildChannel::BleedingEdge,
                            runtime_type,
                            architecture,
                            target_platform,
                            download_url,
                            "",
                        )
                        .with_filename(decoded_filename);

                        if !list.iter().any(|b: &BepInExBuild| b.id == build.id) {
                            list.push(build);
                        }
                    }
                }
            }
        }
        list
    }

    /// Predefined official/standard builds for tests and fallback catalogs.
    pub fn official_catalog() -> Self {
        Self::with_builds(vec![
            // ==========================================
            // 1. BepInEx 5 经典稳定版 (最高兼容性，适合 Mono 游戏与老插件)
            // ==========================================
            BepInExBuild::new(
                "bepinex-v5.4.23.2-win-x64-mono",
                "5.4.23.2",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::Windows,
                "https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x64_5.4.23.2.zip",
                "f752ce4e838f4c305b9da1404b6745f2cff23b8bfd494f79f0c84d0a01f59b46",
            ).with_filename("BepInEx_win_x64_5.4.23.2.zip"),
            BepInExBuild::new(
                "bepinex-v5.4.23.2-win-x86-mono",
                "5.4.23.2",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X86,
                TargetPlatform::Windows,
                "https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_win_x86_5.4.23.2.zip",
                "7dd51193587a86c11a58530aeeb4789b15763cc6cf2c55e71785cef4ef214253",
            ).with_filename("BepInEx_win_x86_5.4.23.2.zip"),
            BepInExBuild::new(
                "bepinex-v5.4.23.2-linux-x64-mono",
                "5.4.23.2",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::Linux,
                "https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_linux_x64_5.4.23.2.zip",
                "ddc446143a0a277ed92eb7a1f5bf794ff4fdd0023bddc0491f85eafd1898225b",
            ).with_filename("BepInEx_linux_x64_5.4.23.2.zip"),
            BepInExBuild::new(
                "bepinex-v5.4.23.2-macos-x64-mono",
                "5.4.23.2",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::MacOs,
                "https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.2/BepInEx_macos_x64_5.4.23.2.zip",
                "bc724642ae54c31b9ab8e7d5b2ae5daffef0201a7349d78fcd43a7d4cbb8dbbf",
            ).with_filename("BepInEx_macos_x64_5.4.23.2.zip"),

            // ==========================================
            // 2. BepInEx 6 官方 Release 预发布构建
            // ==========================================
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-win-x64-il2cpp",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Il2Cpp,
                Architecture::X64,
                TargetPlatform::Windows,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityIL2CPP_x64_6.0.0-pre.1.zip",
                "b5ac213ee70b029bda1353a82b067d89709ec1a55ec9b775779c2813f3d84212",
            ).with_filename("BepInEx_UnityIL2CPP_x64_6.0.0-pre.1.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-win-x86-il2cpp",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Il2Cpp,
                Architecture::X86,
                TargetPlatform::Windows,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityIL2CPP_x86_6.0.0-pre.1.zip",
                "1cc048c216a1e1b6fc1b41f7e4ebf86d8843671107724dadc6a30112e6c6abd3",
            ).with_filename("BepInEx_UnityIL2CPP_x86_6.0.0-pre.1.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-win-x64-mono",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::Windows,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityMono_x64_6.0.0-pre.1.zip",
                "5107fcb9c8b35cf42bf61474226529eb9072696f539f7e9ac3646a0bffd267fc",
            ).with_filename("BepInEx_UnityMono_x64_6.0.0-pre.1.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-pre.1-win-x86-mono",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X86,
                TargetPlatform::Windows,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityMono_x86_6.0.0-pre.1.zip",
                "ae55440e689a967188b8e2ce88e9775793a26e497feeaa9eb3e43bfd5b470499",
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
                "bepinex-v6.0.0-pre.1-macos-x64-mono",
                "6.0.0-pre.1",
                BuildChannel::Release,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::MacOs,
                "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityMono_macos_x64_6.0.0-pre.1.zip",
                "4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a",
            ).with_filename("BepInEx_UnityMono_macos_x64_6.0.0-pre.1.zip"),

            // ==========================================
            // 3. builds.bepinex.dev (bepinex_be) 最新 Bleeding Edge 夜间构建
            // ==========================================
            BepInExBuild::new(
                "bepinex-v6.0.0-be.790-win-x64-mono",
                "6.0.0-be.790",
                BuildChannel::BleedingEdge,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::Windows,
                "https://builds.bepinex.dev/projects/bepinex_be/790/BepInEx-Unity.Mono-win-x64-6.0.0-be.790%2B4e98359.zip",
                "",
            ).with_filename("BepInEx-Unity.Mono-win-x64-6.0.0-be.790+4e98359.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.790-win-x64-il2cpp",
                "6.0.0-be.790",
                BuildChannel::BleedingEdge,
                RuntimeType::Il2Cpp,
                Architecture::X64,
                TargetPlatform::Windows,
                "https://builds.bepinex.dev/projects/bepinex_be/790/BepInEx-Unity.IL2CPP-win-x64-6.0.0-be.790%2B4e98359.zip",
                "",
            ).with_filename("BepInEx-Unity.IL2CPP-win-x64-6.0.0-be.790+4e98359.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.790-win-x86-mono",
                "6.0.0-be.790",
                BuildChannel::BleedingEdge,
                RuntimeType::Mono,
                Architecture::X86,
                TargetPlatform::Windows,
                "https://builds.bepinex.dev/projects/bepinex_be/790/BepInEx-Unity.Mono-win-x86-6.0.0-be.790%2B4e98359.zip",
                "",
            ).with_filename("BepInEx-Unity.Mono-win-x86-6.0.0-be.790+4e98359.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.790-win-x86-il2cpp",
                "6.0.0-be.790",
                BuildChannel::BleedingEdge,
                RuntimeType::Il2Cpp,
                Architecture::X86,
                TargetPlatform::Windows,
                "https://builds.bepinex.dev/projects/bepinex_be/790/BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.790%2B4e98359.zip",
                "",
            ).with_filename("BepInEx-Unity.IL2CPP-win-x86-6.0.0-be.790+4e98359.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.790-linux-x64-mono",
                "6.0.0-be.790",
                BuildChannel::BleedingEdge,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::Linux,
                "https://builds.bepinex.dev/projects/bepinex_be/790/BepInEx-Unity.Mono-linux-x64-6.0.0-be.790%2B4e98359.zip",
                "",
            ).with_filename("BepInEx-Unity.Mono-linux-x64-6.0.0-be.790+4e98359.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.790-linux-x86-mono",
                "6.0.0-be.790",
                BuildChannel::BleedingEdge,
                RuntimeType::Mono,
                Architecture::X86,
                TargetPlatform::Linux,
                "https://builds.bepinex.dev/projects/bepinex_be/790/BepInEx-Unity.Mono-linux-x86-6.0.0-be.790%2B4e98359.zip",
                "",
            ).with_filename("BepInEx-Unity.Mono-linux-x86-6.0.0-be.790+4e98359.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.790-linux-x64-il2cpp",
                "6.0.0-be.790",
                BuildChannel::BleedingEdge,
                RuntimeType::Il2Cpp,
                Architecture::X64,
                TargetPlatform::Linux,
                "https://builds.bepinex.dev/projects/bepinex_be/790/BepInEx-Unity.IL2CPP-linux-x64-6.0.0-be.790%2B4e98359.zip",
                "",
            ).with_filename("BepInEx-Unity.IL2CPP-linux-x64-6.0.0-be.790+4e98359.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.790-macos-x64-mono",
                "6.0.0-be.790",
                BuildChannel::BleedingEdge,
                RuntimeType::Mono,
                Architecture::X64,
                TargetPlatform::MacOs,
                "https://builds.bepinex.dev/projects/bepinex_be/790/BepInEx-Unity.Mono-macos-x64-6.0.0-be.790%2B4e98359.zip",
                "",
            ).with_filename("BepInEx-Unity.Mono-macos-x64-6.0.0-be.790+4e98359.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.790-macos-x64-il2cpp",
                "6.0.0-be.790",
                BuildChannel::BleedingEdge,
                RuntimeType::Il2Cpp,
                Architecture::X64,
                TargetPlatform::MacOs,
                "https://builds.bepinex.dev/projects/bepinex_be/790/BepInEx-Unity.IL2CPP-macos-x64-6.0.0-be.790%2B4e98359.zip",
                "",
            ).with_filename("BepInEx-Unity.IL2CPP-macos-x64-6.0.0-be.790+4e98359.zip"),
            BepInExBuild::new(
                "bepinex-v6.0.0-be.790-macos-arm64-il2cpp",
                "6.0.0-be.790",
                BuildChannel::BleedingEdge,
                RuntimeType::Il2Cpp,
                Architecture::Arm64,
                TargetPlatform::MacOs,
                "https://builds.bepinex.dev/projects/bepinex_be/790/BepInEx-Unity.IL2CPP-macos-arm64-6.0.0-be.790%2B4e98359.zip",
                "",
            ).with_filename("BepInEx-Unity.IL2CPP-macos-arm64-6.0.0-be.790+4e98359.zip"),
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
        assert_eq!(custom.list_all().len(), 0);
        custom.add(b.clone());
        assert_eq!(custom.builds().len(), 1);
        assert_eq!(custom.list_all().len(), 1);
        assert_eq!(custom.get(&b.id).unwrap(), b);
    }
}
