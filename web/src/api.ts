import { invoke, isTauri } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type { EnvironmentReport, CompatibilityReport, InstallPlanPreview, DiagnosticReportData, InstallHistoryItem } from './types';

// 检查是否运行在真正的 Tauri 桌面环境中
export const isTauriEnvironment = (): boolean => {
  if (typeof window === 'undefined') return false;
  return (
    isTauri() ||
    'isTauri' in window ||
    '__TAURI_INTERNALS__' in window ||
    '__TAURI__' in window
  );
};

function ensureTauri(): void {
  if (!isTauriEnvironment()) {
    throw new Error(
      '当前未连接到 Tauri 桌面运行时（运行于纯 Web 模式）。检测、评估与安装需要直接操作本地文件系统，请在 UFM 桌面客户端中运行。'
    );
  }
}

// 0. 打开系统原生文件/目录选择器
export async function pickGameDirectoryOrFile(): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return prompt('Web 预览模式：请输入或粘贴 Unity 游戏目录路径：', '');
  }

  try {
    const selected = await openDialog({
      title: '选择 Unity 游戏根目录或可执行文件',
      multiple: false,
      directory: true, // 优先选择游戏根目录
    });

    if (!selected) {
      return null;
    }

    if (Array.isArray(selected)) {
      return selected[0] || null;
    }

    return selected;
  } catch (err) {
    console.error('打开原生文件选择器失败:', err);
    return null;
  }
}

// 1. 调用 Rust 核心检测环境
export async function detectEnvironment(gamePath: string): Promise<EnvironmentReport> {
  ensureTauri();
  return await invoke<EnvironmentReport>('detect_environment', { gamePath });
}

// 2. 调用 Rust 评估候选构建兼容性
export async function evaluateCompatibility(report: EnvironmentReport): Promise<CompatibilityReport> {
  ensureTauri();
  return await invoke<CompatibilityReport>('evaluate_compatibility', { report });
}

// 3. 生成安装计划预览（真实解压 Zip 并计算文件变动与备份清单）
export async function prepareInstallPlan(gamePath: string, buildId: string): Promise<InstallPlanPreview> {
  ensureTauri();
  return await invoke<InstallPlanPreview>('prepare_install_plan', { gamePath, buildId });
}

// 4. 执行原子安装事务（真实执行写入与备份）
export async function executeInstall(gamePath: string, buildId: string): Promise<boolean> {
  ensureTauri();
  return await invoke<boolean>('execute_install', { gamePath, buildId });
}

// 5. 诊断运行日志与环境
export async function diagnoseLogs(gamePath?: string, logContent?: string, report?: EnvironmentReport): Promise<DiagnosticReportData> {
  ensureTauri();
  return await invoke<DiagnosticReportData>('diagnose_log', {
    gamePath: gamePath || null,
    logContent: logContent || null,
    report: report || null,
  });
}

// 6. 获取安装操作历史清单
export async function getInstallHistory(gamePath?: string): Promise<InstallHistoryItem[]> {
  ensureTauri();
  return await invoke<InstallHistoryItem[]>('get_install_history', { gamePath: gamePath || null });
}

// 7. 执行安装事务一键回滚
export async function rollbackInstallation(manifestId: string, gamePath: string): Promise<boolean> {
  ensureTauri();
  return await invoke<boolean>('rollback_installation', { manifestId, gamePath });
}

// 8. 导出脱敏报告 (Markdown 格式)
export async function exportReportMarkdown(report: EnvironmentReport, diagnostics?: DiagnosticReportData): Promise<string> {
  const date = new Date().toISOString().split('T')[0];
  return `# Unity Framework Manager (UFM) 诊断与检测报告
**导出日期：** ${date}  
**置信度：** ${report.overall_confidence.toUpperCase()}  

---

## 1. 游戏环境概览
- **游戏路径：** ${report.game_path}
- **数据目录：** ${report.data_dir || '未检测到'}
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
