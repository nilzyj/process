import { useState, useEffect, lazy, Suspense } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DbConfig } from './types';
import SetupPage from './components/SetupPage';
import Home from './pages/Home';

const Stats = lazy(() => import('./pages/Stats'));

import './App.css';

type Page = 'home' | 'stats';

export default function App() {
  const [connected, setConnected] = useState(false);
  const [page, setPage] = useState<Page>('home');
  const [showSetup, setShowSetup] = useState(false);

  useEffect(() => {
    const check = async () => {
      try {
        const config = await invoke<DbConfig | null>('get_config');
        if (config) {
          await invoke<string>('init_db', { config });
          setConnected(true);
        } else {
          setShowSetup(true);
        }
      } catch {
        setShowSetup(true);
      }
    };
    check();
  }, []);

  const handleConnected = () => {
    setConnected(true);
    setShowSetup(false);
  };

  if (showSetup) {
    return <SetupPage onConnected={handleConnected} />;
  }

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
        <div style={{ flex: 1 }} />
        <button
          className="btn btn-secondary btn-sm"
          onClick={async () => {
            await invoke('save_config', { config_data: { host: '', port: 3306, user: '', password: '', database: '' } });
            setConnected(false);
            setShowSetup(true);
          }}
        >
          断开
        </button>
      </header>

      <div className="app-content">
        {page === 'home' ? <Home connected={connected} /> : <Suspense fallback={null}><Stats /></Suspense>}
      </div>
    </div>
  );
}
