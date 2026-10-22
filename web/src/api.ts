import { invoke } from '@tauri-apps/api/core';
import type { EnvironmentReport, CompatibilityReport, InstallPlanPreview, DiagnosticReportData } from './types';

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

// 3. 生成安装计划预览（新增、修改、备份清单）
export async function prepareInstallPlan(gamePath: string, buildId: string): Promise<InstallPlanPreview> {
  if (!isTauriEnvironment()) {
    return {
      package_name: buildId,
      package_version: "6.0.0-pre.1",
      total_actions: 4,
      backup_dir: `${gamePath}/.ufm_backups/20261022_195500`,
      changes: [
        {
          relative_path: "winhttp.dll",
          action_type: "modify",
          size: 26624,
          will_backup: true
        },
        {
          relative_path: "doorstop_config.ini",
          action_type: "modify",
          size: 1451,
          will_backup: true
        },
        {
          relative_path: "BepInEx/core/BepInEx.Core.dll",
          action_type: "add",
          size: 428032,
          will_backup: false
        },
        {
          relative_path: "BepInEx/core/BepInEx.Unity.Mono.Preloader.dll",
          action_type: "add",
          size: 98304,
          will_backup: false
        }
      ]
    };
  }

  return await invoke<InstallPlanPreview>('prepare_install_plan', { gamePath, buildId });
}

// 4. 执行原子安装事务
export async function executeInstall(gamePath: string, buildId: string): Promise<boolean> {
  if (!isTauriEnvironment()) {
    console.log(`[UFM 模拟安装] 针对 ${gamePath} 安装 ${buildId} 成功`);
    await new Promise(resolve => setTimeout(resolve, 800));
    return true;
  }

  return await invoke<boolean>('execute_install', { gamePath, buildId });
}

// 5. 诊断运行日志与环境
export async function diagnoseLogs(gamePath: string, report?: EnvironmentReport): Promise<DiagnosticReportData> {
  if (!isTauriEnvironment()) {
    console.log(`[UFM 诊断] 分析路径: ${gamePath}`);
    return {
      total_issues: 2,
      max_risk: 'warning',
      summary: '检测到 2 项需关注的运行环境与配置问题',
      issues: [
        {
          rule_id: 'LOADER_CONFLICT_MELON_BEPINEX',
          title: 'MelonLoader 与 BepInEx 混装共存',
          risk_level: 'warning',
          description: '目录中同时检测到 MelonLoader (.net6) 和 BepInEx 运行库，两者可能竞争劫持 Unity 运行时导致崩溃。',
          matched_lines: [
            'Found MelonLoader/net6/MelonLoader.dll',
            'Found BepInEx/core/BepInEx.Core.dll'
          ],
          suggested_fix: '建议仅保留一套框架。若使用 BepInEx，请移除或备份重命名 MelonLoader 目录。',
          references: ['https://bepinex.github.io/bepinex_docs/master/articles/advanced/troubleshooting.html']
        },
        {
          rule_id: 'MULTIPLE_HOOK_DLLS',
          title: '检测到多个代理注入钩子 (winhttp.dll / version.dll)',
          risk_level: 'warning',
          description: '游戏根目录下同时存在两个常用注入 DLL，可能产生多重链式拦截导致初始化死锁。',
          matched_lines: [
            'Found winhttp.dll (Doorstop proxy)',
            'Found version.dll (Proxy loader)'
          ],
          suggested_fix: '检查并移除多余的 DLL 钩子，标准 BepInEx 仅需 winhttp.dll。',
          references: []
        }
      ]
    };
  }

  return await invoke<DiagnosticReportData>('diagnose_log', { logContent: '', report });
}

// 6. 导出脱敏报告 (Markdown 格式)
export async function exportReportMarkdown(report: EnvironmentReport, diagnostics?: DiagnosticReportData): Promise<string> {
  const date = new Date().toISOString().split('T')[0];
  return `# Unity Framework Manager (UFM) 诊断与检测报告
**导出日期：** ${date}  
**置信度：** ${report.overall_confidence.toUpperCase()}  

---

## 1. 游戏环境概览
- **Unity 版本：** ${report.unity_version?.raw || '未知'}
- **运行时类型：** ${report.runtime_type.toUpperCase()}
- **程序架构：** ${report.architecture.toUpperCase()}
- **目标平台：** ${report.target_platform}
- **已装框架：** ${report.installed_frameworks.join(', ') || '无'}

---

## 2. 诊断与风险发现
${diagnostics?.issues.map(i => `### [${i.risk_level.toUpperCase()}] ${i.title}\n- **描述：** ${i.description}\n- **修复建议：** ${i.suggested_fix}`).join('\n\n') || '未发现异常'}

---
*本报告由 UFM 客户端离线生成，已自动脱敏敏感路径信息。*
`;
}


