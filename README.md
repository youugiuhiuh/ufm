# Unity Framework Manager (UFM)

<p align="center">
  <strong>Open Source · Cross-Platform · Evidence-Driven Unity Game Environment Detection & Mod Framework Manager</strong>
</p>

<p align="center">
  <a href="https://github.com/youugiuhiuh/ufm/actions"><img src="https://github.com/youugiuhiuh/ufm/actions/workflows/public-release.yml/badge.svg" alt="Release Build" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg" alt="License" /></a>
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Tauri-v2-24C8DB.svg" alt="Tauri 2" /></a>
  <a href="https://react.dev/"><img src="https://img.shields.io/badge/React-18%2B-61DAFB.svg" alt="React" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.80%2B-orange.svg" alt="Rust" /></a>
</p>

---

## 📖 Overview

**Unity Framework Manager (UFM)** is an open-source, cross-platform desktop application designed for Unity gamers and mod developers. By inspecting static evidence chains across executable binaries, engine assets, and runtime directories, UFM accurately analyzes the game runtime environment (Unity engine version, Mono / IL2CPP runtime, PE/ELF/Mach-O architecture, existing loaders), recommends optimal framework builds, and performs atomic installations with automatic collision backups and one-click lossless rollbacks.

### Key Features

- 🎯 **Evidence-Driven Detection**: Rejects single-file guesswork. Analyzes executable headers, asset metadata (`globalgamemanagers`), and runtime markers (`MonoBleedingEdge` / `Managed` / `il2cpp_data`), generating explainable reports with explicit confidence tiers (*Confirmed*, *HighProbability*, *Conflict*, *Unknown*).
- ⚡ **Latest Bleeding Edge & BepInEx 5 Compatibility**:
  - Prioritizes the latest builds directly from [builds.bepinex.dev (bepinex_be)](https://builds.bepinex.dev/projects/bepinex_be) by default, providing modern fixes for newer Unity releases.
  - Dynamically parses and updates online nightlies while maintaining built-in offline catalogs.
  - Built-in stable fallback for **BepInEx 5.4** for legacy Mono games and plugins incompatible with BepInEx 6.
- 🛡️ **Safe Atomic Installation & Rollback**:
  - Generates an actionable plan preview detailing all file modifications and additions before writing.
  - Automatically backs up conflicting files into `.ufm_backups/`.
  - Maintains persistent atomic manifests supporting one-click rollbacks and uninstallation without damaging player personal files.
- 📦 **Native Cross-Platform Installers**:
  - **Windows**: Native `.msi` and NSIS setup packages (x64, WiX Toolset supported)
  - **Linux**: Native `.deb`, `.rpm`, and `.AppImage` packages (x64)
  - **macOS**: Native `.dmg` disk images (Apple Silicon & Intel x86_64)
- 🔍 **Real Runtime Diagnostics**: Built-in rules engine identifying `DllNotFoundException`, `BadImageFormatException`, loader conflicts (MelonLoader vs. BepInEx), Doorstop misconfigurations, with custom log paste analysis and privacy-redacted export.
- 🌐 **Internationalization**: Full bilingual support (English & Simplified Chinese) with dark and light themes.

---

## 🏗️ Architecture

The project is structured as a decoupled **Cargo Workspace**:

```text
ufm/
├── crates/
│   ├── ufm-core/         # Core data types and contracts (EnvironmentReport, Confidence, EvidenceItem)
│   ├── ufm-runtime/      # Binary header static analysis (PE/ELF/Mach-O parsing and runtime inspection)
│   ├── ufm-detector/     # Unity data directory detection, engine version extraction, loader inspection
│   ├── ufm-registry/     # BepInEx catalog models, builds.bepinex.dev scraper, and release feeds
│   ├── ufm-compat/       # Multi-dimensional compatibility evaluation and recommendation rules engine
│   ├── ufm-download/     # Download scheduler, cache manager, and SHA-256 integrity verifier
│   ├── ufm-installer/    # Safe zip archive extraction (Zip Slip guarded), plan builder, atomic transactions & rollback
│   ├── ufm-diagnostics/  # Runtime log and environment issue diagnosis rules engine
│   └── ufm-report/       # Markdown and JSON report exporter with sensitive information redactor
├── src-tauri/            # Tauri 2 native desktop application host and IPC bridge
├── web/                  # Modern React + TypeScript + CSS Variables frontend
└── packaging/            # Native packaging configurations (WiX .wxs, RPM .spec, macOS Info.plist)
```

---

## 🚀 Getting Started & Local Development

### Prerequisites

- [Rust](https://www.rust-lang.org/) (1.80+)
- [Bun](https://bun.sh/) (recommended) or Node.js (18+)
- Tauri 2 OS prerequisites (see [Tauri Prerequisites Documentation](https://v2.tauri.app/start/prerequisites/))

### Steps

1. **Clone the repository**:
   ```bash
   git clone https://github.com/youugiuhiuh/ufm.git
   cd ufm
   ```

2. **Install frontend dependencies**:
   ```bash
   cd web
   bun install
   bun run build
   cd ..
   ```

3. **Run Rust workspace test suite**:
   ```bash
   cargo test --workspace --exclude ufm-desktop
   ```

4. **Run end-to-end sandbox pipeline demonstration**:
   ```bash
   cargo run --example e2e_pipeline_test -p ufm-installer -- "/path/to/unity/game"
   ```

5. **Start desktop development mode**:
   ```bash
   bun --cwd web dev
   # In another terminal:
   cargo tauri dev
   ```

---

## 📄 License

This project is licensed under a **Dual License**:

- **[MIT License](LICENSE-MIT)**
- **[Apache 2.0 License](LICENSE-APACHE)**

You may choose either license at your option.
