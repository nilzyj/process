import { useState, useEffect, lazy, Suspense } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DbConfig } from './types';
import SetupPage from './components/SetupPage';
import Home from './pages/Home';

const Stats = lazy(() => import('./pages/Stats'));
const Library = lazy(() => import('./pages/Library'));

import './App.css';

type Page = 'home' | 'stats' | 'library';

export default function App() {
  const [connected, setConnected] = useState(false);
  const [page, setPage] = useState<Page>('home');

  useEffect(() => {
    const check = async () => {
      try {
        const config = await invoke<DbConfig | null>('get_config');
        if (config) {
          await invoke<string>('init_db', { config });
          setConnected(true);
        } else {
          // 媒体库不依赖数据库，未配置 DB 时默认落在这一页
          setPage('library');
        }
      } catch {
        setPage('library');
      }
    };
    check();
  }, []);

  const handleConnected = () => setConnected(true);

  // 主页与统计依赖数据库；媒体库始终可用
  const showSetup = !connected && (page === 'home' || page === 'stats');

  return (
    <div className="app-layout">
      <header className="app-header">
        <span className="app-title">PROCESS</span>
        <button className={`tab-btn ${page === 'home' ? 'active' : ''}`} onClick={() => setPage('home')}>
          主页
        </button>
        <button className={`tab-btn ${page === 'stats' ? 'active' : ''}`} onClick={() => setPage('stats')}>
          统计
        </button>
        <button className={`tab-btn ${page === 'library' ? 'active' : ''}`} onClick={() => setPage('library')}>
          媒体库
        </button>
        <div style={{ flex: 1 }} />
        {connected ? (
          <button
            className="btn btn-secondary btn-sm"
            onClick={async () => {
              await invoke('save_config', { config_data: { host: '', port: 3306, user: '', password: '', database: '' } });
              setConnected(false);
              setPage('library');
            }}
          >
            断开
          </button>
        ) : (
          <button className="btn btn-secondary btn-sm" onClick={() => setPage('home')}>
            连接数据库
          </button>
        )}
      </header>

      <div className="app-content">
        {showSetup ? (
          <SetupPage onConnected={handleConnected} onSkip={() => setPage('library')} />
        ) : page === 'home' ? (
          <Home connected={connected} />
        ) : page === 'stats' ? (
          <Suspense fallback={null}><Stats /></Suspense>
        ) : (
          <Suspense fallback={null}><Library /></Suspense>
        )}
      </div>
    </div>
  );
}
