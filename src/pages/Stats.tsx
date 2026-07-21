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
      <div className="stats-row">
        <CompletionRates stats={stats} />
        <AnnualTrends stats={stats} />
      </div>
      <TypeStatusCross stats={stats} />
      <MonthlyTimeline data={stats.daily_activity} />
      <div className="stats-row">
        <CountryDist stats={stats} />
        <TagDist stats={stats} />
      </div>
      <ProgressBuckets stats={stats} />
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
    <div className="summary-grid">
      <div className="summary-card total">
        <div className="sc-value">{stats.total}</div>
        <div className="sc-label">总条目</div>
      </div>
      <div className="summary-card done">
        <div className="sc-value">{completedCount}</div>
        <div className="sc-label">已完成</div>
      </div>
      <div className="summary-card rate">
        <div className="sc-value">{completionRate}%</div>
        <div className="sc-label">完成率</div>
      </div>
      <div className="summary-card types">
        <div className="sc-value">{stats.by_type.length}</div>
        <div className="sc-label">类型数</div>
      </div>
      <div className="summary-card accent">
        <div className="sc-value">{r.new_today}</div>
        <div className="sc-label">今日新增</div>
      </div>
      <div className="summary-card">
        <div className="sc-value">{r.new_week}</div>
        <div className="sc-label">本周新增</div>
      </div>
      <div className="summary-card accent">
        <div className="sc-value">{r.completed_today}</div>
        <div className="sc-label">今日完成</div>
      </div>
      <div className="summary-card">
        <div className="sc-value">{r.completed_week}</div>
        <div className="sc-label">本周完成</div>
      </div>
    </div>
  );
}

function CompletionRates({ stats }: { stats: StatsType }) {
  const maxTotal = Math.max(...stats.completion_rates.map((r) => r.total), 1);
  return (
    <div className="section">
      <h3>完成率按类型</h3>
      <div className="bar-list">
        {stats.completion_rates.map((r) => {
          const info = MEDIA_TYPES[r.media_type];
          const pct = r.total > 0 ? Math.round((r.completed / r.total) * 100) : 0;
          const barW = Math.round((r.total / maxTotal) * 100);
          const color = pct >= 100 ? '#A5D6A7' : pct >= 50 ? '#81D4FA' : '#FFE082';
          return (
            <div key={r.media_type} className="bar-item">
              <span className="bi-label">{info?.icon ?? '📄'} {r.media_type}</span>
              <div className="bar-track">
                <div className="bar-fill" style={{ width: `${barW}%`, background: color }}>
                  <span className="bf-text">{r.completed}/{r.total} ({pct}%)</span>
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
    for (const ts of stats.type_status) {
      if (!seen.has(ts.media_type)) { seen.add(ts.media_type); }
    }
    return Array.from(seen);
  }, [stats.type_status]);
  const statusOrder = ['待办', '进行中', '已完成'];

  return (
    <div className="section">
      <h3>类型 × 状态</h3>
      <div className="cross-list">
        {typeOrder.map((type) => {
          const rows = stats.type_status.filter((t) => t.media_type === type);
          const total = rows.reduce((s, r) => s + r.count, 0);
          if (total === 0) return null;
          const info = MEDIA_TYPES[type];
          return (
            <div key={type} className="cross-item">
              <span className="ci-label">{info?.icon ?? '📄'} {type}</span>
              <div className="cross-bar">
                {statusOrder.map((st) => {
                  const row = rows.find((r) => r.status === st);
                  const cnt = row?.count ?? 0;
                  if (cnt === 0) return null;
                  const pct = (cnt / total) * 100;
                  const style = STATUS_STYLES[st];
                  return (
                    <div key={st} className="cross-seg" style={{ width: `${pct}%`, background: style?.color ?? '#555' }} title={`${st}: ${cnt}`} />
                  );
                })}
              </div>
              <span className="ci-count">{total}</span>
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
    <div className="section">
      <h3>年度趋势</h3>
      <div className="bar-list">
        {stats.by_year.map((y) => (
          <div key={y.year} className="bar-item">
            <span className="bi-label">{y.year}</span>
            <div className="bar-track">
              <div className="bar-fill" style={{ width: `${Math.round((y.count / maxCount) * 100)}%`, background: 'linear-gradient(90deg, #FFCC80, #FFAB91)' }}>
                {y.count}
              </div>
            </div>
          </div>
        ))}
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
  if (!monthly.length) return null;

  return (
    <div className="section">
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
    <div className="section">
      <h3>国家分布</h3>
      <div className="bar-list">
        {stats.by_country.map((c) => (
          <div key={c.country} className="bar-item">
            <span className="bi-label">{c.country}</span>
            <div className="bar-track">
              <div className="bar-fill" style={{ width: `${Math.round((c.count / maxCount) * 100)}%`, background: 'linear-gradient(90deg, #CE93D8, #BA68C8)' }}>
                {c.count}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function TagDist({ stats }: { stats: StatsType }) {
  if (!stats.by_tags.length) return null;
  const maxCount = Math.max(...stats.by_tags.map((t) => t.count), 1);
  return (
    <div className="section">
      <h3>标签分布</h3>
      <div className="bar-list">
        {stats.by_tags.map((t) => (
          <div key={t.tag} className="bar-item">
            <span className="bi-label">{t.tag}</span>
            <div className="bar-track">
              <div className="bar-fill" style={{ width: `${Math.round((t.count / maxCount) * 100)}%`, background: 'linear-gradient(90deg, #F48FB1, #F06292)' }}>
                {t.count}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function ProgressBuckets({ stats }: { stats: StatsType }) {
  const maxCount = Math.max(...stats.progress_buckets.map((b) => b.count), 1);
  const colors = ['#EF9A9A', '#FFCC80', '#FFE082', '#81D4FA', '#A5D6A7'];
  return (
    <div className="section">
      <h3>进度分布</h3>
      <div className="bar-list">
        {stats.progress_buckets.map((b, i) => (
          <div key={b.label} className="bar-item">
            <span className="bi-label">{b.label}</span>
            <div className="bar-track">
              <div className="bar-fill" style={{ width: `${Math.round((b.count / maxCount) * 100)}%`, background: colors[i] }}>
                {b.count}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
