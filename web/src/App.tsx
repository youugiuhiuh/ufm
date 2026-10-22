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
  SunMoon 
} from 'lucide-react';
import './App.css';

export function App() {
  const { t, i18n } = useTranslation();
  const [activeTab, setActiveTab] = useState<'home' | 'environment' | 'frameworks' | 'diagnostics' | 'history' | 'settings'>('home');
  const [theme, setTheme] = useState<'dark' | 'light'>('dark');

  const toggleLanguage = () => {
    const next = i18n.language === 'zh-CN' ? 'en-US' : 'zh-CN';
    i18n.changeLanguage(next);
  };

  const toggleTheme = () => {
    const next = theme === 'dark' ? 'light' : 'dark';
    setTheme(next);
    document.documentElement.setAttribute('data-theme', next);
  };

  return (
    <div className="app-container">
      {/* 侧边导航 */}
      <aside className="sidebar">
        <div className="brand">
          <Layers className="brand-icon" size={24} />
          <span className="brand-text">UFM</span>
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
            <Globe2 size={18} />
            <span className="btn-label">{i18n.language === 'zh-CN' ? '中' : 'EN'}</span>
          </button>
          <button className="icon-btn" onClick={toggleTheme} title={t('common.theme')}>
            <SunMoon size={18} />
          </button>
        </div>
      </aside>

      {/* 主工作区 */}
      <main className="main-content">
        <header className="content-header">
          <h2>{t(`nav.${activeTab}`)}</h2>
          <span className="header-subtitle">{t('app.subtitle')}</span>
        </header>

        <div className="content-body">
          {activeTab === 'home' && (
            <div className="home-view">
              <div className="hero-card">
                <FolderSearch size={48} className="hero-icon" />
                <h3>{t('home.selectGame')}</h3>
                <p>{t('home.selectGameDesc')}</p>
                <button className="primary-btn">
                  {t('home.selectGame')}
                </button>
              </div>

              <div className="recent-section">
                <h4>{t('home.recentGames')}</h4>
                <div className="empty-placeholder">
                  {t('home.noRecent')}
                </div>
              </div>
            </div>
          )}

          {activeTab !== 'home' && (
            <div className="placeholder-view">
              <p>功能模块 [{t(`nav.${activeTab}`)}] 正在按开发阶段接入中...</p>
            </div>
          )}
        </div>
      </main>
    </div>
  );
}

export default App;
