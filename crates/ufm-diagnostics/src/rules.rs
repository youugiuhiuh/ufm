use crate::rule::{DiagnosticContext, DiagnosticRule};
use crate::types::{DiagnosticIssue, RiskLevel};

fn extract_quoted_name(line: &str) -> Option<String> {
    if let Some(start) = line.find('\'') {
        if let Some(end) = line[start + 1..].find('\'') {
            return Some(line[start + 1..start + 1 + end].to_string());
        }
    }
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start + 1..].find('"') {
            return Some(line[start + 1..start + 1 + end].to_string());
        }
    }
    None
}

/// Rule 1: DllNotFoundException
pub struct DllNotFoundRule;

impl DiagnosticRule for DllNotFoundRule {
    fn id(&self) -> &str {
        "RULE_DLL_NOT_FOUND"
    }

    fn name(&self) -> &str {
        "DLL Not Found Detector"
    }

    fn evaluate(&self, ctx: &DiagnosticContext) -> Vec<DiagnosticIssue> {
        let mut matched_lines = Vec::new();
        let mut missing_dlls = Vec::new();

        for line in &ctx.lines {
            if line.contains("DllNotFoundException")
                || line.contains("Unable to load DLL")
                || line.contains("Could not load file or assembly")
            {
                matched_lines.push(line.trim().to_string());
                if let Some(dll) = extract_quoted_name(line) {
                    if !missing_dlls.contains(&dll) {
                        missing_dlls.push(dll);
                    }
                }
            }
        }

        if matched_lines.is_empty() {
            return Vec::new();
        }

        let is_critical = missing_dlls
            .iter()
            .any(|d| d.to_lowercase().contains("doorstop") || d.to_lowercase().contains("mono"));

        let risk_level = if is_critical {
            RiskLevel::Critical
        } else {
            RiskLevel::High
        };

        let desc = if !missing_dlls.is_empty() {
            format!(
                "Required dynamic link library not found in search path: {}.",
                missing_dlls.join(", ")
            )
        } else {
            "A required native DLL or managed assembly could not be located by the game runtime."
                .to_string()
        };

        vec![DiagnosticIssue {
            rule_id: self.id().to_string(),
            title: "Missing Dynamic Link Library (DllNotFoundException)".to_string(),
            risk_level,
            description: desc,
            matched_lines,
            suggested_fix: "1. Install latest Microsoft Visual C++ Redistributable (both x64 and x86).\n2. Ensure all mod prerequisite native DLLs are present in the game root folder.\n3. Verify anti-virus software did not quarantine the required DLL.".to_string(),
            references: vec![
                "https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist".to_string(),
            ],
        }]
    }
}

/// Rule 2: BadImageFormatException
pub struct BadImageFormatRule;

impl DiagnosticRule for BadImageFormatRule {
    fn id(&self) -> &str {
        "RULE_BAD_IMAGE_FORMAT"
    }

    fn name(&self) -> &str {
        "Architecture / Bitness Mismatch Detector"
    }

    fn evaluate(&self, ctx: &DiagnosticContext) -> Vec<DiagnosticIssue> {
        let mut matched_lines = Vec::new();

        for line in &ctx.lines {
            if line.contains("BadImageFormatException")
                || line.contains("An attempt was made to load a program with an incorrect format")
                || line.contains("0x8007000B")
            {
                matched_lines.push(line.trim().to_string());
            }
        }

        if matched_lines.is_empty() {
            return Vec::new();
        }

        let arch_info = if let Some(env) = ctx.environment {
            format!(
                " Detected target environment architecture: {:?}.",
                env.architecture
            )
        } else {
            String::new()
        };

        vec![DiagnosticIssue {
            rule_id: self.id().to_string(),
            title: "Architecture / Bitness Mismatch (BadImageFormatException)".to_string(),
            risk_level: RiskLevel::Critical,
            description: format!(
                "The game runtime attempted to load a binary with an incorrect architecture format (e.g. 32-bit vs 64-bit mismatch).{}",
                arch_info
            ),
            matched_lines,
            suggested_fix: "1. Check the architecture (x64 vs x86) of the game executable.\n2. Reinstall the mod loader and plugins matching the exact architecture of your game.\n3. Never mix 32-bit preloader DLLs with 64-bit Unity runtimes.".to_string(),
            references: vec![
                "https://docs.unity3d.com/Manual/PluginsForDesktop.html".to_string(),
            ],
        }]
    }
}

/// Rule 3: EntryPointNotFoundException
pub struct EntryPointNotFoundRule;

impl DiagnosticRule for EntryPointNotFoundRule {
    fn id(&self) -> &str {
        "RULE_ENTRY_POINT_NOT_FOUND"
    }

    fn name(&self) -> &str {
        "DLL Entry Point Detector"
    }

    fn evaluate(&self, ctx: &DiagnosticContext) -> Vec<DiagnosticIssue> {
        let mut matched_lines = Vec::new();

        for line in &ctx.lines {
            if line.contains("EntryPointNotFoundException")
                || line.contains("Unable to find an entry point named")
            {
                matched_lines.push(line.trim().to_string());
            }
        }

        if matched_lines.is_empty() {
            return Vec::new();
        }

        vec![DiagnosticIssue {
            rule_id: self.id().to_string(),
            title: "Missing Entry Point in DLL (EntryPointNotFoundException)".to_string(),
            risk_level: RiskLevel::High,
            description: "A specified function or entry point was not found inside the loaded DLL. This usually indicates an outdated or incompatible version of a native library.".to_string(),
            matched_lines,
            suggested_fix: "1. Update the mod to match the current version of the game.\n2. Ensure no conflicting or leftover versions of the DLL exist in the game or plugin directories.\n3. Reinstall the mod framework to restore original library versions.".to_string(),
            references: vec![],
        }]
    }
}

/// Rule 4: DoorstopConfigRule
pub struct DoorstopConfigRule;

impl DiagnosticRule for DoorstopConfigRule {
    fn id(&self) -> &str {
        "RULE_DOORSTOP_CONFIG"
    }

    fn name(&self) -> &str {
        "Doorstop Configuration Detector"
    }

    fn evaluate(&self, ctx: &DiagnosticContext) -> Vec<DiagnosticIssue> {
        let mut matched_lines = Vec::new();
        let mut disabled = false;
        let mut missing_assembly = false;
        let mut init_failed = false;

        for line in &ctx.lines {
            let lower = line.to_lowercase();
            if lower.contains("doorstop") || lower.contains("unitydoorstop") {
                if lower.contains("enabled=false")
                    || lower.contains("enabled = false")
                    || lower.contains("doorstop is disabled")
                {
                    matched_lines.push(line.trim().to_string());
                    disabled = true;
                } else if lower.contains("failed to find target assembly")
                    || lower.contains("target_assembly does not exist")
                    || lower.contains("targetassembly is empty")
                {
                    matched_lines.push(line.trim().to_string());
                    missing_assembly = true;
                } else if lower.contains("initialization failed")
                    || lower.contains("could not find doorstop.dll")
                    || lower.contains("doorstop was not loaded")
                {
                    matched_lines.push(line.trim().to_string());
                    init_failed = true;
                }
            }
        }

        if matched_lines.is_empty() {
            return Vec::new();
        }

        let desc = if disabled {
            "Doorstop mod preloader is explicitly disabled in doorstop.ini."
        } else if missing_assembly {
            "Doorstop target_assembly configuration points to a non-existent or empty path."
        } else if init_failed {
            "Doorstop failed to initialize or hook the Unity engine boot process."
        } else {
            "Doorstop configuration error detected."
        };

        vec![DiagnosticIssue {
            rule_id: self.id().to_string(),
            title: "Doorstop Preloader Configuration Error".to_string(),
            risk_level: RiskLevel::High,
            description: desc.to_string(),
            matched_lines,
            suggested_fix: "1. Open 'doorstop.ini' in the game root folder.\n2. Ensure 'enabled=true' under the [UnityDoorstop] section.\n3. Verify 'target_assembly' points to an existing DLL path (e.g., 'BepInEx\\core\\BepInEx.Preloader.dll').\n4. Confirm 'doorstop.dll' (or 'version.dll' / 'winhttp.dll') is present in the game executable folder.".to_string(),
            references: vec![
                "https://github.com/NeighTools/UnityDoorstop".to_string(),
            ],
        }]
    }
}

/// Rule 5: HarmonyPatchRule
pub struct HarmonyPatchRule;

impl DiagnosticRule for HarmonyPatchRule {
    fn id(&self) -> &str {
        "RULE_HARMONY_PATCH_FAILURE"
    }

    fn name(&self) -> &str {
        "Harmony Patch Conflict Detector"
    }

    fn evaluate(&self, ctx: &DiagnosticContext) -> Vec<DiagnosticIssue> {
        let mut matched_lines = Vec::new();

        for line in &ctx.lines {
            if line.contains("HarmonyLib.PatchException")
                || line.contains("Failed to patch method")
                || line.contains("PatchProcessor") && line.contains("failed")
            {
                matched_lines.push(line.trim().to_string());
            }
        }

        if matched_lines.is_empty() {
            return Vec::new();
        }

        vec![DiagnosticIssue {
            rule_id: self.id().to_string(),
            title: "Harmony Patch Conflict or Failure".to_string(),
            risk_level: RiskLevel::High,
            description: "A Harmony method patch failed to apply. This usually indicates an outdated mod targeting obsolete method signatures, or two mods hooking the same method incompatibly.".to_string(),
            matched_lines,
            suggested_fix: "1. Check the stack trace to identify which mod emitted the Harmony patch.\n2. Disable conflicting or outdated mods.\n3. Update the mod to a version that supports the current game patch.".to_string(),
            references: vec![
                "https://harmony.pardeike.net/".to_string(),
            ],
        }]
    }
}

/// Rule 6: MissingMemberRule
pub struct MissingMemberRule;

impl DiagnosticRule for MissingMemberRule {
    fn id(&self) -> &str {
        "RULE_MISSING_MEMBER"
    }

    fn name(&self) -> &str {
        "Missing Member / TypeLoad Detector"
    }

    fn evaluate(&self, ctx: &DiagnosticContext) -> Vec<DiagnosticIssue> {
        let mut matched_lines = Vec::new();

        for line in &ctx.lines {
            if line.contains("MissingMethodException")
                || line.contains("MissingFieldException")
                || line.contains("TypeLoadException")
            {
                matched_lines.push(line.trim().to_string());
            }
        }

        if matched_lines.is_empty() {
            return Vec::new();
        }

        vec![DiagnosticIssue {
            rule_id: self.id().to_string(),
            title: "API Incompatibility (Missing Member or TypeLoad)".to_string(),
            risk_level: RiskLevel::High,
            description: "A mod attempted to invoke a method, field, or type that does not exist in the loaded assemblies. This occurs when a mod is built for a different game update.".to_string(),
            matched_lines,
            suggested_fix: "1. Update the mod to the latest version matching your game build.\n2. Verify all mod dependencies and shared libraries (e.g. 0Harmony.dll) are up-to-date.".to_string(),
            references: vec![],
        }]
    }
}
