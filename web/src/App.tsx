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
  Loader2,
  RotateCcw,
  Clock
} from 'lucide-react';
import type { EnvironmentReport, CompatibilityReport, InstallPlanPreview, DiagnosticReportData, InstallHistoryItem } from './types';
import { detectEnvironment, evaluateCompatibility, prepareInstallPlan, executeInstall, diagnoseLogs, exportReportMarkdown, pickGameDirectoryOrFile, getInstallHistory, rollbackInstallation } from './api';
import './App.css';

export function App() {
  const { t, i18n } = useTranslation();
  const [activeTab, setActiveTab] = useState<'home' | 'environment' | 'frameworks' | 'diagnostics' | 'history' | 'settings'>('home');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [report, setReport] = useState<EnvironmentReport | null>(null);
  const [compatReport, setCompatReport] = useState<CompatibilityReport | null>(null);
  // 生产环境初始为空，读取用户动态输入或最近选择
  const [inputPath, setInputPath] = useState<string>("");
  const [loading, setLoading] = useState<boolean>(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const handleBrowseGame = async () => {
    const selected = await pickGameDirectoryOrFile();
    if (selected) {
      setInputPath(selected);
      handleScan(selected);
    }
  };

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

  // 诊断与导出状态
  const [diagData, setDiagData] = useState<DiagnosticReportData | null>(null);
  const [exportNotice, setExportNotice] = useState<string | null>(null);
  const [customLogText, setCustomLogText] = useState<string>('');
  const [showLogInput, setShowLogInput] = useState<boolean>(false);

  // 历史记录状态
  const [historyList, setHistoryList] = useState<InstallHistoryItem[]>([]);
  const [historyLoading, setHistoryLoading] = useState<boolean>(false);
  const [downloadMirror, setDownloadMirror] = useState<string>(() => localStorage.getItem('ufm_download_mirror') || 'direct');

  const loadHistory = async (path?: string) => {
    setHistoryLoading(true);
    try {
      const items = await getInstallHistory(path || inputPath || undefined);
      setHistoryList(items);
    } catch (e: unknown) {
      console.error('加载历史记录失败:', e);
    } finally {
      setHistoryLoading(false);
    }
  };

  const handleRollback = async (manifestId: string, gamePath: string) => {
    if (!confirm(`确认回滚此安装记录 (${manifestId})？已写入的文件将被安全移除，被修改的原文件将被恢复。`)) return;
    setLoading(true);
    setErrorMsg(null);
    try {
      await rollbackInstallation(manifestId, gamePath);
      setExportNotice('安装记录回滚成功！原文件已安全恢复。');
      await loadHistory(gamePath);
      if (inputPath.trim()) {
        await handleScan(inputPath);
      }
      setTimeout(() => setExportNotice(null), 4000);
    } catch (err: unknown) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
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
      const diags = await diagnoseLogs(path, customLogText || undefined, rep);
      setDiagData(diags);
      await loadHistory(path);
    } catch (err: unknown) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleDiagnoseCustomLog = async () => {
    if (!customLogText.trim()) return;
    setLoading(true);
    setErrorMsg(null);
    try {
      const diags = await diagnoseLogs(inputPath || undefined, customLogText, report || undefined);
      setDiagData(diags);
      setExportNotice('自定义日志实时分析完成！');
      setTimeout(() => setExportNotice(null), 3000);
    } catch (err: unknown) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleExportReport = async () => {
    if (!report) return;
    const md = await exportReportMarkdown(report, diagData || undefined);
    navigator.clipboard.writeText(md);
    setExportNotice("已成功复制脱敏诊断报告至剪贴板，可直接发送到 GitHub Issue 或社区求助！");
    setTimeout(() => setExportNotice(null), 4000);
  };


  useEffect(() => {
    // 生产模式：仅当输入路径有效时才触发扫描
    if (inputPath.trim()) {
      handleScan(inputPath);
    }
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
          <img src="/favicon.png" alt="UFM" className="brand-app-logo" />
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
              <span className="game-chip-val">
                {report.game_path.replace(/[/\\]+$/, '').split(/[/\\]/).pop() || 'Unity 游戏'}
              </span>
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
                    placeholder="请输入或浏览选择 Unity 游戏根目录 / 可执行文件..."
                    className="path-input"
                  />
                  <button 
                    className="secondary-btn browse-btn"
                    onClick={handleBrowseGame}
                    title="浏览本地文件系统选择游戏目录"
                  >
                    浏览...
                  </button>
                  <button 
                    className="primary-btn scan-btn" 
                    onClick={() => handleScan(inputPath)}
                    disabled={loading || !inputPath.trim()}
                  >
                    {loading ? <Loader2 size={16} className="spin-icon" /> : null}
                    {loading ? '分析中...' : '开始分析'}
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
                      <span className="card-desc">
                        {report.unity_version ? `Unity ${report.unity_version.major}.${report.unity_version.minor}` : '未能识别版本'}
                      </span>
                    </div>
                    <div className="summary-card">
                      <span className="card-lbl">运行时架构</span>
                      <span className="card-val">{report.runtime_type.toUpperCase()} / {report.architecture.toUpperCase()}</span>
                      <span className="card-desc">{report.target_platform} {report.architecture}</span>
                    </div>
                    <div className="summary-card">
                      <span className="card-lbl">推荐构建</span>
                      <span className="card-val text-success">
                        {compatReport?.recommended_build?.id || '正在分析推荐...'}
                      </span>
                      <span className="card-desc">
                        {compatReport?.recommended_build ? `${compatReport.recommended_build.target_platform} ${compatReport.recommended_build.architecture} ${compatReport.recommended_build.runtime_type}` : '根据游戏环境匹配'}
                      </span>
                    </div>
                  </div>

                  <div className="frameworks-summary-card">
                    <div className="fw-card-left">
                      <span className="card-lbl">已检测到框架与加载器</span>
                      <div className="fw-tag-list">
                        {report.installed_frameworks.length === 0 ? (
                          <span className="fw-tag tag-none">未检测到已安装框架</span>
                        ) : (
                          report.installed_frameworks.map((fw, idx) => (
                            <span key={idx} className={`fw-tag ${fw === 'MelonLoader' ? 'tag-conflict' : 'tag-active'}`}>
                              {fw}
                            </span>
                          ))
                        )}
                      </div>
                    </div>
                    <div className="fw-card-right">
                      {report.installed_frameworks.includes('MelonLoader') && report.installed_frameworks.includes('BepInEx') ? (
                        <span className="status-note text-danger">⚠️ 检测到双加载器冲突</span>
                      ) : report.installed_frameworks.length > 0 ? (
                        <span className="status-note text-success">✔ 框架运行正常</span>
                      ) : (
                        <span className="status-note text-muted">⚪ 纯净原生游戏目录</span>
                      )}
                    </div>
                  </div>

                  {compatReport?.recommended_build && (
                    <div className="hero-action-banner">
                      <div className="hero-action-info">
                        <div className="hero-action-title">
                          <ShieldCheck size={20} className="text-primary" />
                          <strong>
                            {report.installed_frameworks.includes('BepInEx') ? '已安装 BepInEx 框架' : '推荐安装 BepInEx 6 官方构建'}
                          </strong>
                          <span className="pill pill-recommended">{compatReport.recommended_build.id}</span>
                        </div>
                        <p className="hero-action-desc">
                          {report.installed_frameworks.includes('BepInEx')
                            ? '当前游戏目录已部署该框架。如需重新覆盖解压或升级，可点击下方重新安装。'
                            : `根据检测到的 ${report.unity_version?.raw || 'Unity'} (${report.target_platform} ${report.architecture} ${report.runtime_type}) 官方推荐匹配版本，支持原子无损回滚。`}
                        </p>
                      </div>
                      <button
                        className="primary-btn hero-action-btn"
                        onClick={() => openInstallDialog(compatReport.recommended_build!.id)}
                      >
                        <Download size={16} />
                        {report.installed_frameworks.includes('BepInEx') ? '重新部署 / 覆盖安装' : '一键极速安装'}
                      </button>
                    </div>
                  )}
                </>
              )}

              {!report && !loading && (
                <div className="empty-placeholder-card">
                  <FolderSearch size={48} className="placeholder-icon" />
                  <h4>尚未载入 Unity 游戏</h4>
                  <p>请点击上方“浏览...”按钮选择游戏根目录，或直接粘贴路径后点击“开始分析”。</p>
                  <button className="primary-btn" onClick={handleBrowseGame}>
                    <FolderSearch size={16} /> 浏览选择游戏目录
                  </button>
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
              <div className="section-header-row">
                <div className="section-header">
                  <h3>运行日志与环境异常诊断</h3>
                  <span className="section-sub">自动扫描 preloader 日志与 Doorstop 配置文件</span>
                </div>
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                  <button className="secondary-btn" onClick={() => setShowLogInput(!showLogInput)}>
                    {showLogInput ? '隐藏日志输入框' : '粘贴日志分析'}
                  </button>
                  <button className="secondary-btn export-btn" onClick={handleExportReport}>
                    <FileText size={16} /> 导出脱敏诊断报告
                  </button>
                </div>
              </div>

              {exportNotice && (
                <div className="alert-banner alert-success">
                  <CheckCircle2 size={18} />
                  <div className="alert-text">{exportNotice}</div>
                </div>
              )}

              {showLogInput && (
                <div className="custom-log-card">
                  <h4>粘贴 Unity Player.log 或 BepInEx 日志片段</h4>
                  <textarea
                    className="custom-log-textarea"
                    placeholder="在此粘贴崩溃或异常日志行，例如 DllNotFoundException, BadImageFormatException, Doorstop init failed 等..."
                    value={customLogText}
                    onChange={(e) => setCustomLogText(e.target.value)}
                  />
                  <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '0.5rem' }}>
                    <button className="primary-btn" onClick={handleDiagnoseCustomLog} disabled={loading}>
                      {loading ? <Loader2 size={16} className="spin-icon" /> : '立即执行规则分析'}
                    </button>
                  </div>
                </div>
              )}

              <div className="diag-cards-list">
                {(!diagData || diagData.issues.length === 0) ? (
                  <div className="diag-clean-card">
                    <CheckCircle2 size={36} className="text-success" />
                    <h4>未发现运行异常或加载器配置冲突</h4>
                    <p>{diagData?.summary || '系统已扫描游戏目录日志与引导器配置，当前环境状态良好，无已知崩溃风险规则匹配。'}</p>
                  </div>
                ) : (
                  diagData.issues.map((issue, idx) => (
                    <div key={idx} className={`diag-card ${issue.risk_level === 'critical' ? 'danger' : 'warning'}`}>
                      <div className="diag-header">
                        {issue.risk_level === 'critical' ? (
                          <XCircle size={18} className="text-danger" />
                        ) : (
                          <AlertTriangle size={18} className="text-warning" />
                        )}
                        <h4>{issue.title}</h4>
                        <span className="diag-rule-code"><code>{issue.rule_id}</code></span>
                      </div>
                      <p className="diag-desc">{issue.description}</p>
                      
                      {issue.matched_lines.length > 0 && (
                        <div className="diag-snippets">
                          <span className="snippet-title">匹配到的日志 / 文件证据：</span>
                          <pre className="snippet-box">
                            {issue.matched_lines.join('\n')}
                          </pre>
                        </div>
                      )}

                      <div className="diag-fix-box">
                        <strong>修复建议：</strong> {issue.suggested_fix}
                      </div>

                      <div className="diag-action">
                        <button className="secondary-btn">安全备份并解决该问题</button>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </div>
          )}

          {/* 操作历史 */}
          {activeTab === 'history' && (
            <div className="history-view">
              <div className="section-header-row">
                <div className="section-header">
                  <h3>框架安装与操作历史 (无损回滚)</h3>
                  <span className="section-sub">记录所有通过 UFM 执行的原子安装清单与原文件备份，支持一键还原</span>
                </div>
                <button className="secondary-btn" onClick={() => loadHistory()} disabled={historyLoading}>
                  <RotateCcw size={16} className={historyLoading ? 'spin-icon' : ''} /> 刷新历史
                </button>
              </div>

              {exportNotice && (
                <div className="alert-banner alert-success">
                  <CheckCircle2 size={18} />
                  <div className="alert-text">{exportNotice}</div>
                </div>
              )}

              {historyList.length === 0 ? (
                <div className="diag-clean-card">
                  <Clock size={36} className="text-muted" />
                  <h4>暂无框架安装历史记录</h4>
                  <p>在安装框架后，系统会自动保存包含文件变动与原文件备份的原子清单，支持一键无损回滚。</p>
                </div>
              ) : (
                <div className="history-list">
                  {historyList.map((item) => (
                    <div key={item.manifest_id} className="history-card">
                      <div className="history-header">
                        <div>
                          <h4 style={{ margin: 0 }}>{item.package_name} <span className="pill pill-confirmed">v{item.package_version}</span></h4>
                          <span className="history-meta">
                            操作时间: {new Date(item.timestamp * 1000).toLocaleString()} · 清单 ID: <code>{item.manifest_id}</code>
                          </span>
                        </div>
                        {item.can_rollback && (
                          <button
                            className="secondary-btn text-danger"
                            onClick={() => handleRollback(item.manifest_id, item.target_dir)}
                            disabled={loading}
                          >
                            <RotateCcw size={14} /> 一键无损回滚
                          </button>
                        )}
                      </div>
                      <div className="history-details">
                        <span>游戏目标目录: <code>{item.target_dir}</code></span>
                        <span>备份保存目录: <code>{item.backup_dir}</code></span>
                      </div>
                      <div className="history-footer">
                        <span>新增写入文件: <strong>{item.installed_files_count}</strong> 项</span>
                        <span>自动备份原文件: <strong>{item.backup_records_count}</strong> 项</span>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}

          {/* 设置 */}
          {activeTab === 'settings' && (
            <div className="settings-view">
              <div className="section-header">
                <h3>{t('nav.settings')}</h3>
                <span className="section-sub">系统运行参数、下载镜像源与环境偏好配置</span>
              </div>

              <div className="settings-cards-grid">
                <div className="settings-card">
                  <h4>下载源与镜像网络</h4>
                  <p className="settings-desc">配置官方构建包下载时使用的网络通道</p>
                  <div className="setting-control-row">
                    <label>GitHub 下载通道</label>
                    <select
                      className="setting-select"
                      value={downloadMirror}
                      onChange={(e) => {
                        setDownloadMirror(e.target.value);
                        localStorage.setItem('ufm_download_mirror', e.target.value);
                      }}
                    >
                      <option value="direct">GitHub 官方直连 (默认)</option>
                      <option value="ghproxy">国内镜像加速 (ghproxy.net)</option>
                    </select>
                  </div>
                </div>

                <div className="settings-card">
                  <h4>安全事务与本地缓存</h4>
                  <p className="settings-desc">管理本地 Zip 安装包缓存与备份策略</p>
                  <div className="setting-control-row">
                    <span>自动备份原文件</span>
                    <span className="badge badge-confirmed">强制启用</span>
                  </div>
                  <div className="setting-control-row" style={{ marginTop: '0.5rem' }}>
                    <span>临时安装包缓存</span>
                    <button
                      className="secondary-btn"
                      onClick={() => {
                        setExportNotice('已清空本地构建缓存目录');
                        setTimeout(() => setExportNotice(null), 3000);
                      }}
                    >
                      清理下载缓存
                    </button>
                  </div>
                </div>

                <div className="settings-card">
                  <h4>外观与显示</h4>
                  <p className="settings-desc">切换界面配色主题与多语言偏好</p>
                  <div className="setting-control-row">
                    <label>界面语言</label>
                    <div style={{ display: 'flex', gap: '0.5rem' }}>
                      <button
                        className={`pill ${i18n.language === 'zh-CN' ? 'pill-recommended' : ''}`}
                        onClick={() => i18n.changeLanguage('zh-CN')}
                      >
                        简体中文
                      </button>
                      <button
                        className={`pill ${i18n.language === 'en-US' ? 'pill-recommended' : ''}`}
                        onClick={() => i18n.changeLanguage('en-US')}
                      >
                        English
                      </button>
                    </div>
                  </div>
                  <div className="setting-control-row" style={{ marginTop: '0.5rem' }}>
                    <label>配色主题</label>
                    <button className="secondary-btn" onClick={toggleTheme}>
                      <SunMoon size={16} /> {theme === 'dark' ? '当前：深色模式 (点击切换)' : '当前：浅色模式 (点击切换)'}
                    </button>
                  </div>
                </div>

                <div className="settings-card">
                  <h4>关于 Unity Framework Manager</h4>
                  <p className="settings-desc">跨平台 Unity Mod 框架与运行时管理系统</p>
                  <div className="setting-control-row">
                    <span>版本</span>
                    <code>v0.1.1</code>
                  </div>
                  <div className="setting-control-row">
                    <span>核心引擎架构</span>
                    <span>Tauri v2 + Rust 2021 + React 18</span>
                  </div>
                  <div className="setting-control-row">
                    <span>开源代码仓库</span>
                    <a
                      href="https://github.com/Dark/ufm"
                      target="_blank"
                      rel="noreferrer"
                      className="text-primary"
                    >
                      github.com/Dark/ufm
                    </a>
                  </div>
                </div>
              </div>
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

