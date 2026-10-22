# Unity Framework Manager (UFM)

<p align="center">
  <strong>开源 · 跨平台 · 证据驱动 · 简体中文优先的 Unity 游戏运行环境检测与 Mod 框架管理器</strong>
</p>

<p align="center">
  <a href="https://github.com/youugiuhiuh/ufm/actions"><img src="https://github.com/youugiuhiuh/ufm/actions/workflows/public-release.yml/badge.svg" alt="Release Build" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg" alt="License" /></a>
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Tauri-v2-24C8DB.svg" alt="Tauri 2" /></a>
  <a href="https://react.dev/"><img src="https://img.shields.io/badge/React-19-61DAFB.svg" alt="React 19" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-1.80%2B-orange.svg" alt="Rust" /></a>
</p>

---

## 📖 简介

**Unity Framework Manager (UFM)** 是一款专为 Unity 游戏玩家与 Mod 开发者设计的跨平台桌面工具。通过多维静态证据链精准分析游戏运行环境（Unity 版本、Mono / IL2CPP 运行时、ELF/PE/Mach-O 架构、现有加载器），智能匹配官方构建，并以原子事务保证安全安装、自动备份及无损回滚。

### 核心特性

- 🎯 **证据驱动检测**：拒绝单文件猜测。通过分析可执行文件头特征、引擎核心库资产（`globalgamemanagers` 等）、运行时特征（`MonoBleedingEdge` / `Managed` / `il2cpp_data`），给出具备证据等级（Confirmed / HighProbability / Conflict / Unknown）的可解释报告。
- 🛡️ **安全安装事务**：安装前生成计划预览，展示新增与修改覆盖的文件清单。覆盖已有配置前自动备份至 `.ufm_backups/`，记录原子清单以实现一键无损卸载，严防误删玩家私有文件。
- 📦 **原生安装包分发**：
  - **Windows**：原生 `.msi` 安装包（支持 amd64 / arm64，基于 WiX Toolset）
  - **Linux**：原生 `.deb` 与 `.rpm` 安装包（支持 amd64 / arm64）
  - **macOS**：原生 `.dmg` 磁盘映像（支持 Apple Silicon 与 Intel 芯片）
- 🔍 **故障异常诊断**：内置常见异常规则引擎（识别 `DllNotFoundException`、`BadImageFormatException`、双加载器竞争冲突等），给出修复建议与一键脱敏报告导出。
- 🌐 **简体中文优先**：完整支持中英双语国际化切换与深浅主题切换。

---

## 🏗️ 架构设计

项目采用标准 **Cargo Workspace** 进行强解耦分层：

```text
ufm/
├── crates/
│   ├── ufm-core/         # 核心公共类型契约 (EnvironmentReport, Confidence, EvidenceItem)
│   ├── ufm-runtime/      # 二进制头特征静态分析 (PE/ELF/Mach-O 架构解析与运行时特征)
│   ├── ufm-detector/     # Unity 数据目录识别、引擎版本提取、现有框架/加载器扫描
│   ├── ufm-registry/     # BepInEx 6 官方构建元数据模型与发布源定义
│   ├── ufm-compat/       # 环境匹配规则引擎、多维兼容性评估与风险阻断
│   ├── ufm-download/     # 下载调度器、本地缓存管理与 SHA-256 完整性校验
│   ├── ufm-installer/    # 原生 Zip 解压 (防 Zip Slip)、安装计划预览、原子事务与回滚卸载
│   ├── ufm-diagnostics/  # 运行时日志异常解析与故障归类建议
│   └── ufm-report/       # Markdown 与 JSON 报告导出，支持敏感个人隐私自动脱敏
├── src-tauri/            # Tauri 2 桌面端宿主与 IPC 通信层
├── web/                  # React 19 + TypeScript + CSS Variables 现代化前端
└── packaging/            # 各平台原生安装包打包规范 (WiX .wxs, RPM .spec, macOS Info.plist)
```

---

## 🚀 快速上手与本地开发

### 环境准备

- [Rust](https://www.rust-lang.org/) (1.80+)
- [Bun](https://bun.sh/) (推荐) 或 Node.js (18+)
- Tauri 2 系统依赖（参考 [Tauri 官方前置条件](https://v2.tauri.app/start/prerequisites/)）

### 运行步骤

1. **克隆仓库**：
   ```bash
   git clone https://github.com/youugiuhiuh/ufm.git
   cd ufm
   ```

2. **安装前端依赖并启动开发**：
   ```bash
   cd web
   bun install
   bun dev
   ```

3. **运行 Rust 核心测试套件**：
   ```bash
   cargo test --workspace
   ```

4. **运行端到端流水线沙盒测试**：
   ```bash
   cargo run --example e2e_pipeline_test -p ufm-installer
   ```

---

## 📄 开源许可证

本项目采用 **双重开源许可证（Dual License）**：

- **[MIT 许可证](LICENSE-MIT)**
- **[Apache 2.0 许可证](LICENSE-APACHE)**

你可以根据自身需要在上述许可证中选择其一使用。
