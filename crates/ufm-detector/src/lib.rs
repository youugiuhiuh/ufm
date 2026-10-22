//! `ufm-detector` crate
//!
//! Responsible for Unity game detection:
//! - Discovering Unity game data directories and executables
//! - Extracting Unity version numbers from asset files and binaries
//! - Detecting modding frameworks and hook loaders (BepInEx, Doorstop, MelonLoader)
//! - Gathering multi-faceted evidence and computing overall confidence

pub mod binary;
pub mod detector;
pub mod frameworks;
pub mod version;

pub use binary::{inspect_binary_file, inspect_binary_format};
pub use detector::{calculate_confidence, detect, DetectorError};
pub use frameworks::{detect_frameworks, FrameworkDetection};
pub use version::{
    extract_unity_version_from_bytes, extract_unity_version_from_file, parse_unity_version,
};

/// Health / readiness check for ufm-detector crate
pub fn is_ready() -> bool {
    ufm_runtime::is_ready()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ready() {
        assert!(is_ready());
    }
}
