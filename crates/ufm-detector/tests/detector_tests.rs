use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use ufm_core::{Architecture, Confidence, RuntimeType, TargetPlatform};
use ufm_detector::{detect, DetectorError};

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempGameDir {
    path: PathBuf,
}

impl TempGameDir {
    fn new(prefix: &str) -> Self {
        let count = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "ufm_detector_test_{}_{}_{}",
            prefix,
            std::process::id(),
            count
        ));
        fs::create_dir_all(&path).expect("Failed to create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempGameDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

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

fn write_asset_file_with_version(path: &Path, version: &str) {
    let mut data = vec![0u8; 128];
    // Put version string at offset 20
    let ver_bytes = version.as_bytes();
    data[20..20 + ver_bytes.len()].copy_from_slice(ver_bytes);
    data[20 + ver_bytes.len()] = 0; // null terminator
    fs::write(path, &data).unwrap();
}

#[test]
fn test_detect_unity_mono_windows() {
    let temp = TempGameDir::new("mono_win");
    let root = temp.path();

    // 1. Root files
    let exe_bytes = make_pe_header(0x8664, 0x40); // x64 PE
    fs::write(root.join("MyGame.exe"), &exe_bytes).unwrap();
    fs::write(root.join("UnityPlayer.dll"), &exe_bytes).unwrap();
    fs::write(root.join("UnityCrashHandler64.exe"), b"stub").unwrap();

    // 2. Data directory: MyGame_Data
    let data_dir = root.join("MyGame_Data");
    fs::create_dir_all(&data_dir).unwrap();
    write_asset_file_with_version(&data_dir.join("globalgamemanagers"), "2021.3.16f1");

    // Managed folder with dummy assemblies
    let managed_dir = data_dir.join("Managed");
    fs::create_dir_all(&managed_dir).unwrap();
    fs::write(managed_dir.join("Assembly-CSharp.dll"), b"assembly").unwrap();
    fs::write(managed_dir.join("mscorlib.dll"), b"mscorlib").unwrap();

    // Test detection from game root directory
    let report = detect(root).expect("Detection should succeed");
    assert_eq!(report.overall_confidence, Confidence::Confirmed);
    assert_eq!(report.runtime_type, RuntimeType::Mono);
    assert_eq!(report.architecture, Architecture::X64);
    assert_eq!(report.target_platform, TargetPlatform::Windows);
    assert!(report.data_dir.is_some());
    assert!(report.data_dir.unwrap().ends_with("MyGame_Data"));

    let ver = report.unity_version.expect("Unity version should be found");
    assert_eq!(ver.raw, "2021.3.16f1");
    assert_eq!(ver.major, 2021);
    assert_eq!(ver.minor, 3);
    assert_eq!(ver.patch, 16);
    assert_eq!(ver.suffix.as_deref(), Some("f1"));

    // Test detection by pointing directly to the executable
    let exe_path = root.join("MyGame.exe");
    let report_exe = detect(&exe_path).expect("Detection from exe path should succeed");
    assert_eq!(report_exe.overall_confidence, Confidence::Confirmed);
    assert_eq!(report_exe.runtime_type, RuntimeType::Mono);
}

#[test]
fn test_detect_unity_il2cpp_windows() {
    let temp = TempGameDir::new("il2cpp_win");
    let root = temp.path();

    // Root files
    let exe_bytes = make_pe_header(0x8664, 0x40);
    fs::write(root.join("ActionGame.exe"), &exe_bytes).unwrap();
    fs::write(root.join("GameAssembly.dll"), b"il2cpp assembly").unwrap();
    fs::write(root.join("UnityPlayer.dll"), &exe_bytes).unwrap();

    // Data directory
    let data_dir = root.join("ActionGame_Data");
    fs::create_dir_all(&data_dir).unwrap();
    write_asset_file_with_version(&data_dir.join("globalgamemanagers"), "2019.4.40f1");

    let metadata_dir = data_dir.join("il2cpp_data").join("Metadata");
    fs::create_dir_all(&metadata_dir).unwrap();
    fs::write(metadata_dir.join("global-metadata.dat"), b"il2cpp meta").unwrap();

    let report = detect(root).expect("Detection should succeed");
    assert_eq!(report.overall_confidence, Confidence::Confirmed);
    assert_eq!(report.runtime_type, RuntimeType::Il2Cpp);
    assert_eq!(report.architecture, Architecture::X64);

    let ver = report.unity_version.expect("Unity version should be found");
    assert_eq!(ver.raw, "2019.4.40f1");
    assert_eq!(ver.major, 2019);
    assert_eq!(ver.minor, 4);
    assert_eq!(ver.patch, 40);
    assert_eq!(ver.suffix.as_deref(), Some("f1"));
}

#[test]
fn test_detect_bepinex_and_doorstop() {
    let temp = TempGameDir::new("bepinex_doorstop");
    let root = temp.path();

    // Root setup
    let exe_bytes = make_pe_header(0x8664, 0x40);
    fs::write(root.join("Game.exe"), &exe_bytes).unwrap();
    fs::write(root.join("UnityPlayer.dll"), &exe_bytes).unwrap();

    // Doorstop files
    fs::write(
        root.join("doorstop_config.ini"),
        "[General]\nenabled=true\ntarget_assembly=BepInEx/core/BepInEx.Preloader.dll\n",
    )
    .unwrap();
    fs::write(root.join("winhttp.dll"), b"proxy dll").unwrap();

    // BepInEx structure
    let bep_core = root.join("BepInEx").join("core");
    fs::create_dir_all(&bep_core).unwrap();
    fs::write(bep_core.join("BepInEx.Core.dll"), b"bepinex core").unwrap();
    fs::write(bep_core.join("BepInEx.Preloader.dll"), b"preloader").unwrap();

    // Data directory
    let data_dir = root.join("Game_Data");
    fs::create_dir_all(&data_dir).unwrap();
    write_asset_file_with_version(&data_dir.join("globalgamemanagers"), "2020.3.48f1");

    let report = detect(root).expect("Detection should succeed");
    assert!(report.installed_frameworks.contains(&"BepInEx".to_string()));
    assert!(report
        .installed_frameworks
        .contains(&"Doorstop".to_string()));
    assert!(report
        .evidences
        .iter()
        .any(|e| e.key == "framework.bepinex"));
    assert!(report
        .evidences
        .iter()
        .any(|e| e.key == "framework.doorstop"));
}

#[test]
fn test_detect_melonloader() {
    let temp = TempGameDir::new("melonloader");
    let root = temp.path();

    let exe_bytes = make_pe_header(0x8664, 0x40);
    fs::write(root.join("Game.exe"), &exe_bytes).unwrap();
    fs::write(root.join("UnityPlayer.dll"), &exe_bytes).unwrap();
    fs::write(root.join("version.dll"), b"proxy dll").unwrap();

    // MelonLoader folder
    let melon_dir = root.join("MelonLoader");
    fs::create_dir_all(&melon_dir).unwrap();
    fs::write(melon_dir.join("MelonLoader.dll"), b"melonloader dll").unwrap();

    let data_dir = root.join("Game_Data");
    fs::create_dir_all(&data_dir).unwrap();
    write_asset_file_with_version(&data_dir.join("globalgamemanagers"), "2021.1.0b8");

    let report = detect(root).expect("Detection should succeed");
    assert!(report
        .installed_frameworks
        .contains(&"MelonLoader".to_string()));
    assert!(report
        .evidences
        .iter()
        .any(|e| e.key == "framework.melonloader"));

    let ver = report.unity_version.unwrap();
    assert_eq!(ver.raw, "2021.1.0b8");
    assert_eq!(ver.suffix.as_deref(), Some("b8"));
}

#[test]
fn test_detect_version_from_unity_player() {
    let temp = TempGameDir::new("ver_from_player");
    let root = temp.path();

    // Make a fake UnityPlayer.dll containing embedded version string
    let mut player_bytes = make_pe_header(0x8664, 0x40);
    player_bytes.extend_from_slice(&[0u8; 128]);
    let ver_tag = b"UnityPlayer/2022.3.10f1 (build)\0";
    let insert_pos = player_bytes.len() - ver_tag.len();
    player_bytes[insert_pos..].copy_from_slice(ver_tag);
    fs::write(root.join("UnityPlayer.dll"), &player_bytes).unwrap();

    let data_dir = root.join("Game_Data");
    fs::create_dir_all(&data_dir).unwrap();
    // No globalgamemanagers here, only player dll has version

    let report = detect(root).expect("Detection should succeed");
    let ver = report
        .unity_version
        .expect("Should extract from UnityPlayer.dll");
    assert_eq!(ver.raw, "2022.3.10f1");
    assert_eq!(ver.major, 2022);
    assert_eq!(ver.minor, 3);
    assert_eq!(ver.patch, 10);
    assert_eq!(ver.suffix.as_deref(), Some("f1"));
}

#[test]
fn test_detect_version_from_data_unity3d() {
    let temp = TempGameDir::new("data_unity3d");
    let root = temp.path();

    let exe_bytes = make_pe_header(0x8664, 0x40);
    fs::write(root.join("WebGame.exe"), &exe_bytes).unwrap();
    fs::write(root.join("UnityPlayer.dll"), &exe_bytes).unwrap();

    let data_dir = root.join("WebGame_Data");
    fs::create_dir_all(&data_dir).unwrap();
    write_asset_file_with_version(&data_dir.join("data.unity3d"), "5.6.7f1");

    let report = detect(root).expect("Detection should succeed");
    let ver = report
        .unity_version
        .expect("Should extract from data.unity3d");
    assert_eq!(ver.raw, "5.6.7f1");
    assert_eq!(ver.major, 5);
    assert_eq!(ver.minor, 6);
    assert_eq!(ver.patch, 7);
}

#[test]
fn test_detect_ambiguous_data_dirs_conflict() {
    let temp = TempGameDir::new("conflict_dirs");
    let root = temp.path();

    // Create two conflicting data dirs without any matching executable
    fs::create_dir_all(root.join("Alpha_Data")).unwrap();
    fs::create_dir_all(root.join("Beta_Data")).unwrap();

    let report = detect(root).expect("Detection should complete");
    assert_eq!(report.overall_confidence, Confidence::Conflict);
}

#[test]
fn test_detect_unknown_empty_directory() {
    let temp = TempGameDir::new("unknown_empty");
    let root = temp.path();

    // Non-Unity files
    fs::write(root.join("readme.txt"), b"Just a text file").unwrap();

    let report = detect(root).expect("Detection should succeed");
    assert_eq!(report.overall_confidence, Confidence::Unknown);
    assert_eq!(report.runtime_type, RuntimeType::Unknown);
    assert_eq!(report.installed_frameworks.len(), 0);
}

#[test]
fn test_detect_nonexistent_path() {
    let temp = TempGameDir::new("non_existent");
    let non_existent = temp.path().join("does_not_exist");

    let err = detect(&non_existent).unwrap_err();
    match err {
        DetectorError::PathNotFound(p) => assert_eq!(p, non_existent),
        other => panic!("Expected PathNotFound, got: {:?}", other),
    }
}
