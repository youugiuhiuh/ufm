//! ufm-runtime crate
//! Mono / IL2CPP runtime detection and executable architecture analysis.

pub mod arch;
pub mod detector;

pub use arch::{
    detect_architecture_from_bytes, detect_binary_info, detect_file_architecture,
    detect_file_binary_info, BinaryFormat, BinaryInfo,
};
pub use detector::{
    detect_game_architecture, detect_runtime, detect_runtime_from_paths,
    detect_runtime_with_data_dir, RuntimeAnalysis,
};
pub use ufm_core::{Architecture, Confidence, EvidenceItem, RuntimeType, TargetPlatform};

/// Basic liveness check for crate readiness.
pub fn is_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Self {
            let count = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "ufm_test_{}_{}_{}",
                prefix,
                std::process::id(),
                count
            ));
            fs::create_dir_all(&path).expect("Failed to create test temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    // Helper functions to synthesize test binary headers
    fn make_pe_header(machine: u16, pe_offset: u32) -> Vec<u8> {
        let size = (pe_offset as usize + 24).max(64);
        let mut data = vec![0u8; size];
        data[0] = b'M';
        data[1] = b'Z';
        data[0x3C..0x40].copy_from_slice(&pe_offset.to_le_bytes());
        let pe_idx = pe_offset as usize;
        data[pe_idx..pe_idx + 4].copy_from_slice(b"PE\0\0");
        data[pe_idx + 4..pe_idx + 6].copy_from_slice(&machine.to_le_bytes());
        data
    }

    fn make_elf_header(machine: u16, is_64: bool, is_little_endian: bool) -> Vec<u8> {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
        data[4] = if is_64 { 2 } else { 1 };
        data[5] = if is_little_endian { 1 } else { 2 };
        data[6] = 1;
        if is_little_endian {
            data[0x12..0x14].copy_from_slice(&machine.to_le_bytes());
        } else {
            data[0x12..0x14].copy_from_slice(&machine.to_be_bytes());
        }
        data
    }

    fn make_macho_single(cputype: u32, is_64: bool, is_little_endian: bool) -> Vec<u8> {
        let mut data = vec![0u8; 16];
        let magic = match (is_64, is_little_endian) {
            (false, false) => [0xFE, 0xED, 0xFA, 0xCE], // MH_MAGIC
            (false, true) => [0xCE, 0xFA, 0xED, 0xFE],  // MH_CIGAM
            (true, false) => [0xFE, 0xED, 0xFA, 0xCF],  // MH_MAGIC_64
            (true, true) => [0xCF, 0xFA, 0xED, 0xFE],   // MH_CIGAM_64
        };
        data[0..4].copy_from_slice(&magic);
        if is_little_endian {
            data[4..8].copy_from_slice(&cputype.to_le_bytes());
        } else {
            data[4..8].copy_from_slice(&cputype.to_be_bytes());
        }
        data
    }

    fn make_macho_fat(cputypes: &[u32], is_little_endian: bool) -> Vec<u8> {
        let nfat = cputypes.len() as u32;
        let mut data = vec![0u8; 8 + cputypes.len() * 20];
        let magic = if is_little_endian {
            [0xBE, 0xBA, 0xFE, 0xCA] // FAT_CIGAM
        } else {
            [0xCA, 0xFE, 0xBA, 0xBE] // FAT_MAGIC
        };
        data[0..4].copy_from_slice(&magic);
        if is_little_endian {
            data[4..8].copy_from_slice(&nfat.to_le_bytes());
        } else {
            data[4..8].copy_from_slice(&nfat.to_be_bytes());
        }

        for (i, &cpu) in cputypes.iter().enumerate() {
            let offset = 8 + i * 20;
            if is_little_endian {
                data[offset..offset + 4].copy_from_slice(&cpu.to_le_bytes());
            } else {
                data[offset..offset + 4].copy_from_slice(&cpu.to_be_bytes());
            }
        }
        data
    }

    #[test]
    fn test_ready() {
        assert!(is_ready());
    }

    #[test]
    fn test_pe_architectures() {
        // x86 (0x014c)
        let pe_x86 = make_pe_header(0x014C, 0x40);
        let info = detect_binary_info(&pe_x86);
        assert_eq!(info.format, BinaryFormat::Pe);
        assert_eq!(info.architecture, Architecture::X86);

        // x64 (0x8664)
        let pe_x64 = make_pe_header(0x8664, 0x40);
        let info = detect_binary_info(&pe_x64);
        assert_eq!(info.format, BinaryFormat::Pe);
        assert_eq!(info.architecture, Architecture::X64);

        // ARM64 (0xAA64)
        let pe_arm64 = make_pe_header(0xAA64, 0x40);
        let info = detect_binary_info(&pe_arm64);
        assert_eq!(info.format, BinaryFormat::Pe);
        assert_eq!(info.architecture, Architecture::Arm64);

        // Unknown machine (0x0200)
        let pe_unk = make_pe_header(0x0200, 0x40);
        let info = detect_binary_info(&pe_unk);
        assert_eq!(info.format, BinaryFormat::Pe);
        assert_eq!(info.architecture, Architecture::Unknown);

        // Invalid / Truncated PE
        let info_truncated = detect_binary_info(&[b'M', b'Z', 0, 0]);
        assert_eq!(info_truncated.format, BinaryFormat::Pe);
        assert_eq!(info_truncated.architecture, Architecture::Unknown);
    }

    #[test]
    fn test_pe_file_large_offset() {
        let temp = TempDir::new("pe_seek");
        let file_path = temp.path().join("game.exe");

        // e_lfanew at 0x2000 (8192 bytes, beyond initial 4KB buffer)
        let pe_large = make_pe_header(0x8664, 0x2000);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(&pe_large).unwrap();

        let arch = detect_file_architecture(&file_path).unwrap();
        assert_eq!(arch, Architecture::X64);
    }

    #[test]
    fn test_elf_architectures() {
        // x86 (EM_386 = 3, little-endian)
        let elf_x86 = make_elf_header(3, false, true);
        let info = detect_binary_info(&elf_x86);
        assert_eq!(info.format, BinaryFormat::Elf);
        assert_eq!(info.architecture, Architecture::X86);

        // x64 (EM_X86_64 = 62, little-endian)
        let elf_x64 = make_elf_header(62, true, true);
        let info = detect_binary_info(&elf_x64);
        assert_eq!(info.format, BinaryFormat::Elf);
        assert_eq!(info.architecture, Architecture::X64);

        // Arm64 (EM_AARCH64 = 183, little-endian)
        let elf_arm64 = make_elf_header(183, true, true);
        let info = detect_binary_info(&elf_arm64);
        assert_eq!(info.format, BinaryFormat::Elf);
        assert_eq!(info.architecture, Architecture::Arm64);

        // x64 big-endian
        let elf_be = make_elf_header(62, true, false);
        let info = detect_binary_info(&elf_be);
        assert_eq!(info.format, BinaryFormat::Elf);
        assert_eq!(info.architecture, Architecture::X64);

        // Unknown machine (8 = EM_MIPS)
        let elf_unk = make_elf_header(8, true, true);
        let info = detect_binary_info(&elf_unk);
        assert_eq!(info.format, BinaryFormat::Elf);
        assert_eq!(info.architecture, Architecture::Unknown);

        // Truncated ELF
        let elf_trunc = detect_binary_info(&[0x7F, b'E', b'L', b'F', 2]);
        assert_eq!(elf_trunc.format, BinaryFormat::Elf);
        assert_eq!(elf_trunc.architecture, Architecture::Unknown);
    }

    #[test]
    fn test_macho_architectures() {
        // 64-bit x64 Mach-O
        let macho_x64 = make_macho_single(0x0100_0007, true, true);
        let info = detect_binary_info(&macho_x64);
        assert_eq!(info.format, BinaryFormat::MachO);
        assert_eq!(info.architecture, Architecture::X64);

        // 64-bit Arm64 Mach-O
        let macho_arm64 = make_macho_single(0x0100_000C, true, false);
        let info = detect_binary_info(&macho_arm64);
        assert_eq!(info.format, BinaryFormat::MachO);
        assert_eq!(info.architecture, Architecture::Arm64);

        // 32-bit x86 Mach-O
        let macho_x86 = make_macho_single(7, false, true);
        let info = detect_binary_info(&macho_x86);
        assert_eq!(info.format, BinaryFormat::MachO);
        assert_eq!(info.architecture, Architecture::X86);

        // Fat binary with x64 + Arm64 -> resolves to Arm64
        let macho_fat = make_macho_fat(&[0x0100_0007, 0x0100_000C], false);
        let info = detect_binary_info(&macho_fat);
        assert_eq!(info.format, BinaryFormat::MachO);
        assert_eq!(info.architecture, Architecture::Arm64);

        // Fat binary with only x86
        let macho_fat_x86 = make_macho_fat(&[7], true);
        let info = detect_binary_info(&macho_fat_x86);
        assert_eq!(info.format, BinaryFormat::MachO);
        assert_eq!(info.architecture, Architecture::X86);
    }

    #[test]
    fn test_unknown_binary() {
        let data = b"This is just random text or plain data";
        let info = detect_binary_info(data);
        assert_eq!(info.format, BinaryFormat::Unknown);
        assert_eq!(info.architecture, Architecture::Unknown);
    }

    #[test]
    fn test_windows_mono_layout() {
        let temp = TempDir::new("win_mono");
        let root = temp.path();

        // Create Windows Mono structure:
        // Root/
        //   Game.exe
        //   MonoBleedingEdge/
        //   Game_Data/
        //     Managed/
        //       Assembly-CSharp.dll
        //       mscorlib.dll
        let exe_bytes = make_pe_header(0x8664, 0x40);
        fs::write(root.join("Game.exe"), &exe_bytes).unwrap();
        fs::create_dir_all(root.join("MonoBleedingEdge")).unwrap();
        let managed = root.join("Game_Data/Managed");
        fs::create_dir_all(&managed).unwrap();
        fs::write(managed.join("Assembly-CSharp.dll"), b"dummy dll").unwrap();
        fs::write(managed.join("mscorlib.dll"), b"dummy dll").unwrap();

        let analysis = detect_runtime(root).unwrap();
        assert_eq!(analysis.runtime_type, RuntimeType::Mono);
        assert_eq!(analysis.confidence, Confidence::Confirmed);
        assert!(analysis
            .evidences
            .iter()
            .any(|e| e.key == "mono_bleeding_edge_dir"));
        assert!(analysis
            .evidences
            .iter()
            .any(|e| e.key == "assembly_csharp"));

        // Game architecture detection
        let arch = detect_game_architecture(root).unwrap();
        assert_eq!(arch, Architecture::X64);
    }

    #[test]
    fn test_windows_il2cpp_layout() {
        let temp = TempDir::new("win_il2cpp");
        let root = temp.path();

        // Create Windows IL2CPP structure:
        // Root/
        //   Game.exe
        //   GameAssembly.dll
        //   Game_Data/
        //     il2cpp_data/
        //       Metadata/
        //         global-metadata.dat
        let exe_bytes = make_pe_header(0x8664, 0x40);
        fs::write(root.join("Game.exe"), &exe_bytes).unwrap();
        fs::write(root.join("GameAssembly.dll"), b"dummy dll").unwrap();

        let metadata_dir = root.join("Game_Data/il2cpp_data/Metadata");
        fs::create_dir_all(&metadata_dir).unwrap();
        fs::write(metadata_dir.join("global-metadata.dat"), b"metadata").unwrap();

        let analysis = detect_runtime(root).unwrap();
        assert_eq!(analysis.runtime_type, RuntimeType::Il2Cpp);
        assert_eq!(analysis.confidence, Confidence::Confirmed);
        assert!(analysis.evidences.iter().any(|e| e.key == "game_assembly"));
        assert!(analysis
            .evidences
            .iter()
            .any(|e| e.key == "il2cpp_metadata"));

        let arch = detect_game_architecture(root).unwrap();
        assert_eq!(arch, Architecture::X64);
    }

    #[test]
    fn test_linux_il2cpp_layout() {
        let temp = TempDir::new("linux_il2cpp");
        let root = temp.path();

        let elf_bytes = make_elf_header(62, true, true);
        fs::write(root.join("Game.x86_64"), &elf_bytes).unwrap();
        fs::write(root.join("GameAssembly.so"), b"elf so").unwrap();

        let meta = root.join("Game_Data/il2cpp_data/Metadata");
        fs::create_dir_all(&meta).unwrap();
        fs::write(meta.join("global-metadata.dat"), b"meta").unwrap();

        let analysis = detect_runtime(root).unwrap();
        assert_eq!(analysis.runtime_type, RuntimeType::Il2Cpp);
        assert_eq!(analysis.confidence, Confidence::Confirmed);

        let arch = detect_game_architecture(root).unwrap();
        assert_eq!(arch, Architecture::X64);
    }

    #[test]
    fn test_linux_mono_layout() {
        let temp = TempDir::new("linux_mono");
        let root = temp.path();

        let elf_bytes = make_elf_header(183, true, true); // Arm64
        fs::write(root.join("Game.x86_64"), &elf_bytes).unwrap();

        let data = root.join("Game_Data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("libmonobdwgc-2.0.so"), b"so").unwrap();
        fs::create_dir_all(data.join("Managed")).unwrap();
        fs::write(data.join("Managed/Assembly-CSharp.dll"), b"dll").unwrap();

        let analysis = detect_runtime(root).unwrap();
        assert_eq!(analysis.runtime_type, RuntimeType::Mono);
        assert_eq!(analysis.confidence, Confidence::Confirmed);

        let arch = detect_game_architecture(root).unwrap();
        assert_eq!(arch, Architecture::Arm64);
    }

    #[test]
    fn test_macos_bundle_layout() {
        let temp = TempDir::new("macos_app");
        let root = temp.path().join("Game.app");

        // IL2CPP macOS layout:
        // Game.app/Contents/Frameworks/GameAssembly.dylib
        // Game.app/Contents/Resources/Data/il2cpp_data/Metadata/global-metadata.dat
        let frameworks = root.join("Contents/Frameworks");
        fs::create_dir_all(&frameworks).unwrap();
        fs::write(frameworks.join("GameAssembly.dylib"), b"dylib").unwrap();

        let data = root.join("Contents/Resources/Data/il2cpp_data/Metadata");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("global-metadata.dat"), b"meta").unwrap();

        let analysis = detect_runtime(&root).unwrap();
        assert_eq!(analysis.runtime_type, RuntimeType::Il2Cpp);
        assert_eq!(analysis.confidence, Confidence::Confirmed);
    }

    #[test]
    fn test_conflict_layout() {
        let temp = TempDir::new("conflict");
        let root = temp.path();

        // Both GameAssembly.dll AND mono-2.0-bdwgc.dll present
        fs::write(root.join("GameAssembly.dll"), b"assembly").unwrap();
        fs::write(root.join("mono-2.0-bdwgc.dll"), b"mono").unwrap();

        let analysis = detect_runtime(root).unwrap();
        assert_eq!(analysis.runtime_type, RuntimeType::Unknown);
        assert_eq!(analysis.confidence, Confidence::Conflict);
    }

    #[test]
    fn test_empty_or_nonexistent_directory() {
        let temp = TempDir::new("empty");
        let analysis = detect_runtime(temp.path()).unwrap();
        assert_eq!(analysis.runtime_type, RuntimeType::Unknown);
        assert_eq!(analysis.confidence, Confidence::Unknown);
        assert!(analysis.evidences.is_empty());

        let non_existent = temp.path().join("does_not_exist");
        let analysis_non_existent = detect_runtime(&non_existent).unwrap();
        assert_eq!(analysis_non_existent.runtime_type, RuntimeType::Unknown);
        assert_eq!(analysis_non_existent.confidence, Confidence::Unknown);
    }

    #[test]
    fn test_detect_runtime_from_paths() {
        // Test in-memory path detection for IL2CPP
        let paths = vec![
            "Game.exe",
            "GameAssembly.dll",
            "Game_Data/il2cpp_data/Metadata/global-metadata.dat",
        ];
        let analysis = detect_runtime_from_paths(&paths);
        assert_eq!(analysis.runtime_type, RuntimeType::Il2Cpp);
        assert_eq!(analysis.confidence, Confidence::Confirmed);

        // Test in-memory path detection for Mono
        let mono_paths = vec![
            "Game.exe",
            "MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll",
            "Game_Data/Managed/Assembly-CSharp.dll",
        ];
        let analysis_mono = detect_runtime_from_paths(&mono_paths);
        assert_eq!(analysis_mono.runtime_type, RuntimeType::Mono);
        assert_eq!(analysis_mono.confidence, Confidence::Confirmed);
    }
}
