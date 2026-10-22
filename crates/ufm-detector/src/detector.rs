use std::path::{Path, PathBuf};
use thiserror::Error;
use ufm_core::{
    Architecture, Confidence, EnvironmentReport, EvidenceItem, RuntimeType, TargetPlatform,
    UnityVersionInfo,
};

use crate::frameworks::detect_frameworks;
use crate::version::extract_unity_version_from_file;

#[derive(Debug, Error)]
pub enum DetectorError {
    #[error("Path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Detection error: {0}")]
    Other(String),
}

/// Detects a Unity game environment from a game root directory or executable path.
pub fn detect<P: AsRef<Path>>(target: P) -> Result<EnvironmentReport, DetectorError> {
    // Verify runtime layer readiness
    let _ = ufm_runtime::is_ready();

    let target_path = target.as_ref();
    if !target_path.exists() {
        return Err(DetectorError::PathNotFound(target_path.to_path_buf()));
    }

    let mut evidences = Vec::new();

    // 1. Resolve game root and data directory
    let (game_root, data_dir, dir_evidences) = resolve_game_paths(target_path);
    evidences.extend(dir_evidences);

    // 2. Extract Unity version
    let (unity_version, version_evidences) =
        extract_version_with_evidences(&game_root, data_dir.as_deref());
    evidences.extend(version_evidences);

    // 3. Detect runtime type (Mono vs IL2CPP) via ufm-runtime
    let (runtime_type, runtime_evidences) = detect_runtime(&game_root, data_dir.as_deref());
    evidences.extend(runtime_evidences);

    // 4. Detect architecture and target platform
    let (architecture, target_platform, arch_evidences) =
        detect_architecture_and_platform(&game_root, data_dir.as_deref());
    evidences.extend(arch_evidences);

    // 5. Detect modding frameworks (BepInEx, Doorstop, MelonLoader)
    let framework_result = detect_frameworks(&game_root);
    evidences.extend(framework_result.evidences);

    // 6. Calculate overall confidence
    let has_data_dir = data_dir.is_some();
    let has_unity_player =
        game_root.join("UnityPlayer.dll").is_file() || game_root.join("UnityPlayer.so").is_file();
    let has_version = unity_version.is_some();
    let overall_confidence =
        calculate_confidence(&evidences, has_data_dir, has_unity_player, has_version);

    Ok(EnvironmentReport {
        game_path: game_root.to_string_lossy().to_string(),
        data_dir: data_dir.map(|d| d.to_string_lossy().to_string()),
        unity_version,
        runtime_type,
        architecture,
        target_platform,
        installed_frameworks: framework_result.frameworks,
        evidences,
        overall_confidence,
    })
}

/// Resolves the game root directory and data directory candidate.
fn resolve_game_paths(target: &Path) -> (PathBuf, Option<PathBuf>, Vec<EvidenceItem>) {
    let mut evidences = Vec::new();

    if target.is_file() {
        let parent = target.parent().unwrap_or(target).to_path_buf();
        evidences.push(EvidenceItem {
            key: "target.executable".to_string(),
            description: format!("Target executable specified: {}", target.display()),
            confidence: Confidence::Confirmed,
            path: Some(target.to_string_lossy().to_string()),
        });

        // Check if <exe_stem>_Data exists in parent
        if let Some(stem) = target.file_stem().and_then(|s| s.to_str()) {
            let direct_data = parent.join(format!("{}_Data", stem));
            if direct_data.is_dir() {
                evidences.push(EvidenceItem {
                    key: "unity.data_dir".to_string(),
                    description: format!(
                        "Matching Unity data directory found: {}",
                        direct_data.display()
                    ),
                    confidence: Confidence::Confirmed,
                    path: Some(direct_data.to_string_lossy().to_string()),
                });
                return (parent, Some(direct_data), evidences);
            }
        }

        // Otherwise scan parent directory for any *_Data
        let (found_data, scan_evidences) = find_data_dir(&parent);
        evidences.extend(scan_evidences);
        return (parent, found_data, evidences);
    }

    // target is a directory
    let dir_name = target
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    // If target itself is a *_Data directory
    if dir_name.ends_with("_Data")
        || dir_name.ends_with("_data")
        || target.join("globalgamemanagers").is_file()
        || target.join("data.unity3d").is_file()
    {
        let parent = target.parent().unwrap_or(target).to_path_buf();
        evidences.push(EvidenceItem {
            key: "unity.data_dir".to_string(),
            description: format!("Target is Unity data directory: {}", target.display()),
            confidence: Confidence::Confirmed,
            path: Some(target.to_string_lossy().to_string()),
        });
        return (parent, Some(target.to_path_buf()), evidences);
    }

    // Otherwise target is the game root directory
    let (found_data, scan_evidences) = find_data_dir(target);
    evidences.extend(scan_evidences);
    (target.to_path_buf(), found_data, evidences)
}

/// Scans a game root directory for `*_Data` or asset files.
fn find_data_dir(root: &Path) -> (Option<PathBuf>, Vec<EvidenceItem>) {
    let mut evidences = Vec::new();
    let mut candidates = Vec::new();

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.ends_with("_Data")
                    || name_str.ends_with("_data")
                    || name_str.ends_with(".data")
                {
                    candidates.push(path);
                }
            }
        }
    }

    if candidates.is_empty() {
        // Check if globalgamemanagers or data.unity3d is placed directly in root
        if root.join("globalgamemanagers").is_file() || root.join("data.unity3d").is_file() {
            evidences.push(EvidenceItem {
                key: "unity.asset_file".to_string(),
                description: "Found Unity asset file in root directory".to_string(),
                confidence: Confidence::HighProbability,
                path: Some(root.to_string_lossy().to_string()),
            });
            return (Some(root.to_path_buf()), evidences);
        }
        return (None, evidences);
    }

    if candidates.len() == 1 {
        let data_dir = candidates.into_iter().next().unwrap();
        evidences.push(EvidenceItem {
            key: "unity.data_dir".to_string(),
            description: format!("Found Unity data directory: {}", data_dir.display()),
            confidence: Confidence::Confirmed,
            path: Some(data_dir.to_string_lossy().to_string()),
        });
        return (Some(data_dir), evidences);
    }

    // Multiple candidates found: check if one matches an executable name
    for cand in &candidates {
        let cand_name = cand.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let exe_stem = cand_name
            .strip_suffix("_Data")
            .or_else(|| cand_name.strip_suffix("_data"));
        if let Some(stem) = exe_stem {
            let exe_path = root.join(format!("{}.exe", stem));
            if exe_path.is_file() {
                evidences.push(EvidenceItem {
                    key: "unity.data_dir".to_string(),
                    description: format!("Found matching Unity data directory: {}", cand.display()),
                    confidence: Confidence::Confirmed,
                    path: Some(cand.to_string_lossy().to_string()),
                });
                return (Some(cand.clone()), evidences);
            }
        }
    }

    // Multiple data dirs and none uniquely matched -> Conflict
    evidences.push(EvidenceItem {
        key: "unity.data_dir_conflict".to_string(),
        description: format!(
            "Multiple ambiguous *_Data directories found: {:?}",
            candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
        ),
        confidence: Confidence::Conflict,
        path: None,
    });
    (Some(candidates[0].clone()), evidences)
}

/// Extracts Unity version from globalgamemanagers, data.unity3d, or UnityPlayer.dll.
fn extract_version_with_evidences(
    game_root: &Path,
    data_dir: Option<&Path>,
) -> (Option<UnityVersionInfo>, Vec<EvidenceItem>) {
    let mut evidences = Vec::new();

    // Check data_dir/globalgamemanagers
    if let Some(dir) = data_dir {
        let ggm = dir.join("globalgamemanagers");
        if ggm.is_file() {
            evidences.push(EvidenceItem {
                key: "unity.globalgamemanagers".to_string(),
                description: "Found globalgamemanagers asset file".to_string(),
                confidence: Confidence::Confirmed,
                path: Some(ggm.to_string_lossy().to_string()),
            });
            if let Some(ver) = extract_unity_version_from_file(&ggm) {
                evidences.push(EvidenceItem {
                    key: "unity.version".to_string(),
                    description: format!(
                        "Extracted Unity version from globalgamemanagers: {}",
                        ver.raw
                    ),
                    confidence: Confidence::Confirmed,
                    path: Some(ggm.to_string_lossy().to_string()),
                });
                return (Some(ver), evidences);
            }
        }

        // Check data_dir/data.unity3d
        let data_unity3d = dir.join("data.unity3d");
        if data_unity3d.is_file() {
            evidences.push(EvidenceItem {
                key: "unity.data_unity3d".to_string(),
                description: "Found data.unity3d asset bundle".to_string(),
                confidence: Confidence::Confirmed,
                path: Some(data_unity3d.to_string_lossy().to_string()),
            });
            if let Some(ver) = extract_unity_version_from_file(&data_unity3d) {
                evidences.push(EvidenceItem {
                    key: "unity.version".to_string(),
                    description: format!("Extracted Unity version from data.unity3d: {}", ver.raw),
                    confidence: Confidence::Confirmed,
                    path: Some(data_unity3d.to_string_lossy().to_string()),
                });
                return (Some(ver), evidences);
            }
        }

        // Check data_dir/mainData
        let main_data = dir.join("mainData");
        if main_data.is_file() {
            if let Some(ver) = extract_unity_version_from_file(&main_data) {
                evidences.push(EvidenceItem {
                    key: "unity.version".to_string(),
                    description: format!("Extracted Unity version from mainData: {}", ver.raw),
                    confidence: Confidence::Confirmed,
                    path: Some(main_data.to_string_lossy().to_string()),
                });
                return (Some(ver), evidences);
            }
        }
    }

    // Check game_root/UnityPlayer.dll
    let unity_player = game_root.join("UnityPlayer.dll");
    if unity_player.is_file() {
        evidences.push(EvidenceItem {
            key: "unity.player_dll".to_string(),
            description: "Found UnityPlayer.dll".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(unity_player.to_string_lossy().to_string()),
        });
        if let Some(ver) = extract_unity_version_from_file(&unity_player) {
            evidences.push(EvidenceItem {
                key: "unity.version".to_string(),
                description: format!("Extracted Unity version from UnityPlayer.dll: {}", ver.raw),
                confidence: Confidence::Confirmed,
                path: Some(unity_player.to_string_lossy().to_string()),
            });
            return (Some(ver), evidences);
        }
    }

    // Check game_root/UnityPlayer.so
    let unity_player_so = game_root.join("UnityPlayer.so");
    if unity_player_so.is_file() {
        evidences.push(EvidenceItem {
            key: "unity.player_so".to_string(),
            description: "Found UnityPlayer.so".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(unity_player_so.to_string_lossy().to_string()),
        });
        if let Some(ver) = extract_unity_version_from_file(&unity_player_so) {
            evidences.push(EvidenceItem {
                key: "unity.version".to_string(),
                description: format!("Extracted Unity version from UnityPlayer.so: {}", ver.raw),
                confidence: Confidence::Confirmed,
                path: Some(unity_player_so.to_string_lossy().to_string()),
            });
            return (Some(ver), evidences);
        }
    }

    (None, evidences)
}

/// Detects runtime type: Mono vs IL2CPP using ufm-runtime.
fn detect_runtime(game_root: &Path, data_dir: Option<&Path>) -> (RuntimeType, Vec<EvidenceItem>) {
    match ufm_runtime::detect_runtime_with_data_dir(game_root, data_dir) {
        Ok(analysis) => (analysis.runtime_type, analysis.evidences),
        Err(_) => (RuntimeType::Unknown, Vec::new()),
    }
}

/// Detects architecture and target platform by inspecting binaries in game root.
fn detect_architecture_and_platform(
    game_root: &Path,
    data_dir: Option<&Path>,
) -> (Architecture, TargetPlatform, Vec<EvidenceItem>) {
    let mut evidences = Vec::new();
    let mut detected_arch = Architecture::Unknown;
    let mut detected_plat = TargetPlatform::Unknown;

    // Check UnityCrashHandler64.exe / UnityCrashHandler32.exe
    let crash64 = game_root.join("UnityCrashHandler64.exe");
    let crash32 = game_root.join("UnityCrashHandler32.exe");
    if crash64.is_file() {
        detected_arch = Architecture::X64;
        detected_plat = TargetPlatform::Windows;
        evidences.push(EvidenceItem {
            key: "binary.crash_handler".to_string(),
            description: "UnityCrashHandler64.exe indicates x64 Windows binary".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(crash64.to_string_lossy().to_string()),
        });
    } else if crash32.is_file() {
        detected_arch = Architecture::X86;
        detected_plat = TargetPlatform::Windows;
        evidences.push(EvidenceItem {
            key: "binary.crash_handler".to_string(),
            description: "UnityCrashHandler32.exe indicates x86 Windows binary".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(crash32.to_string_lossy().to_string()),
        });
    }

    // Call ufm_runtime architecture detection
    if let Ok(arch) = ufm_runtime::detect_game_architecture(game_root) {
        if arch != Architecture::Unknown {
            detected_arch = arch;
        }
    }

    // Determine platform by checking UnityPlayer or executables
    let unity_player = game_root.join("UnityPlayer.dll");
    let unity_player_so = game_root.join("UnityPlayer.so");
    if unity_player.is_file() {
        detected_plat = TargetPlatform::Windows;
        if detected_arch == Architecture::Unknown {
            if let Ok(info) = ufm_runtime::detect_file_binary_info(&unity_player) {
                detected_arch = info.architecture;
            }
        }
    } else if unity_player_so.is_file() {
        detected_plat = TargetPlatform::Linux;
        if detected_arch == Architecture::Unknown {
            if let Ok(info) = ufm_runtime::detect_file_binary_info(&unity_player_so) {
                detected_arch = info.architecture;
            }
        }
    }

    // If platform is still unknown, check any .exe files in root
    if detected_plat == TargetPlatform::Unknown && game_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(game_root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext == "exe" {
                        detected_plat = TargetPlatform::Windows;
                        break;
                    } else if ext == "x86_64" || ext == "x86" || ext == "so" {
                        detected_plat = TargetPlatform::Linux;
                        break;
                    }
                }
            }
        }
    }

    // Check if macOS .app structure
    if game_root.join("Contents/MacOS").is_dir()
        || data_dir
            .map(|d| d.to_string_lossy().contains(".app/"))
            .unwrap_or(false)
    {
        detected_plat = TargetPlatform::MacOs;
    }

    if detected_arch != Architecture::Unknown {
        evidences.push(EvidenceItem {
            key: "binary.architecture".to_string(),
            description: format!("Detected game architecture: {:?}", detected_arch),
            confidence: Confidence::Confirmed,
            path: None,
        });
    }

    if detected_plat != TargetPlatform::Unknown {
        evidences.push(EvidenceItem {
            key: "binary.platform".to_string(),
            description: format!("Detected target platform: {:?}", detected_plat),
            confidence: Confidence::Confirmed,
            path: None,
        });
    }

    (detected_arch, detected_plat, evidences)
}

/// Computes overall confidence according to discovered evidence items.
pub fn calculate_confidence(
    evidences: &[EvidenceItem],
    has_data_dir: bool,
    has_unity_player: bool,
    has_version: bool,
) -> Confidence {
    // 1. If any evidence item has Conflict status, overall is Conflict
    if evidences
        .iter()
        .any(|e| e.confidence == Confidence::Conflict)
    {
        return Confidence::Conflict;
    }

    // Count confirmed Unity indicators
    let confirmed_count = evidences
        .iter()
        .filter(|e| e.confidence == Confidence::Confirmed && e.key.starts_with("unity."))
        .count();

    // 2. Confirmed: Data directory + (UnityPlayer or game executable or version) + confirmed unity assets
    if has_data_dir && (has_unity_player || has_version || confirmed_count >= 2) {
        return Confidence::Confirmed;
    }

    // 3. High Probability: Found Unity data directory OR UnityPlayer.dll alone
    if has_data_dir || has_unity_player || has_version || confirmed_count >= 1 {
        return Confidence::HighProbability;
    }

    // 4. Unknown: No recognizable Unity evidence
    Confidence::Unknown
}
