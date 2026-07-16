import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Stats as StatsType } from '../types';
import { MEDIA_TYPES, STATUS_STYLES } from '../types';

export default function Stats() {
  const [stats, setStats] = useState<StatsType | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetch = async () => {
      try {
        const data = await invoke<StatsType>('get_stats');
        setStats(data);
      } catch (e) {
        console.error(e);
      } finally {
        setLoading(false);
      }
    };
    fetch();
  }, []);

  if (loading) {
    return <div className="stats-page"><p>加载中...</p></div>;
  }

  if (!stats) {
    return <div className="stats-page"><p>无法获取统计</p></div>;
  }

  const maxTypeCount = Math.max(...stats.by_type.map((t) => t.count), 1);
  const maxStatusCount = Math.max(...stats.by_status.map((s) => s.count), 1);

  const completedCount = stats.by_status.find((s) => s.status === '已完成')?.count ?? 0;
  const completionRate = stats.total > 0 ? Math.round((completedCount / stats.total) * 100) : 0;

  return (
    <div className="stats-page">
      <div className="stats-grid">
        <div className="stats-card">
          <div className="count">{stats.total}</div>
          <div className="label">总条目</div>
        </div>

        <div className="stats-card">
          <div className="count">{completedCount}</div>
          <div className="label">已完成</div>
        </div>

        <div className="stats-card">
          <div className="count">{completionRate}%</div>
          <div className="label">完成率</div>
        </div>

        <div className="stats-card">
          <div className="count">{stats.by_type.length}</div>
          <div className="label">类型数</div>
        </div>
      </div>

      <div className="stats-section">
        <h3>📊 类型分布</h3>
        <div className="stats-bar-list">
          {stats.by_type.map((t) => {
            const info = MEDIA_TYPES[t.media_type];
            const color = info?.color ?? '#888';
            const pct = Math.round((t.count / maxTypeCount) * 100);
            return (
              <div key={t.media_type} className="stats-bar-item">
                <span className="label">{info?.icon ?? '📄'} {t.media_type}</span>
                <div className="stats-bar-track">
                  <div className="stats-bar-fill" style={{ width: `${pct}%`, background: color }}>
                    {t.count}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <div className="stats-section">
        <h3>📈 状态分布</h3>
        <div className="stats-bar-list">
          {stats.by_status.map((s) => {
            const info = STATUS_STYLES[s.status];
            const color = info?.color ?? '#888';
            const pct = Math.round((s.count / maxStatusCount) * 100);
            return (
              <div key={s.status} className="stats-bar-item">
                <span className="label">{s.status}</span>
                <div className="stats-bar-track">
                  <div className="stats-bar-fill" style={{ width: `${pct}%`, background: color }}>
                    {s.count}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
