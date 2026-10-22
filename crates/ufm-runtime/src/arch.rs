//! Binary architecture and format inspection (PE, ELF, Mach-O).

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use ufm_core::Architecture;

/// Binary executable format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BinaryFormat {
    Pe,
    Elf,
    MachO,
    Unknown,
}

/// Inspection summary containing format and target architecture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryInfo {
    pub format: BinaryFormat,
    pub architecture: Architecture,
}

// Mach-O Magic constants
const MH_MAGIC: [u8; 4] = [0xFE, 0xED, 0xFA, 0xCE];
const MH_CIGAM: [u8; 4] = [0xCE, 0xFA, 0xED, 0xFE];
const MH_MAGIC_64: [u8; 4] = [0xFE, 0xED, 0xFA, 0xCF];
const MH_CIGAM_64: [u8; 4] = [0xCF, 0xFA, 0xED, 0xFE];
const FAT_MAGIC: [u8; 4] = [0xCA, 0xFE, 0xBA, 0xBE];
const FAT_CIGAM: [u8; 4] = [0xBE, 0xBA, 0xFE, 0xCA];
const FAT_MAGIC_64: [u8; 4] = [0xCA, 0xFE, 0xBA, 0xBF];
const FAT_CIGAM_64: [u8; 4] = [0xBF, 0xBA, 0xFE, 0xCA];

// Mach-O CPU Types
const CPU_TYPE_X86: u32 = 7;
const CPU_TYPE_X86_64: u32 = 0x0100_0007;
const CPU_TYPE_ARM64: u32 = 0x0100_000C;

/// Parse binary header bytes to determine format and architecture.
pub fn detect_binary_info(bytes: &[u8]) -> BinaryInfo {
    if bytes.len() >= 2 && &bytes[0..2] == b"MZ" {
        return parse_pe(bytes);
    }

    if bytes.len() >= 4 && bytes[0..4] == [0x7F, b'E', b'L', b'F'] {
        return parse_elf(bytes);
    }

    if bytes.len() >= 4 {
        if let Some(info) = parse_macho(bytes) {
            return info;
        }
    }

    BinaryInfo {
        format: BinaryFormat::Unknown,
        architecture: Architecture::Unknown,
    }
}

/// Detect architecture directly from in-memory binary slice.
pub fn detect_architecture_from_bytes(bytes: &[u8]) -> Architecture {
    detect_binary_info(bytes).architecture
}

/// Detect architecture of an executable file on disk with minimal I/O.
pub fn detect_file_architecture<P: AsRef<Path>>(path: P) -> std::io::Result<Architecture> {
    let info = detect_file_binary_info(path)?;
    Ok(info.architecture)
}

/// Inspect an executable file on disk, reading only necessary header bytes.
pub fn detect_file_binary_info<P: AsRef<Path>>(path: P) -> std::io::Result<BinaryInfo> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 4096];
    let n = file.read(&mut buffer)?;
    let slice = &buffer[..n];

    // Check for PE with e_lfanew extending beyond the initial buffer
    if slice.len() >= 0x40 && &slice[0..2] == b"MZ" {
        let e_lfanew = u32::from_le_bytes(slice[0x3C..0x40].try_into().unwrap()) as usize;
        if e_lfanew + 24 > n {
            file.seek(SeekFrom::Start(e_lfanew as u64))?;
            let mut pe_header = [0u8; 24];
            file.read_exact(&mut pe_header)?;
            if &pe_header[0..4] == b"PE\0\0" {
                let machine = u16::from_le_bytes([pe_header[4], pe_header[5]]);
                return Ok(BinaryInfo {
                    format: BinaryFormat::Pe,
                    architecture: map_pe_machine(machine),
                });
            }
        }
    }

    Ok(detect_binary_info(slice))
}

fn parse_pe(bytes: &[u8]) -> BinaryInfo {
    if bytes.len() < 0x40 {
        return BinaryInfo {
            format: BinaryFormat::Pe,
            architecture: Architecture::Unknown,
        };
    }

    let e_lfanew = u32::from_le_bytes(bytes[0x3C..0x40].try_into().unwrap()) as usize;
    if e_lfanew + 6 > bytes.len() {
        return BinaryInfo {
            format: BinaryFormat::Pe,
            architecture: Architecture::Unknown,
        };
    }

    if &bytes[e_lfanew..e_lfanew + 4] != b"PE\0\0" {
        return BinaryInfo {
            format: BinaryFormat::Pe,
            architecture: Architecture::Unknown,
        };
    }

    let machine = u16::from_le_bytes(bytes[e_lfanew + 4..e_lfanew + 6].try_into().unwrap());
    BinaryInfo {
        format: BinaryFormat::Pe,
        architecture: map_pe_machine(machine),
    }
}

fn map_pe_machine(machine: u16) -> Architecture {
    match machine {
        0x014C => Architecture::X86,
        0x8664 => Architecture::X64,
        0xAA64 => Architecture::Arm64,
        _ => Architecture::Unknown,
    }
}

fn parse_elf(bytes: &[u8]) -> BinaryInfo {
    if bytes.len() < 0x14 {
        return BinaryInfo {
            format: BinaryFormat::Elf,
            architecture: Architecture::Unknown,
        };
    }

    let endian = bytes[5];
    let machine = match endian {
        1 => u16::from_le_bytes([bytes[0x12], bytes[0x13]]),
        2 => u16::from_be_bytes([bytes[0x12], bytes[0x13]]),
        _ => {
            return BinaryInfo {
                format: BinaryFormat::Elf,
                architecture: Architecture::Unknown,
            }
        }
    };

    let architecture = match machine {
        0x0003 => Architecture::X86,   // EM_386
        0x003E => Architecture::X64,   // EM_X86_64
        0x00B7 => Architecture::Arm64, // EM_AARCH64
        _ => Architecture::Unknown,
    };

    BinaryInfo {
        format: BinaryFormat::Elf,
        architecture,
    }
}

fn parse_macho(bytes: &[u8]) -> Option<BinaryInfo> {
    let magic = &bytes[0..4];

    // Single-architecture Mach-O
    let (is_single, is_be) = if magic == MH_MAGIC || magic == MH_MAGIC_64 {
        (true, true)
    } else if magic == MH_CIGAM || magic == MH_CIGAM_64 {
        (true, false)
    } else {
        (false, false)
    };

    if is_single {
        if bytes.len() < 8 {
            return Some(BinaryInfo {
                format: BinaryFormat::MachO,
                architecture: Architecture::Unknown,
            });
        }
        let cputype = if is_be {
            u32::from_be_bytes(bytes[4..8].try_into().unwrap())
        } else {
            u32::from_le_bytes(bytes[4..8].try_into().unwrap())
        };

        return Some(BinaryInfo {
            format: BinaryFormat::MachO,
            architecture: map_macho_cputype(cputype),
        });
    }

    // Fat Universal Binary
    let (is_fat, is_fat_be, is_fat_64) = if magic == FAT_MAGIC {
        (true, true, false)
    } else if magic == FAT_CIGAM {
        (true, false, false)
    } else if magic == FAT_MAGIC_64 {
        (true, true, true)
    } else if magic == FAT_CIGAM_64 {
        (true, false, true)
    } else {
        (false, false, false)
    };

    if is_fat {
        if bytes.len() < 8 {
            return Some(BinaryInfo {
                format: BinaryFormat::MachO,
                architecture: Architecture::Unknown,
            });
        }
        let nfat = if is_fat_be {
            u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize
        } else {
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize
        };

        let entry_size = if is_fat_64 { 32 } else { 20 };
        let mut arches = Vec::new();

        for i in 0..nfat.min(32) {
            let offset = 8 + i * entry_size;
            if offset + 4 <= bytes.len() {
                let cputype = if is_fat_be {
                    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
                } else {
                    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
                };
                let arch = map_macho_cputype(cputype);
                if arch != Architecture::Unknown {
                    arches.push(arch);
                }
            }
        }

        // Priority preference for universal binaries: Arm64 > X64 > X86
        let architecture = if arches.contains(&Architecture::Arm64) {
            Architecture::Arm64
        } else if arches.contains(&Architecture::X64) {
            Architecture::X64
        } else if arches.contains(&Architecture::X86) {
            Architecture::X86
        } else {
            Architecture::Unknown
        };

        return Some(BinaryInfo {
            format: BinaryFormat::MachO,
            architecture,
        });
    }

    None
}

fn map_macho_cputype(cputype: u32) -> Architecture {
    match cputype {
        CPU_TYPE_X86 => Architecture::X86,
        CPU_TYPE_X86_64 => Architecture::X64,
        CPU_TYPE_ARM64 => Architecture::Arm64,
        _ => Architecture::Unknown,
    }
}
