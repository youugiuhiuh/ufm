import { invoke } from '@tauri-apps/api/core';
import type { EnvironmentReport, CompatibilityReport } from './types';

// 检查是否运行在真正的 Tauri 桌面环境中
export const isTauriEnvironment = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

// 1. 调用 Rust 核心检测环境
export async function detectEnvironment(gamePath: string): Promise<EnvironmentReport> {
  if (!isTauriEnvironment()) {
    console.warn('[UFM] 非 Tauri 运行环境，使用标准模拟结果');
    // 浏览器预览回退（展示 3D Custom Okazu Maker 的真实扫描数据）
    return {
      game_path: gamePath,
      data_dir: `${gamePath}/3D Custom Okazu Maker_Data`,
      unity_version: {
        raw: "2021.3.16f1",
        major: 2021,
        minor: 3,
        patch: 16,
        suffix: "f1"
      },
      runtime_type: "mono",
      architecture: "x64",
      target_platform: "windows",
      installed_frameworks: ["BepInEx", "Doorstop", "MelonLoader"],
      overall_confidence: "confirmed",
      evidences: [
        { key: "unity.data_dir", description: "找到 Unity 数据目录: 3D Custom Okazu Maker_Data", confidence: "confirmed" },
        { key: "unity.version", description: "从 globalgamemanagers 提取版本: 2021.3.16f1", confidence: "confirmed" },
        { key: "mono_bleeding_edge", description: "MonoBleedingEdge 运行环境完整", confidence: "confirmed" },
        { key: "managed_assemblies", description: "包含 Managed/Assembly-CSharp.dll 及核心程序集", confidence: "high_probability" },
        { key: "binary.arch", description: "UnityCrashHandler64.exe 确认 64 位 PE 结构", confidence: "confirmed" },
        { key: "framework.bepinex", description: "已安装 BepInEx 核心库 (BepInEx.Core.dll)", confidence: "confirmed" },
        { key: "framework.doorstop", description: "已配置 Doorstop 引导器 (doorstop_config.ini)", confidence: "confirmed" },
        { key: "framework.melonloader", description: "已检测到 MelonLoader (.net6) 存在潜在冲突", confidence: "conflict" }
      ]
    };
  }

  return await invoke<EnvironmentReport>('detect_environment', { gamePath });
}

// 2. 调用 Rust 评估候选构建兼容性
export async function evaluateCompatibility(report: EnvironmentReport): Promise<CompatibilityReport> {
  if (!isTauriEnvironment()) {
    return {
      assessments: [
        {
          build: {
            id: "bepinex-v6.0.0-pre.1-win-x64-mono",
            version: "6.0.0-pre.1",
            channel: "release",
            runtime_type: "mono",
            architecture: "x64",
            target_platform: "windows",
            download_url: "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityMono_x64_6.0.0-pre.1.zip",
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
          },
          status: "compatible",
          reasons: [
            { aspect: "runtime", state: "matched", detail: "Runtime matches detected Mono." },
            { aspect: "architecture", state: "matched", detail: "Architecture matches detected X64." },
            { aspect: "platform", state: "matched", detail: "Platform matches detected Windows." }
          ],
          warnings: []
        },
        {
          build: {
            id: "bepinex-v6.0.0-pre.1-win-x64-il2cpp",
            version: "6.0.0-pre.1",
            channel: "release",
            runtime_type: "il2cpp",
            architecture: "x64",
            target_platform: "windows",
            download_url: "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityIL2CPP_x64_6.0.0-pre.1.zip",
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
          },
          status: "incompatible",
          reasons: [
            { aspect: "runtime", state: "mismatched", detail: "Runtime mismatch: build targets Il2Cpp, but game is Mono." }
          ],
          warnings: [
            { code: "IL2CPP_NOTICE", message: "IL2CPP 构建无法在 Mono 游戏中执行", severity: "risk" }
          ]
        }
      ],
      recommended_build: {
        id: "bepinex-v6.0.0-pre.1-win-x64-mono",
        version: "6.0.0-pre.1",
        channel: "release",
        runtime_type: "mono",
        architecture: "x64",
        target_platform: "windows",
        download_url: "https://github.com/BepInEx/BepInEx/releases/download/v6.0.0-pre.1/BepInEx_UnityMono_x64_6.0.0-pre.1.zip",
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
      },
      global_warnings: []
    };
  }

  return await invoke<CompatibilityReport>('evaluate_compatibility', { report });
}
