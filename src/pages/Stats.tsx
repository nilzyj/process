import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Stats as StatsType } from '../types';
import { MEDIA_TYPES, STATUS_STYLES } from '../types';
import ActivityHeatmap from '../components/ActivityHeatmap';

export default function Stats() {
  const [stats, setStats] = useState<StatsType | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    const fetch = async () => {
      try {
        const data = await invoke<StatsType>('get_stats');
        setStats(data);
      } catch (e) {
        setError(String(e));
        console.error(e);
      } finally {
        setLoading(false);
      }
    };
    fetch();
  }, []);

  if (loading) return <div className="stats-page"><div className="loading-spinner" /></div>;
  if (error) return <div className="stats-page"><p className="stats-error">{error}</p></div>;
  if (!stats) return <div className="stats-page"><p className="stats-error">无法获取统计</p></div>;

  return (
    <div className="stats-page">
      <SummaryCards stats={stats} />
      <CompletionRates stats={stats} />
      <div className="stats-row">
        <TypeStatusCross stats={stats} />
        <AnnualTrends stats={stats} />
      </div>
      <MonthlyTimeline data={stats.daily_activity} />
      <CountryDist stats={stats} />
      <TagDist stats={stats} />
      <ProgressBuckets stats={stats} />
      <RecentActivity stats={stats} />
      <div className="stats-section">
        <h3>活动热力图</h3>
        <ActivityHeatmap data={stats.daily_activity} />
      </div>
    </div>
  );
}

function SummaryCards({ stats }: { stats: StatsType }) {
  const completedCount = stats.by_status.find((s) => s.status === '已完成')?.count ?? 0;
  const completionRate = stats.total > 0 ? Math.round((completedCount / stats.total) * 100) : 0;
  const r = stats.recent;
  return (
    <div className="stats-grid">
      <div className="stats-card"><div className="count">{stats.total}</div><div className="label">总条目</div></div>
      <div className="stats-card"><div className="count">{completedCount}</div><div className="label">已完成</div></div>
      <div className="stats-card"><div className="count">{completionRate}%</div><div className="label">完成率</div></div>
      <div className="stats-card"><div className="count">{stats.by_type.length}</div><div className="label">类型数</div></div>
      <div className="stats-card"><div className="count today">{r.new_today}</div><div className="label">今日新增</div></div>
      <div className="stats-card"><div className="count">{r.new_week}</div><div className="label">本周新增</div></div>
      <div className="stats-card"><div className="count today">{r.completed_today}</div><div className="label">今日完成</div></div>
      <div className="stats-card"><div className="count">{r.completed_week}</div><div className="label">本周完成</div></div>
    </div>
  );
}

function CompletionRates({ stats }: { stats: StatsType }) {
  const maxTotal = Math.max(...stats.completion_rates.map((r) => r.total), 1);
  return (
    <div className="stats-section">
      <h3>完成率按类型</h3>
      <div className="stats-bar-list">
        {stats.completion_rates.map((r) => {
          const info = MEDIA_TYPES[r.media_type];
          const pct = r.total > 0 ? Math.round((r.completed / r.total) * 100) : 0;
          const barW = Math.round((r.total / maxTotal) * 100);
          return (
            <div key={r.media_type} className="stats-bar-item">
              <span className="label">{info?.icon ?? '📄'} {r.media_type}</span>
              <div className="stats-bar-track">
                <div
                  className="stats-bar-fill completion-fill"
                  style={{ width: `${barW}%`, background: pct >= 100 ? '#22c55e' : pct > 0 ? '#22d3ee' : '#555' }}
                >
                  <span className="cr-text">{r.completed}/{r.total} ({pct}%)</span>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function TypeStatusCross({ stats }: { stats: StatsType }) {
  const typeOrder = useMemo(() => {
    const seen = new Set<string>();
    const order: string[] = [];
    for (const ts of stats.type_status) {
      if (!seen.has(ts.media_type)) { seen.add(ts.media_type); order.push(ts.media_type); }
    }
    return order;
  }, [stats.type_status]);

  const statusOrder = ['待办', '进行中', '已完成'];

  return (
    <div className="stats-section stats-section-half">
      <h3>类型 × 状态</h3>
      <div className="ts-cross-list">
        {typeOrder.map((type) => {
          const rows = stats.type_status.filter((t) => t.media_type === type);
          const total = rows.reduce((s, r) => s + r.count, 0);
          if (total === 0) return null;
          const info = MEDIA_TYPES[type];
          return (
            <div key={type} className="ts-cross-item">
              <span className="label">{info?.icon ?? '📄'} {type}</span>
              <div className="ts-cross-bar">
                {statusOrder.map((st) => {
                  const row = rows.find((r) => r.status === st);
                  const cnt = row?.count ?? 0;
                  const pct = (cnt / total) * 100;
                  if (cnt === 0) return null;
                  const style = STATUS_STYLES[st];
                  return (
                    <div
                      key={st}
                      className="ts-cross-seg"
                      style={{ width: `${pct}%`, background: style?.color ?? '#555' }}
                      title={`${st}: ${cnt}`}
                    />
                  );
                })}
              </div>
              <span className="ts-cross-total">{total}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function AnnualTrends({ stats }: { stats: StatsType }) {
  const maxCount = Math.max(...stats.by_year.map((y) => y.count), 1);
  return (
    <div className="stats-section stats-section-half">
      <h3>年度趋势</h3>
      <div className="stats-bar-list">
        {stats.by_year.map((y) => {
          const pct = Math.round((y.count / maxCount) * 100);
          return (
            <div key={y.year} className="stats-bar-item">
              <span className="label">{y.year}</span>
              <div className="stats-bar-track">
                <div className="stats-bar-fill" style={{ width: `${pct}%`, background: '#f97316' }}>
                  {y.count}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function MonthlyTimeline({ data }: { data: { date: string; count: number }[] }) {
  const monthly = useMemo(() => {
    const map = new Map<string, number>();
    for (const d of data) {
      const m = d.date.slice(0, 7);
      map.set(m, (map.get(m) || 0) + d.count);
    }
    return Array.from(map.entries())
      .map(([month, count]) => ({ month, count }))
      .sort((a, b) => a.month.localeCompare(b.month))
      .slice(-24);
  }, [data]);

  const maxCount = Math.max(...monthly.map((m) => m.count), 1);
  return (
    <div className="stats-section">
      <h3>时间线（月度）</h3>
      <div className="monthly-grid">
        {monthly.map((m) => {
          const pct = Math.round((m.count / maxCount) * 100);
          const [, mo] = m.month.split('-');
          return (
            <div key={m.month} className="monthly-item">
              <span className="monthly-val">{m.count}</span>
              <div className="monthly-bar-wrap">
                <div className="monthly-bar" style={{ height: `${Math.max(pct, 3)}%` }} />
              </div>
              <span className="monthly-label">{mo}月</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function CountryDist({ stats }: { stats: StatsType }) {
  const maxCount = Math.max(...stats.by_country.map((c) => c.count), 1);
  return (
    <div className="stats-section stats-section-half">
      <h3>国家分布</h3>
      <div className="stats-bar-list">
        {stats.by_country.map((c) => {
          const pct = Math.round((c.count / maxCount) * 100);
          return (
            <div key={c.country} className="stats-bar-item">
              <span className="label">{c.country}</span>
              <div className="stats-bar-track">
                <div className="stats-bar-fill" style={{ width: `${pct}%`, background: '#a855f7' }}>
                  {c.count}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function TagDist({ stats }: { stats: StatsType }) {
  if (!stats.by_tags.length) return null;
  const maxCount = Math.max(...stats.by_tags.map((t) => t.count), 1);
  const palette = ['#FF6B6B','#4ECDC4','#45B7D1','#96CEB4','#FFD93D','#DDA0DD','#6C5B7B','#F08A5D','#B83B5E','#08D9D6','#FF2E63','#00ADB5'];
  return (
    <div className="stats-section">
      <h3>标签分布</h3>
      <div className="tag-bubbles">
        {stats.by_tags.map((t, i) => {
          const size = 52 + Math.round((t.count / maxCount) * 64);
          const color = palette[i % palette.length];
          const fs = size < 64 ? 10 : size < 86 ? 11 : 12;
          return (
            <div key={t.tag} className="tag-bubble" style={{ width: size, height: size, borderColor: `${color}88`, background: `${color}18` }}>
              <span className="tb-name" style={{ fontSize: fs, color }}>{t.tag}</span>
              <span className="tb-count" style={{ color: `${color}cc` }}>{t.count}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ProgressBuckets({ stats }: { stats: StatsType }) {
  const maxCount = Math.max(...stats.progress_buckets.map((b) => b.count), 1);
  const colors = ['#ef4444', '#f97316', '#eab308', '#22d3ee', '#22c55e'];
  return (
    <div className="stats-section">
      <h3>进度分布</h3>
      <div className="stats-bar-list">
        {stats.progress_buckets.map((b, i) => {
          const pct = Math.round((b.count / maxCount) * 100);
          return (
            <div key={b.label} className="stats-bar-item">
              <span className="label">{b.label}</span>
              <div className="stats-bar-track">
                <div className="stats-bar-fill" style={{ width: `${pct}%`, background: colors[i] }}>
                  {b.count}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function RecentActivity({ stats }: { stats: StatsType }) {
  const r = stats.recent;
  return (
    <div className="stats-section">
      <h3>近期动态</h3>
      <div className="stats-grid">
        <div className="stats-card"><div className="count">{r.new_month}</div><div className="label">近30天新增</div></div>
        <div className="stats-card"><div className="count">{r.completed_month}</div><div className="label">近30天完成</div></div>
      </div>
    </div>
  );
}
