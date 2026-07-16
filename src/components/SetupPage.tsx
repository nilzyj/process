import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DbConfig } from '../types';

interface Props {
  onConnected: (config: DbConfig) => void;
}

export default function SetupPage({ onConnected }: Props) {
  const [config, setConfig] = useState<DbConfig>({
    host: '8.136.136.131',
    port: 3307,
    user: 'root',
    password: '',
    database: 'process',
  });
  const [testing, setTesting] = useState(false);
  const [msg, setMsg] = useState('');

  const update = (k: keyof DbConfig, v: string | number) =>
    setConfig((prev) => ({ ...prev, [k]: v }));

  const handleTest = async () => {
    setTesting(true);
    setMsg('');
    try {
      const res = await invoke<string>('test_connection', { config });
      setMsg(res);
    } catch (e) {
      setMsg(`测试失败: ${e}`);
    } finally {
      setTesting(false);
    }
  };

  const handleConnect = async () => {
    setTesting(true);
    setMsg('');
    try {
      const res = await invoke<string>('init_db', { config });
      setMsg(res);
      onConnected(config);
    } catch (e) {
      setMsg(`连接失败: ${e}`);
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="setup-page">
      <div className="setup-card">
        <h2>数据库连接</h2>
        <p>配置 MySQL 连接信息</p>

        <div className="form-group">
          <label>主机地址</label>
          <input value={config.host} onChange={(e) => update('host', e.target.value)} />
        </div>

        <div className="form-row">
          <div className="form-group">
            <label>端口</label>
            <input type="number" value={config.port} onChange={(e) => update('port', Number(e.target.value))} />
          </div>
          <div className="form-group">
            <label>数据库名</label>
            <input value={config.database} onChange={(e) => update('database', e.target.value)} />
          </div>
        </div>

        <div className="form-group">
          <label>用户名</label>
          <input value={config.user} onChange={(e) => update('user', e.target.value)} />
        </div>

        <div className="form-group">
          <label>密码</label>
          <input type="password" value={config.password} onChange={(e) => update('password', e.target.value)} />
        </div>

        {msg && (
          <div style={{ fontSize: 13, marginBottom: 16, color: msg.includes('成功') ? 'var(--success)' : 'var(--danger)' }}>
            {msg}
          </div>
        )}

        <div style={{ display: 'flex', gap: 10 }}>
          <button className="btn btn-secondary" onClick={handleTest} disabled={testing}>
            {testing ? '测试中...' : '测试连接'}
          </button>
          <button className="btn btn-primary" onClick={handleConnect} disabled={testing}>
            {testing ? '连接中...' : '连接'}
          </button>
        </div>
      </div>
    </div>
  );
}
