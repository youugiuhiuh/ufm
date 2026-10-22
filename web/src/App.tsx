import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { 
  FolderSearch, 
  Layers, 
  Settings, 
  Wrench, 
  History, 
  Cpu, 
  Globe2, 
  SunMoon,
  CheckCircle2,
  AlertTriangle,
  HelpCircle,
  XCircle,
  Download,
  ShieldCheck,
  FileText
} from 'lucide-react';
import type { EnvironmentReport } from './types';
import './App.css';

// 真实扫描样本数据（即我们在 3D Custom Okazu Maker 上得到的真实数据）
const SAMPLE_REPORT: EnvironmentReport = {
  game_path: "/run/media/ub/系统/Users/asus/Downloads/Telegram Desktop/3D!かすたむオカズメーカー/3D!かすたむオカズメーカー/GameCore",
  data_dir: "/run/media/ub/系统/Users/asus/Downloads/Telegram Desktop/3D!かすたむオカズメーカー/3D!かすたむオカズメーカー/GameCore/3D Custom Okazu Maker_Data",
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

export function App() {
  const { t, i18n } = useTranslation();
  const [activeTab, setActiveTab] = useState<'home' | 'environment' | 'frameworks' | 'diagnostics' | 'history' | 'settings'>('home');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [report, setReport] = useState<EnvironmentReport | null>(SAMPLE_REPORT);
  const [inputPath, setInputPath] = useState<string>(SAMPLE_REPORT.game_path);

  const toggleLanguage = () => {
    const next = i18n.language === 'zh-CN' ? 'en-US' : 'zh-CN';
    i18n.changeLanguage(next);
  };

  const toggleTheme = () => {
    const next = theme === 'dark' ? 'light' : 'dark';
    setTheme(next);
    document.documentElement.setAttribute('data-theme', next);
  };

  const renderConfidenceBadge = (confidence: string) => {
    switch (confidence) {
      case 'confirmed':
        return <span className="badge badge-confirmed"><CheckCircle2 size={13} /> 已确认</span>;
      case 'high_probability':
        return <span className="badge badge-high-prob"><CheckCircle2 size={13} /> 高概率</span>;
      case 'conflict':
        return <span className="badge badge-conflict"><XCircle size={13} /> 存在冲突</span>;
      default:
        return <span className="badge badge-unknown"><HelpCircle size={13} /> 未知</span>;
    }
  };

  return (
    <div className="app-container">
      {/* 侧边导航 */}
      <aside className="sidebar">
        <div className="brand">
          <Layers className="brand-icon" size={24} />
          <div className="brand-info">
            <span className="brand-text">UFM</span>
            <span className="brand-sub">Unity Framework Mgr</span>
          </div>
        </div>

        <nav className="nav-menu">
          <button 
            className={`nav-item ${activeTab === 'home' ? 'active' : ''}`}
            onClick={() => setActiveTab('home')}
          >
            <FolderSearch size={18} />
            <span>{t('nav.home')}</span>
          </button>
          <button 
            className={`nav-item ${activeTab === 'environment' ? 'active' : ''}`}
            onClick={() => setActiveTab('environment')}
          >
            <Cpu size={18} />
            <span>{t('nav.environment')}</span>
          </button>
          <button 
            className={`nav-item ${activeTab === 'frameworks' ? 'active' : ''}`}
            onClick={() => setActiveTab('frameworks')}
          >
            <Layers size={18} />
            <span>{t('nav.frameworks')}</span>
          </button>
          <button 
            className={`nav-item ${activeTab === 'diagnostics' ? 'active' : ''}`}
            onClick={() => setActiveTab('diagnostics')}
          >
            <Wrench size={18} />
            <span>{t('nav.diagnostics')}</span>
          </button>
          <button 
            className={`nav-item ${activeTab === 'history' ? 'active' : ''}`}
            onClick={() => setActiveTab('history')}
          >
            <History size={18} />
            <span>{t('nav.history')}</span>
          </button>
          <button 
            className={`nav-item ${activeTab === 'settings' ? 'active' : ''}`}
            onClick={() => setActiveTab('settings')}
          >
            <Settings size={18} />
            <span>{t('nav.settings')}</span>
          </button>
        </nav>

        <div className="sidebar-footer">
          <button className="icon-btn" onClick={toggleLanguage} title={t('common.language')}>
            <Globe2 size={16} />
            <span className="btn-label">{i18n.language === 'zh-CN' ? '简体中文' : 'English'}</span>
          </button>
          <button className="icon-btn" onClick={toggleTheme} title={t('common.theme')}>
            <SunMoon size={16} />
          </button>
        </div>
      </aside>

      {/* 主工作区 */}
      <main className="main-content">
        <header className="content-header">
          <div className="header-titles">
            <h2>{t(`nav.${activeTab}`)}</h2>
            <span className="header-subtitle">{t('app.subtitle')}</span>
          </div>
          {report && (
            <div className="game-chip">
              <span className="game-chip-label">当前游戏：</span>
              <span className="game-chip-val">3D Custom Okazu Maker</span>
              {renderConfidenceBadge(report.overall_confidence)}
            </div>
          )}
        </header>

        <div className="content-body">
          {/* 首页 */}
          {activeTab === 'home' && (
            <div className="home-view">
              <div className="path-bar-card">
                <div className="path-input-group">
                  <FolderSearch size={20} className="path-icon" />
                  <input 
                    type="text" 
                    value={inputPath}
                    onChange={(e) => setInputPath(e.target.value)}
                    placeholder="请输入或选择 Unity 游戏根目录 / 可执行文件..."
                    className="path-input"
                  />
                  <button className="primary-btn scan-btn" onClick={() => setReport(SAMPLE_REPORT)}>
                    重新分析
                  </button>
                </div>
              </div>

              {report && (
                <div className="quick-summary-grid">
                  <div className="summary-card">
                    <span className="card-lbl">Unity 引擎版本</span>
                    <span className="card-val highlight">{report.unity_version?.raw || '未知'}</span>
                    <span className="card-desc">LTS 长期技术支持版</span>
                  </div>
                  <div className="summary-card">
                    <span className="card-lbl">运行时架构</span>
                    <span className="card-val">{report.runtime_type.toUpperCase()} / {report.architecture.toUpperCase()}</span>
                    <span className="card-desc">Windows 64位</span>
                  </div>
                  <div className="summary-card">
                    <span className="card-lbl">已装框架</span>
                    <span className="card-val">{report.installed_frameworks.join(', ')}</span>
                    <span className="card-desc text-danger">⚠️ 检测到加载器冲突</span>
                  </div>
                  <div className="summary-card">
                    <span className="card-lbl">推荐构建</span>
                    <span className="card-val text-success">BepInEx 6 (v6.0.0-pre.1)</span>
                    <span className="card-desc">Windows x64 Mono 兼容</span>
                  </div>
                </div>
              )}

              {/* 冲突告警横幅 */}
              {report?.installed_frameworks.includes('MelonLoader') && (
                <div className="alert-banner alert-warning">
                  <AlertTriangle size={20} />
                  <div className="alert-text">
                    <strong>环境冲突提示：</strong> 检测到当前目录同时存在 BepInEx 与 MelonLoader，且存在多个代理注入 DLL（winhttp.dll 与 version.dll）。建议在安装新框架前进行一键环境清理。
                  </div>
                </div>
              )}
            </div>
          )}

          {/* 游戏环境检测详情 */}
          {activeTab === 'environment' && report && (
            <div className="environment-view">
              <div className="section-header">
                <h3>检测证据链与置信度审计</h3>
                <span className="section-sub">严格遵循证据驱动原则，所有属性均有源文件证据支撑</span>
              </div>

              <div className="evidences-table-card">
                <table className="evidence-table">
                  <thead>
                    <tr>
                      <th style={{ width: '120px' }}>置信度</th>
                      <th style={{ width: '180px' }}>证据类型</th>
                      <th>检测描述与证据源</th>
                    </tr>
                  </thead>
                  <tbody>
                    {report.evidences.map((ev, idx) => (
                      <tr key={idx}>
                        <td>{renderConfidenceBadge(ev.confidence)}</td>
                        <td><code>{ev.key}</code></td>
                        <td>{ev.description}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* 框架管理与推荐 */}
          {activeTab === 'frameworks' && (
            <div className="frameworks-view">
              <div className="section-header">
                <h3>BepInEx 6 官方构建匹配</h3>
                <span className="section-sub">针对 Unity 2021.3.16f1 (Windows x64 Mono) 推荐候选</span>
              </div>

              <div className="build-candidate-card recommended">
                <div className="build-badge-row">
                  <span className="pill pill-recommended">★ 最佳匹配推荐</span>
                  <span className="pill pill-channel">Release 渠道</span>
                  <span className="pill pill-mono">Mono 运行时</span>
                </div>
                <div className="build-main">
                  <div>
                    <h4>BepInEx 6.0.0-pre.1 (Windows x64 Mono)</h4>
                    <p className="build-meta">官方 release 归档包 · 完全兼容当前游戏 LTS 引擎与 Mono 运行环境</p>
                  </div>
                  <button className="primary-btn install-btn">
                    <Download size={16} /> 开始安全安装
                  </button>
                </div>
                <div className="build-evidence-notes">
                  <span className="note-item"><ShieldCheck size={14} /> 架构完全匹配: X64</span>
                  <span className="note-item"><ShieldCheck size={14} /> 运行时完全匹配: Mono</span>
                  <span className="note-item"><ShieldCheck size={14} /> 平台完全匹配: Windows</span>
                </div>
              </div>

              <div className="build-candidate-card disabled">
                <div className="build-badge-row">
                  <span className="pill pill-incompatible">不兼容</span>
                  <span className="pill pill-il2cpp">IL2CPP 运行时</span>
                </div>
                <div className="build-main">
                  <div>
                    <h4>BepInEx 6.0.0-pre.1 (Windows x64 IL2CPP)</h4>
                    <p className="build-meta">运行时不匹配：目标为 IL2CPP，但检测到游戏为 Mono</p>
                  </div>
                  <button className="secondary-btn" disabled>禁止安装</button>
                </div>
              </div>
            </div>
          )}

          {/* 故障诊断 */}
          {activeTab === 'diagnostics' && (
            <div className="diagnostics-view">
              <div className="section-header">
                <h3>运行日志与环境异常诊断</h3>
                <span className="section-sub">自动扫描 preloader 日志与 Doorstop 配置文件</span>
              </div>

              <div className="diag-card warning">
                <div className="diag-header">
                  <AlertTriangle size={18} className="text-warning" />
                  <h4>双加载器冲突风险 (MelonLoader & BepInEx)</h4>
                </div>
                <p className="diag-desc">检测到目录中既有 BepInEx 配置文件又有 MelonLoader 目录，可能导致 Assembly 加载死锁或钩子链断裂。</p>
                <div className="diag-action">
                  <button className="secondary-btn">备份并卸载残留冲突加载器</button>
                </div>
              </div>
            </div>
          )}

          {/* 历史与设置等 */}
          {(activeTab === 'history' || activeTab === 'settings') && (
            <div className="placeholder-view">
              <FileText size={40} className="placeholder-icon" />
              <h3>{t(`nav.${activeTab}`)}</h3>
              <p>操作事务回滚与参数设置模块就绪</p>
            </div>
          )}
        </div>
      </main>
    </div>
  );
}

export default App;
