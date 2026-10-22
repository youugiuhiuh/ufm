//! Mono vs IL2CPP runtime detection and game architecture analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use ufm_core::{Architecture, Confidence, EvidenceItem, RuntimeType};

use crate::arch::detect_file_architecture;

/// Result of runtime analysis containing detected type, overall confidence, and supporting evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeAnalysis {
    pub runtime_type: RuntimeType,
    pub confidence: Confidence,
    pub evidences: Vec<EvidenceItem>,
}

/// Detect the runtime type (Mono vs IL2CPP) of a Unity game directory.
pub fn detect_runtime<P: AsRef<Path>>(game_dir: P) -> std::io::Result<RuntimeAnalysis> {
    detect_runtime_with_data_dir(game_dir, None::<&Path>)
}

/// Detect runtime type with optional explicit data directory override.
pub fn detect_runtime_with_data_dir<P1: AsRef<Path>, P2: AsRef<Path>>(
    game_root: P1,
    data_dir: Option<P2>,
) -> std::io::Result<RuntimeAnalysis> {
    let root_path = game_root.as_ref();
    if !root_path.exists() {
        return Ok(RuntimeAnalysis {
            runtime_type: RuntimeType::Unknown,
            confidence: Confidence::Unknown,
            evidences: Vec::new(),
        });
    }

    let root = if root_path.is_file() {
        root_path.parent().unwrap_or(root_path)
    } else {
        root_path
    };

    let mut evidences = Vec::new();
    let mut seen = HashSet::new();

    // 1. Resolve Data directory
    let resolved_data_dir = match data_dir {
        Some(d) => Some(d.as_ref().to_path_buf()),
        None => find_unity_data_dir(root),
    };

    // 2. Scan Root Directory
    scan_root_directory(root, &mut evidences, &mut seen);

    // 3. Scan Data Directory (if found)
    if let Some(ref data) = resolved_data_dir {
        scan_data_directory(data, &mut evidences, &mut seen);
    }

    // 4. macOS bundle scan
    let macos_frameworks = root.join("Contents/Frameworks");
    if macos_frameworks.is_dir() {
        scan_macos_frameworks(&macos_frameworks, &mut evidences, &mut seen);
    }

    Ok(evaluate_evidences(evidences))
}

/// Detect runtime from a collection of relative or absolute file/directory paths (pure in-memory analysis).
pub fn detect_runtime_from_paths<P: AsRef<Path>>(paths: &[P]) -> RuntimeAnalysis {
    let mut evidences = Vec::new();
    let mut seen = HashSet::new();

    for p in paths {
        let path = p.as_ref();
        let path_str = path.to_string_lossy();
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        // GameAssembly binary
        if matches!(
            file_name.as_str(),
            "gameassembly.dll" | "gameassembly.so" | "gameassembly.dylib"
        ) {
            push_evidence(
                &mut evidences,
                &mut seen,
                "game_assembly",
                format!("IL2CPP native assembly found: {}", path.display()),
                Confidence::Confirmed,
                Some(&path_str),
            );
        }

        // IL2CPP Metadata
        if file_name == "global-metadata.dat" {
            push_evidence(
                &mut evidences,
                &mut seen,
                "il2cpp_metadata",
                format!("IL2CPP global-metadata.dat found: {}", path.display()),
                Confidence::Confirmed,
                Some(&path_str),
            );
        }

        // IL2CPP Shared Library
        if matches!(file_name.as_str(), "libil2cpp.so" | "libil2cpp.dylib") {
            push_evidence(
                &mut evidences,
                &mut seen,
                "il2cpp_library",
                format!("IL2CPP runtime library found: {}", path.display()),
                Confidence::Confirmed,
                Some(&path_str),
            );
        }

        // IL2CPP Data directory
        if file_name == "il2cpp_data"
            || path.components().any(|c| {
                c.as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("il2cpp_data")
            })
        {
            push_evidence(
                &mut evidences,
                &mut seen,
                "il2cpp_data_dir",
                format!("IL2CPP data directory found: {}", path.display()),
                Confidence::HighProbability,
                Some(&path_str),
            );
        }

        // Mono runtime libraries
        if matches!(
            file_name.as_str(),
            "mono-2.0-bdwgc.dll"
                | "libmonobdwgc-2.0.so"
                | "mono.dll"
                | "libmono.so"
                | "libmonosgen-2.0.so"
                | "monoembedruntime"
        ) {
            push_evidence(
                &mut evidences,
                &mut seen,
                "mono_runtime_lib",
                format!("Mono runtime library found: {}", path.display()),
                Confidence::Confirmed,
                Some(&path_str),
            );
        }

        // MonoBleedingEdge directory
        if file_name.eq_ignore_ascii_case("monobleedingedge")
            || path.components().any(|c| {
                c.as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("monobleedingedge")
            })
        {
            push_evidence(
                &mut evidences,
                &mut seen,
                "mono_bleeding_edge_dir",
                format!("MonoBleedingEdge directory found: {}", path.display()),
                Confidence::Confirmed,
                Some(&path_str),
            );
        }

        // Managed C# assemblies
        if file_name == "assembly-csharp.dll" {
            push_evidence(
                &mut evidences,
                &mut seen,
                "assembly_csharp",
                format!("Managed Assembly-CSharp.dll found: {}", path.display()),
                Confidence::HighProbability,
                Some(&path_str),
            );
        }

        if file_name == "mscorlib.dll" {
            push_evidence(
                &mut evidences,
                &mut seen,
                "mscorlib",
                format!("Managed mscorlib.dll found: {}", path.display()),
                Confidence::HighProbability,
                Some(&path_str),
            );
        }

        // Managed directory
        if file_name.eq_ignore_ascii_case("managed")
            || path.components().any(|c| {
                c.as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("managed")
            })
        {
            push_evidence(
                &mut evidences,
                &mut seen,
                "managed_dir",
                format!("Managed directory found: {}", path.display()),
                Confidence::HighProbability,
                Some(&path_str),
            );
        }

        // Mono directory
        if file_name.eq_ignore_ascii_case("mono")
            || path
                .components()
                .any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case("mono"))
        {
            push_evidence(
                &mut evidences,
                &mut seen,
                "mono_dir",
                format!("Mono directory found: {}", path.display()),
                Confidence::HighProbability,
                Some(&path_str),
            );
        }
    }

    evaluate_evidences(evidences)
}

/// Detect the binary architecture of the primary executable in a game directory.
pub fn detect_game_architecture<P: AsRef<Path>>(game_dir: P) -> std::io::Result<Architecture> {
    let path = game_dir.as_ref();
    if !path.exists() {
        return Ok(Architecture::Unknown);
    }

    if path.is_file() {
        return detect_file_architecture(path);
    }

    // 1. Check macOS Contents/MacOS
    let macos_dir = path.join("Contents/MacOS");
    if macos_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&macos_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let arch = detect_file_architecture(&p)?;
                    if arch != Architecture::Unknown {
                        return Ok(arch);
                    }
                }
            }
        }
    }

    // 2. Check root directory for executables
    let mut candidate_exes = Vec::new();
    let mut candidate_libs = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }

            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            if name.starts_with("unitycrashhandler") {
                continue;
            }

            if name.ends_with(".exe") || name.ends_with(".x86_64") || name.ends_with(".x86") {
                candidate_exes.push(p);
            } else if matches!(
                name.as_str(),
                "gameassembly.dll" | "gameassembly.so" | "mono-2.0-bdwgc.dll"
            ) {
                candidate_libs.push(p);
            }
        }
    }

    // First try standard game executables
    for exe in candidate_exes {
        let arch = detect_file_architecture(&exe)?;
        if arch != Architecture::Unknown {
            return Ok(arch);
        }
    }

    // Fall back to runtime libraries
    for lib in candidate_libs {
        let arch = detect_file_architecture(&lib)?;
        if arch != Architecture::Unknown {
            return Ok(arch);
        }
    }

    Ok(Architecture::Unknown)
}

fn find_unity_data_dir(root: &Path) -> Option<PathBuf> {
    // If root itself is already a data directory
    if root.join("Managed").exists() || root.join("il2cpp_data").exists() {
        return Some(root.to_path_buf());
    }

    // Check for macOS app bundle structure: Contents/Resources/Data
    let mac_data = root.join("Contents/Resources/Data");
    if mac_data.is_dir() {
        return Some(mac_data);
    }

    // Check immediate subdirectories for *_Data or Data
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with("_Data") || name == "Data" {
                    return Some(path);
                }
            }
        }
    }

    None
}

fn scan_root_directory(root: &Path, evidences: &mut Vec<EvidenceItem>, seen: &mut HashSet<String>) {
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let lower_name = name.to_ascii_lowercase();

            if path.is_file() {
                if matches!(
                    lower_name.as_str(),
                    "gameassembly.dll" | "gameassembly.so" | "gameassembly.dylib"
                ) {
                    push_evidence(
                        evidences,
                        seen,
                        "game_assembly",
                        format!("IL2CPP native assembly found: {}", name),
                        Confidence::Confirmed,
                        Some(&path.to_string_lossy()),
                    );
                } else if matches!(lower_name.as_str(), "libil2cpp.so" | "libil2cpp.dylib") {
                    push_evidence(
                        evidences,
                        seen,
                        "il2cpp_library",
                        format!("IL2CPP runtime library found: {}", name),
                        Confidence::Confirmed,
                        Some(&path.to_string_lossy()),
                    );
                } else if matches!(
                    lower_name.as_str(),
                    "mono-2.0-bdwgc.dll"
                        | "libmonobdwgc-2.0.so"
                        | "mono.dll"
                        | "libmono.so"
                        | "libmonosgen-2.0.so"
                ) {
                    push_evidence(
                        evidences,
                        seen,
                        "mono_runtime_lib",
                        format!("Mono runtime library found: {}", name),
                        Confidence::Confirmed,
                        Some(&path.to_string_lossy()),
                    );
                }
            } else if path.is_dir() {
                if lower_name == "monobleedingedge" {
                    push_evidence(
                        evidences,
                        seen,
                        "mono_bleeding_edge_dir",
                        format!("MonoBleedingEdge directory found: {}", path.display()),
                        Confidence::Confirmed,
                        Some(&path.to_string_lossy()),
                    );
                } else if lower_name == "mono" {
                    push_evidence(
                        evidences,
                        seen,
                        "mono_dir",
                        format!("Mono directory found: {}", path.display()),
                        Confidence::HighProbability,
                        Some(&path.to_string_lossy()),
                    );
                }
            }
        }
    }
}

fn scan_data_directory(
    data_dir: &Path,
    evidences: &mut Vec<EvidenceItem>,
    seen: &mut HashSet<String>,
) {
    // 0. Direct files in data_dir
    if let Ok(entries) = fs::read_dir(data_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                let lower = name.to_ascii_lowercase();

                if matches!(
                    lower.as_str(),
                    "mono-2.0-bdwgc.dll"
                        | "libmonobdwgc-2.0.so"
                        | "mono.dll"
                        | "libmono.so"
                        | "libmonosgen-2.0.so"
                ) {
                    push_evidence(
                        evidences,
                        seen,
                        "mono_runtime_lib",
                        format!("Mono runtime library found: {}", name),
                        Confidence::Confirmed,
                        Some(&path.to_string_lossy()),
                    );
                } else if matches!(
                    lower.as_str(),
                    "gameassembly.dll" | "gameassembly.so" | "gameassembly.dylib"
                ) {
                    push_evidence(
                        evidences,
                        seen,
                        "game_assembly",
                        format!("IL2CPP native assembly found: {}", name),
                        Confidence::Confirmed,
                        Some(&path.to_string_lossy()),
                    );
                } else if matches!(lower.as_str(), "libil2cpp.so" | "libil2cpp.dylib") {
                    push_evidence(
                        evidences,
                        seen,
                        "il2cpp_library",
                        format!("IL2CPP runtime library found: {}", name),
                        Confidence::Confirmed,
                        Some(&path.to_string_lossy()),
                    );
                }
            }
        }
    }

    // 1. il2cpp_data
    let il2cpp_dir = data_dir.join("il2cpp_data");
    if il2cpp_dir.is_dir() {
        push_evidence(
            evidences,
            seen,
            "il2cpp_data_dir",
            format!("IL2CPP data directory found: {}", il2cpp_dir.display()),
            Confidence::HighProbability,
            Some(&il2cpp_dir.to_string_lossy()),
        );

        let metadata_file = il2cpp_dir.join("Metadata/global-metadata.dat");
        if metadata_file.is_file() {
            push_evidence(
                evidences,
                seen,
                "il2cpp_metadata",
                format!(
                    "IL2CPP global-metadata.dat found: {}",
                    metadata_file.display()
                ),
                Confidence::Confirmed,
                Some(&metadata_file.to_string_lossy()),
            );
        }
    }

    // 2. MonoBleedingEdge inside data_dir
    let mono_bleeding = data_dir.join("MonoBleedingEdge");
    if mono_bleeding.is_dir() {
        push_evidence(
            evidences,
            seen,
            "mono_bleeding_edge_dir",
            format!(
                "MonoBleedingEdge directory found: {}",
                mono_bleeding.display()
            ),
            Confidence::Confirmed,
            Some(&mono_bleeding.to_string_lossy()),
        );
    }

    // 3. Mono directory inside data_dir
    let mono_dir = data_dir.join("Mono");
    if mono_dir.is_dir() {
        push_evidence(
            evidences,
            seen,
            "mono_dir",
            format!("Mono directory found: {}", mono_dir.display()),
            Confidence::HighProbability,
            Some(&mono_dir.to_string_lossy()),
        );
    }

    // 4. Managed directory
    let managed_dir = data_dir.join("Managed");
    if managed_dir.is_dir() {
        push_evidence(
            evidences,
            seen,
            "managed_dir",
            format!("Managed directory found: {}", managed_dir.display()),
            Confidence::HighProbability,
            Some(&managed_dir.to_string_lossy()),
        );

        let csharp = managed_dir.join("Assembly-CSharp.dll");
        if csharp.is_file() {
            push_evidence(
                evidences,
                seen,
                "assembly_csharp",
                format!("Managed Assembly-CSharp.dll found: {}", csharp.display()),
                Confidence::HighProbability,
                Some(&csharp.to_string_lossy()),
            );
        }

        let mscorlib = managed_dir.join("mscorlib.dll");
        if mscorlib.is_file() {
            push_evidence(
                evidences,
                seen,
                "mscorlib",
                format!("Managed mscorlib.dll found: {}", mscorlib.display()),
                Confidence::HighProbability,
                Some(&mscorlib.to_string_lossy()),
            );
        }
    }
}

fn scan_macos_frameworks(
    frameworks_dir: &Path,
    evidences: &mut Vec<EvidenceItem>,
    seen: &mut HashSet<String>,
) {
    if let Ok(entries) = fs::read_dir(frameworks_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_ascii_lowercase();

            if lower.contains("gameassembly") {
                push_evidence(
                    evidences,
                    seen,
                    "game_assembly",
                    format!("IL2CPP framework found: {}", path.display()),
                    Confidence::Confirmed,
                    Some(&path.to_string_lossy()),
                );
            } else if lower.contains("monoembed") || lower.contains("monoruntime") {
                push_evidence(
                    evidences,
                    seen,
                    "mono_runtime_lib",
                    format!("Mono runtime framework found: {}", path.display()),
                    Confidence::Confirmed,
                    Some(&path.to_string_lossy()),
                );
            }
        }
    }
}

fn push_evidence(
    evidences: &mut Vec<EvidenceItem>,
    seen: &mut HashSet<String>,
    key: &str,
    description: String,
    confidence: Confidence,
    path: Option<&str>,
) {
    let dedup_key = format!("{}:{}", key, path.unwrap_or(""));
    if seen.insert(dedup_key) {
        evidences.push(EvidenceItem {
            key: key.to_string(),
            description,
            confidence,
            path: path.map(String::from),
        });
    }
}

fn evaluate_evidences(evidences: Vec<EvidenceItem>) -> RuntimeAnalysis {
    let mut il2cpp_confirmed = false;
    let mut il2cpp_probable = false;
    let mut mono_confirmed = false;
    let mut mono_probable = false;

    for ev in &evidences {
        match ev.key.as_str() {
            "game_assembly" | "il2cpp_metadata" | "il2cpp_library" => {
                if ev.confidence == Confidence::Confirmed {
                    il2cpp_confirmed = true;
                } else {
                    il2cpp_probable = true;
                }
            }
            "il2cpp_data_dir" => {
                il2cpp_probable = true;
            }
            "mono_runtime_lib" | "mono_bleeding_edge_dir" => {
                if ev.confidence == Confidence::Confirmed {
                    mono_confirmed = true;
                } else {
                    mono_probable = true;
                }
            }
            "mono_dir" | "managed_dir" | "assembly_csharp" | "mscorlib" => {
                mono_probable = true;
            }
            _ => {}
        }
    }

    let (runtime_type, confidence) = if il2cpp_confirmed && mono_confirmed {
        (RuntimeType::Unknown, Confidence::Conflict)
    } else if il2cpp_confirmed {
        (RuntimeType::Il2Cpp, Confidence::Confirmed)
    } else if mono_confirmed {
        (RuntimeType::Mono, Confidence::Confirmed)
    } else if il2cpp_probable && mono_probable {
        (RuntimeType::Unknown, Confidence::Conflict)
    } else if il2cpp_probable {
        (RuntimeType::Il2Cpp, Confidence::HighProbability)
    } else if mono_probable {
        (RuntimeType::Mono, Confidence::HighProbability)
    } else {
        (RuntimeType::Unknown, Confidence::Unknown)
    };

    RuntimeAnalysis {
        runtime_type,
        confidence,
        evidences,
    }
}
