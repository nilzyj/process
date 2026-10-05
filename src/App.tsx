import { useState, useEffect, lazy, Suspense } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { StorageInfo } from './types';
import Home from './pages/Home';

const Stats = lazy(() => import('./pages/Stats'));
const Library = lazy(() => import('./pages/Library'));
const Settings = lazy(() => import('./pages/Settings'));

import './App.css';

type Page = 'home' | 'stats' | 'library' | 'settings';

export default function App() {
  // 本地 SQLite 无「连接」概念：init_storage 失败只可能是磁盘/权限问题
  const [ready, setReady] = useState(false);
  const [fatal, setFatal] = useState('');
  const [info, setInfo] = useState<StorageInfo | null>(null);
  const [page, setPage] = useState<Page>('home');

  useEffect(() => {
    const init = async () => {
      try {
        const i = await invoke<StorageInfo>('init_storage');
        setInfo(i);
        setReady(true);
      } catch (e) {
        setFatal(String(e));
      }
    };
    init();
  }, []);

  if (fatal) {
    return (
      <div className="setup-page">
        <div className="setup-card">
          <h2>无法打开本地数据库</h2>
          <p style={{ color: 'var(--danger)', wordBreak: 'break-all' }}>{fatal}</p>
          <p style={{ color: 'var(--text-secondary)', fontSize: 'var(--fs-md)' }}>
            请检查目录权限与磁盘空间。若数据目录被坚果云同步，请把它移出同步目录后再试。
          </p>
        </div>
      </div>
    );
  }

  if (!ready) {
    return (
      <div className="setup-page">
        <div className="setup-card">
          <div className="loading-spinner" />
          <p>正在打开本地数据库...</p>
        </div>
      </div>
    );
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
        <button className={`tab-btn ${page === 'library' ? 'active' : ''}`} onClick={() => setPage('library')}>
          媒体库
        </button>
        <div style={{ flex: 1 }} />
        <span className="app-record-count">{info?.record_count ?? 0} 条</span>
        <button
          className={`tab-btn ${page === 'settings' ? 'active' : ''}`}
          onClick={() => setPage('settings')}
        >
          设置
        </button>
      </header>

      <div className="app-content">
        {page === 'home' ? (
          <Home />
        ) : page === 'stats' ? (
          <Suspense fallback={null}><Stats /></Suspense>
        ) : page === 'library' ? (
          <Suspense fallback={null}><Library /></Suspense>
        ) : (
          <Suspense fallback={null}><Settings /></Suspense>
        )}
      </div>
    </div>
  );
}