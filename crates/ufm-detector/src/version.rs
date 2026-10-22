use std::path::Path;
use ufm_core::UnityVersionInfo;

/// Parses a Unity version string (e.g. "2021.3.16f1", "5.6.7f1", "2020.1.0b5").
pub fn parse_unity_version(text: &str) -> Option<UnityVersionInfo> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    let parts: Vec<&str> = text.splitn(3, '.').collect();
    if parts.len() != 3 {
        return None;
    }

    let major: u32 = parts[0].parse().ok()?;
    let minor: u32 = parts[1].parse().ok()?;

    if !is_valid_unity_major(major) {
        return None;
    }

    let part2 = parts[2];
    let patch_digit_len = part2.chars().take_while(|c| c.is_ascii_digit()).count();
    if patch_digit_len == 0 {
        return None;
    }

    let patch: u32 = part2[..patch_digit_len].parse().ok()?;
    let suffix_str = &part2[patch_digit_len..];
    let suffix = if suffix_str.is_empty() {
        None
    } else {
        let first_char = suffix_str.chars().next()?;
        if !first_char.is_ascii_alphabetic() {
            return None;
        }
        Some(suffix_str.to_string())
    };

    Some(UnityVersionInfo {
        raw: text.to_string(),
        major,
        minor,
        patch,
        suffix,
    })
}

fn is_valid_unity_major(major: u32) -> bool {
    (3..=6).contains(&major) || (2017..=2035).contains(&major) || (6000..=6099).contains(&major)
}

/// Scans binary data for Unity version strings.
pub fn extract_unity_version_from_bytes(bytes: &[u8]) -> Option<UnityVersionInfo> {
    let len = bytes.len();
    if len < 5 {
        return None;
    }

    let mut best_candidate: Option<UnityVersionInfo> = None;
    let mut i = 0;

    while i < len {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }

        // Boundary check: previous byte cannot be alphanumeric, '.', or '_'
        if i > 0 {
            let prev = bytes[i - 1];
            if prev.is_ascii_alphanumeric() || prev == b'.' || prev == b'_' {
                i += 1;
                continue;
            }
        }

        // Scan potential version string (up to 32 bytes)
        let start = i;
        let mut end = i;
        while end < len && (end - start) < 32 {
            let b = bytes[end];
            if b.is_ascii_alphanumeric() || b == b'.' {
                end += 1;
            } else {
                break;
            }
        }

        let slice = &bytes[start..end];
        if let Ok(s) = std::str::from_utf8(slice) {
            if let Some(info) = parse_unity_version(s) {
                // Suffixes starting with 'f', 'b', 'a', 'p', 'rc' are characteristic of Unity builds
                if let Some(ref suf) = info.suffix {
                    let first = suf.chars().next().unwrap_or(' ');
                    if matches!(first, 'f' | 'b' | 'a' | 'p' | 'r') {
                        return Some(info);
                    }
                }
                if best_candidate.is_none() {
                    best_candidate = Some(info);
                }
            }
        }

        i = end + 1;
    }

    best_candidate
}

/// Reads the beginning of a file (up to 4MB) and extracts the Unity version if present.
pub fn extract_unity_version_from_file<P: AsRef<Path>>(path: P) -> Option<UnityVersionInfo> {
    use std::fs::File;
    use std::io::Read;

    let file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    let mut take = file.take(4 * 1024 * 1024);
    take.read_to_end(&mut buffer).ok()?;
    extract_unity_version_from_bytes(&buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unity_version_valid() {
        let v = parse_unity_version("2021.3.16f1").unwrap();
        assert_eq!(v.raw, "2021.3.16f1");
        assert_eq!(v.major, 2021);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 16);
        assert_eq!(v.suffix.as_deref(), Some("f1"));

        let v2 = parse_unity_version("5.6.7f1").unwrap();
        assert_eq!(v2.major, 5);
        assert_eq!(v2.minor, 6);
        assert_eq!(v2.patch, 7);
        assert_eq!(v2.suffix.as_deref(), Some("f1"));

        let v3 = parse_unity_version("2020.1.0b5").unwrap();
        assert_eq!(v3.major, 2020);
        assert_eq!(v3.minor, 1);
        assert_eq!(v3.patch, 0);
        assert_eq!(v3.suffix.as_deref(), Some("b5"));

        let v4 = parse_unity_version("6000.0.16f1").unwrap();
        assert_eq!(v4.major, 6000);
        assert_eq!(v4.minor, 0);
        assert_eq!(v4.patch, 16);
        assert_eq!(v4.suffix.as_deref(), Some("f1"));

        let v5 = parse_unity_version("2019.4.40").unwrap();
        assert_eq!(v5.major, 2019);
        assert_eq!(v5.minor, 4);
        assert_eq!(v5.patch, 40);
        assert_eq!(v5.suffix, None);
    }

    #[test]
    fn test_parse_unity_version_invalid() {
        assert!(parse_unity_version("").is_none());
        assert!(parse_unity_version("10.0.19041").is_none());
        assert!(parse_unity_version("not_a_version").is_none());
        assert!(parse_unity_version("1.2").is_none());
    }

    #[test]
    fn test_extract_unity_version_from_bytes() {
        let mut data = vec![0u8; 64];
        let ver_str = b"2021.3.16f1\0";
        data[20..20 + ver_str.len()].copy_from_slice(ver_str);

        let extracted = extract_unity_version_from_bytes(&data).unwrap();
        assert_eq!(extracted.raw, "2021.3.16f1");
        assert_eq!(extracted.major, 2021);
        assert_eq!(extracted.minor, 3);
        assert_eq!(extracted.patch, 16);
        assert_eq!(extracted.suffix.as_deref(), Some("f1"));
    }
}
