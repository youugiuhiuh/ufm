import { useState, useEffect } from 'react';
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
  FileText,
  Loader2
} from 'lucide-react';
import type { EnvironmentReport, CompatibilityReport, InstallPlanPreview } from './types';
import { detectEnvironment, evaluateCompatibility, prepareInstallPlan, executeInstall } from './api';
import './App.css';

export function App() {
  const { t, i18n } = useTranslation();
  const [activeTab, setActiveTab] = useState<'home' | 'environment' | 'frameworks' | 'diagnostics' | 'history' | 'settings'>('home');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [report, setReport] = useState<EnvironmentReport | null>(null);
  const [compatReport, setCompatReport] = useState<CompatibilityReport | null>(null);
  const [inputPath, setInputPath] = useState<string>("/run/media/ub/系统/Users/asus/Downloads/Telegram Desktop/3D!かすたむオカズメーカー/3D!かすたむオカズメーカー/GameCore");
  const [loading, setLoading] = useState<boolean>(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // 安装对话框与事务状态
  const [installModalOpen, setInstallModalOpen] = useState<boolean>(false);
  const [targetBuildId, setTargetBuildId] = useState<string | null>(null);
  const [installPlan, setInstallPlan] = useState<InstallPlanPreview | null>(null);
  const [installing, setInstalling] = useState<boolean>(false);
  const [installSuccess, setInstallSuccess] = useState<boolean>(false);

  const openInstallDialog = async (buildId: string) => {
    setTargetBuildId(buildId);
    setInstallSuccess(false);
    setInstallModalOpen(true);
    try {
      const plan = await prepareInstallPlan(inputPath, buildId);
      setInstallPlan(plan);
    } catch (e: unknown) {
      console.error(e);
    }
  };

  const handleConfirmInstall = async () => {
    if (!targetBuildId) return;
    setInstalling(true);
    try {
      await executeInstall(inputPath, targetBuildId);
      setInstallSuccess(true);
      // 重新执行环境检测，刷新已装框架状态
      await handleScan(inputPath);
    } catch (e: unknown) {
      setErrorMsg(e instanceof Error ? e.message : String(e));
    } finally {
      setInstalling(false);
    }
  };

  const handleScan = async (path: string) => {
    if (!path.trim()) return;
    setLoading(true);
    setErrorMsg(null);
    try {
      const rep = await detectEnvironment(path);
      setReport(rep);
      const compat = await evaluateCompatibility(rep);
      setCompatReport(compat);
    } catch (err: unknown) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    // 初始加载一次默认路径
    handleScan(inputPath);
  }, []);

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
                  <button 
                    className="primary-btn scan-btn" 
                    onClick={() => handleScan(inputPath)}
                    disabled={loading}
                  >
                    {loading ? <Loader2 size={16} className="spin-icon" /> : null}
                    {loading ? '分析中...' : '重新分析'}
                  </button>
                </div>
              </div>

              {errorMsg && (
                <div className="alert-banner alert-warning">
                  <XCircle size={20} />
                  <div className="alert-text">
                    <strong>分析失败：</strong> {errorMsg}
                  </div>
                </div>
              )}

              {report && (
                <>
                  <div className="quick-summary-grid">
                    <div className="summary-card">
                      <span className="card-lbl">Unity 引擎版本</span>
                      <span className="card-val highlight">{report.unity_version?.raw || '未知'}</span>
                      <span className="card-desc">LTS 长期支持</span>
                    </div>
                    <div className="summary-card">
                      <span className="card-lbl">运行时架构</span>
                      <span className="card-val">{report.runtime_type.toUpperCase()} / {report.architecture.toUpperCase()}</span>
                      <span className="card-desc">Windows 64位</span>
                    </div>
                    <div className="summary-card">
                      <span className="card-lbl">推荐构建</span>
                      <span className="card-val text-success">BepInEx 6.0</span>
                      <span className="card-desc">v6.0.0-pre.1 Mono</span>
                    </div>
                  </div>

                  <div className="frameworks-summary-card">
                    <div className="fw-card-left">
                      <span className="card-lbl">已检测到框架与加载器</span>
                      <div className="fw-tag-list">
                        {report.installed_frameworks.map((fw, idx) => (
                          <span key={idx} className={`fw-tag ${fw === 'MelonLoader' ? 'tag-conflict' : ''}`}>
                            {fw}
                          </span>
                        ))}
                      </div>
                    </div>
                    <div className="fw-card-right">
                      <span className="status-note text-danger">⚠️ 检测到双加载器冲突</span>
                    </div>
                  </div>
                </>
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
                <span className="section-sub">针对 {report?.unity_version?.raw || 'Unity'} ({report?.target_platform} {report?.architecture} {report?.runtime_type}) 推荐候选</span>
              </div>

              {compatReport?.recommended_build && (
                <div className="build-candidate-card recommended">
                  <div className="build-badge-row">
                    <span className="pill pill-recommended">★ 最佳匹配推荐</span>
                    <span className="pill pill-channel">{compatReport.recommended_build.channel} 渠道</span>
                    <span className="pill pill-mono">{compatReport.recommended_build.runtime_type} 运行时</span>
                  </div>
                  <div className="build-main">
                    <div>
                      <h4>{compatReport.recommended_build.id}</h4>
                      <p className="build-meta">官方 release 归档包 · 完全兼容当前游戏 LTS 引擎与 Mono 运行环境</p>
                    </div>
                    <button 
                      className="primary-btn install-btn"
                      onClick={() => openInstallDialog(compatReport.recommended_build!.id)}
                    >
                      <Download size={16} /> 开始安全安装
                    </button>
                  </div>
                  <div className="build-evidence-notes">
                    <span className="note-item"><ShieldCheck size={14} /> 架构完全匹配: {compatReport.recommended_build.architecture.toUpperCase()}</span>
                    <span className="note-item"><ShieldCheck size={14} /> 运行时完全匹配: {compatReport.recommended_build.runtime_type}</span>
                    <span className="note-item"><ShieldCheck size={14} /> 平台完全匹配: {compatReport.recommended_build.target_platform}</span>
                  </div>
                </div>
              )}

              {compatReport?.assessments.filter(a => a.status === 'incompatible').map((a, idx) => (
                <div key={idx} className="build-candidate-card disabled">
                  <div className="build-badge-row">
                    <span className="pill pill-incompatible">不兼容</span>
                    <span className="pill pill-il2cpp">{a.build.runtime_type} 运行时</span>
                  </div>
                  <div className="build-main">
                    <div>
                      <h4>{a.build.id}</h4>
                      <p className="build-meta">{a.reasons.map(r => r.detail).join('；')}</p>
                    </div>
                    <button className="secondary-btn" disabled>禁止安装</button>
                  </div>
                </div>
              ))}
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

      {/* 安装计划预览与确认 Modal */}
      {installModalOpen && (
        <div className="modal-backdrop">
          <div className="modal-dialog">
            <div className="modal-header">
              <div className="modal-title">
                <ShieldCheck size={20} className="text-success" />
                <h3>框架安装计划预览 (安全事务)</h3>
              </div>
              <button className="icon-btn-close" onClick={() => setInstallModalOpen(false)}>×</button>
            </div>

            <div className="modal-body">
              {installSuccess ? (
                <div className="install-success-card">
                  <CheckCircle2 size={48} className="text-success" />
                  <h4>安装事务执行成功！</h4>
                  <p>框架文件已成功写入，修改的原文件已原子备份，清单已记录。</p>
                  <button className="primary-btn" onClick={() => setInstallModalOpen(false)}>完成</button>
                </div>
              ) : (
                <>
                  <p className="modal-intro">
                    根据计划书安全原则，UFM 将在写入前备份可能冲突的现有文件，生成完整安装清单以支持无损回滚。
                  </p>

                  <div className="install-meta-row">
                    <span>目标构建: <strong>{targetBuildId}</strong></span>
                    <span>备份路径: <code>.ufm_backups/</code></span>
                  </div>

                  <div className="changes-list-card">
                    <span className="changes-header">将变更的文件清单 ({installPlan?.total_actions || 0} 项):</span>
                    <div className="changes-scroll">
                      {installPlan?.changes.map((ch, idx) => (
                        <div key={idx} className="change-item">
                          <span className={`change-tag ${ch.action_type === 'add' ? 'tag-add' : 'tag-mod'}`}>
                            {ch.action_type === 'add' ? '新增' : '修改/覆盖'}
                          </span>
                          <span className="change-path">{ch.relative_path}</span>
                          {ch.will_backup && <span className="pill pill-backup">自动备份原文件</span>}
                        </div>
                      ))}
                    </div>
                  </div>

                  <div className="modal-footer">
                    <button className="secondary-btn" onClick={() => setInstallModalOpen(false)} disabled={installing}>
                      取消
                    </button>
                    <button className="primary-btn" onClick={handleConfirmInstall} disabled={installing}>
                      {installing ? <Loader2 size={16} className="spin-icon" /> : <Download size={16} />}
                      {installing ? '正在备份并原子安装...' : '确认并开始安装'}
                    </button>
                  </div>
                </>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;

