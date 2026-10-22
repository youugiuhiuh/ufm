use std::path::Path;
use ufm_core::{Confidence, EvidenceItem};

#[derive(Debug, Clone, Default)]
pub struct FrameworkDetection {
    pub frameworks: Vec<String>,
    pub evidences: Vec<EvidenceItem>,
}

/// Detects modding frameworks (BepInEx, Doorstop, MelonLoader) in the game root directory.
pub fn detect_frameworks<P: AsRef<Path>>(game_root: P) -> FrameworkDetection {
    let root = game_root.as_ref();
    let mut frameworks = Vec::new();
    let mut evidences = Vec::new();

    // 1. BepInEx detection
    let bepinex_dir = root.join("BepInEx");
    let bepinex_core_dll = bepinex_dir.join("core").join("BepInEx.Core.dll");
    let bepinex_preloader_dll = bepinex_dir.join("core").join("BepInEx.Preloader.dll");

    if bepinex_core_dll.is_file() {
        frameworks.push("BepInEx".to_string());
        evidences.push(EvidenceItem {
            key: "framework.bepinex".to_string(),
            description: "BepInEx framework detected (BepInEx.Core.dll found)".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(bepinex_core_dll.to_string_lossy().to_string()),
        });
    } else if bepinex_preloader_dll.is_file() {
        frameworks.push("BepInEx".to_string());
        evidences.push(EvidenceItem {
            key: "framework.bepinex".to_string(),
            description: "BepInEx preloader detected (BepInEx.Preloader.dll found)".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(bepinex_preloader_dll.to_string_lossy().to_string()),
        });
    } else if bepinex_dir.is_dir() {
        frameworks.push("BepInEx".to_string());
        evidences.push(EvidenceItem {
            key: "framework.bepinex".to_string(),
            description: "BepInEx directory found".to_string(),
            confidence: Confidence::HighProbability,
            path: Some(bepinex_dir.to_string_lossy().to_string()),
        });
    }

    // 2. Doorstop detection
    let doorstop_ini = root.join("doorstop_config.ini");
    let winhttp_dll = root.join("winhttp.dll");
    let version_dll = root.join("version.dll");

    let mut doorstop_detected = false;
    if doorstop_ini.is_file() {
        doorstop_detected = true;
        evidences.push(EvidenceItem {
            key: "framework.doorstop".to_string(),
            description: "Doorstop configuration found (doorstop_config.ini)".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(doorstop_ini.to_string_lossy().to_string()),
        });
    }

    if winhttp_dll.is_file() {
        evidences.push(EvidenceItem {
            key: "hook.proxy_dll".to_string(),
            description: "Proxy hook DLL found: winhttp.dll".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(winhttp_dll.to_string_lossy().to_string()),
        });
        // If winhttp.dll exists alongside BepInEx, it's typically Doorstop
        if !doorstop_detected && bepinex_dir.is_dir() {
            doorstop_detected = true;
        }
    }

    if version_dll.is_file() {
        evidences.push(EvidenceItem {
            key: "hook.proxy_dll".to_string(),
            description: "Proxy hook DLL found: version.dll".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(version_dll.to_string_lossy().to_string()),
        });
    }

    if doorstop_detected && !frameworks.contains(&"Doorstop".to_string()) {
        frameworks.push("Doorstop".to_string());
    }

    // 3. MelonLoader detection
    let melon_dir = root.join("MelonLoader");
    let melon_dll = melon_dir.join("MelonLoader.dll");
    let melon_net6_dll = melon_dir.join("net6").join("MelonLoader.dll");

    if melon_dll.is_file() {
        frameworks.push("MelonLoader".to_string());
        evidences.push(EvidenceItem {
            key: "framework.melonloader".to_string(),
            description: "MelonLoader framework detected (MelonLoader.dll found)".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(melon_dll.to_string_lossy().to_string()),
        });
    } else if melon_net6_dll.is_file() {
        frameworks.push("MelonLoader".to_string());
        evidences.push(EvidenceItem {
            key: "framework.melonloader".to_string(),
            description: "MelonLoader framework detected (net6/MelonLoader.dll found)".to_string(),
            confidence: Confidence::Confirmed,
            path: Some(melon_net6_dll.to_string_lossy().to_string()),
        });
    } else if melon_dir.is_dir() {
        frameworks.push("MelonLoader".to_string());
        evidences.push(EvidenceItem {
            key: "framework.melonloader".to_string(),
            description: "MelonLoader directory found".to_string(),
            confidence: Confidence::HighProbability,
            path: Some(melon_dir.to_string_lossy().to_string()),
        });
    }

    FrameworkDetection {
        frameworks,
        evidences,
    }
}
