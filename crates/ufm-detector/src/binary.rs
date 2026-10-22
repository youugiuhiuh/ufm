use std::fs::File;
use std::io::Read;
use std::path::Path;
use ufm_core::{Architecture, TargetPlatform};

/// Inspects binary headers (PE / ELF / Mach-O) to determine architecture and platform.
pub fn inspect_binary_format(bytes: &[u8]) -> (Architecture, TargetPlatform) {
    if bytes.len() >= 2 && &bytes[0..2] == b"MZ" {
        // Windows PE
        let platform = TargetPlatform::Windows;
        if bytes.len() >= 0x40 {
            let e_lfanew =
                u32::from_le_bytes([bytes[0x3C], bytes[0x3D], bytes[0x3E], bytes[0x3F]]) as usize;
            if e_lfanew + 6 <= bytes.len() && &bytes[e_lfanew..e_lfanew + 4] == b"PE\0\0" {
                let machine = u16::from_le_bytes([bytes[e_lfanew + 4], bytes[e_lfanew + 5]]);
                let arch = match machine {
                    0x014c => Architecture::X86,
                    0x8664 => Architecture::X64,
                    0xaa64 => Architecture::Arm64,
                    _ => Architecture::Unknown,
                };
                return (arch, platform);
            }
        }
        return (Architecture::Unknown, platform);
    }

    if bytes.len() >= 4 && &bytes[0..4] == b"\x7FELF" {
        // Linux ELF
        let platform = TargetPlatform::Linux;
        if bytes.len() >= 0x14 {
            let machine = u16::from_le_bytes([bytes[0x12], bytes[0x13]]);
            let arch = match machine {
                0x03 => Architecture::X86,
                0x3E => Architecture::X64,
                0xB7 => Architecture::Arm64,
                _ => match bytes[4] {
                    1 => Architecture::X86,
                    2 => Architecture::X64,
                    _ => Architecture::Unknown,
                },
            };
            return (arch, platform);
        }
        return (Architecture::Unknown, platform);
    }

    if bytes.len() >= 4 {
        // Mach-O
        if &bytes[0..4] == b"\xFE\xED\xFA\xCE"
            || &bytes[0..4] == b"\xCF\xFA\xED\xFE"
            || &bytes[0..4] == b"\xCA\xFE\xBA\xBE"
        {
            return (Architecture::Unknown, TargetPlatform::MacOs);
        }
    }

    (Architecture::Unknown, TargetPlatform::Unknown)
}

/// Reads the beginning of a binary file and detects its architecture and target platform.
pub fn inspect_binary_file<P: AsRef<Path>>(path: P) -> (Architecture, TargetPlatform) {
    if let Ok(mut file) = File::open(path) {
        let mut buffer = [0u8; 1024];
        if let Ok(n) = file.read(&mut buffer) {
            return inspect_binary_format(&buffer[..n]);
        }
    }
    (Architecture::Unknown, TargetPlatform::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pe_x64_detection() {
        let mut pe_data = vec![0u8; 0x100];
        pe_data[0..2].copy_from_slice(b"MZ");
        // e_lfanew at 0x3C -> 0x80
        pe_data[0x3C] = 0x80;
        // Signature "PE\0\0" at 0x80
        pe_data[0x80..0x84].copy_from_slice(b"PE\0\0");
        // Machine AMD64 (0x8664) at 0x84
        pe_data[0x84] = 0x64;
        pe_data[0x85] = 0x86;

        let (arch, plat) = inspect_binary_format(&pe_data);
        assert_eq!(arch, Architecture::X64);
        assert_eq!(plat, TargetPlatform::Windows);
    }

    #[test]
    fn test_pe_x86_detection() {
        let mut pe_data = vec![0u8; 0x100];
        pe_data[0..2].copy_from_slice(b"MZ");
        pe_data[0x3C] = 0x80;
        pe_data[0x80..0x84].copy_from_slice(b"PE\0\0");
        // Machine I386 (0x014c)
        pe_data[0x84] = 0x4c;
        pe_data[0x85] = 0x01;

        let (arch, plat) = inspect_binary_format(&pe_data);
        assert_eq!(arch, Architecture::X86);
        assert_eq!(plat, TargetPlatform::Windows);
    }

    #[test]
    fn test_elf_x64_detection() {
        let mut elf_data = vec![0u8; 0x20];
        elf_data[0..4].copy_from_slice(b"\x7FELF");
        elf_data[4] = 2; // 64-bit
        elf_data[0x12] = 0x3E; // x86_64
        elf_data[0x13] = 0x00;

        let (arch, plat) = inspect_binary_format(&elf_data);
        assert_eq!(arch, Architecture::X64);
        assert_eq!(plat, TargetPlatform::Linux);
    }
}
