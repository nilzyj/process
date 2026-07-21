import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Stats as StatsType } from '../types';
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
      <div className="stats-row">
        <div className="stats-section" style={{ flex: 2 }}>
          <h3>活动热力图</h3>
          <ActivityHeatmap data={stats.daily_activity} />
        </div>
        <div style={{ flex: 1 }}>
          <YearDist stats={stats} />
        </div>
      </div>
      <MonthlyTimeline stats={stats} />
      <CountryDist stats={stats} />
      <TagDist stats={stats} />
      <ProgressBuckets stats={stats} />
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



function YearDist({ stats }: { stats: StatsType }) {
  const [expanded, setExpanded] = useState<string | null>(null);

  const { decades, isGrouped } = useMemo(() => {
    if (stats.by_year.length <= 3) {
      return { decades: stats.by_year.map((y) => ({ label: `${y.year}年`, count: y.count, decade: 0 })), isGrouped: false };
    }
    const map = new Map<number, number>();
    for (const y of stats.by_year) {
      const d = Math.floor(y.year / 10) * 10;
      map.set(d, (map.get(d) || 0) + y.count);
    }
    return {
      decades: Array.from(map.entries()).map(([decade, count]) => ({ label: `${decade}年代`, count, decade })).sort((a, b) => a.label.localeCompare(b.label)),
      isGrouped: true,
    };
  }, [stats.by_year]);

  if (!decades.length) return null;
  const maxCount = Math.max(...decades.map((d) => d.count), 1);

  return (
    <div className="stats-section">
      <h3>发行年份</h3>
      <div className="stats-bar-list">
        {decades.map((d) => {
          const isOpen = expanded === d.label;
          const children = isGrouped && isOpen
            ? stats.by_year.filter((y) => Math.floor(y.year / 10) * 10 === d.decade)
            : [];
          return (
            <div key={d.label}>
              <div
                className={`stats-bar-item${isGrouped ? ' clickable' : ''}`}
                onClick={() => isGrouped && setExpanded(isOpen ? null : d.label)}
              >
                <span className="label">
                  {isGrouped && <span className="yd-arrow">{isOpen ? '▼' : '▶'}</span>}
                  {d.label}
                </span>
                <div className="stats-bar-track">
                  <div className="stats-bar-fill" style={{ width: `${Math.round((d.count / maxCount) * 100)}%`, background: 'linear-gradient(90deg, #f97316, #fb923c)' }}>
                    {d.count}
                  </div>
                </div>
              </div>
              {children.length > 0 && (
                <div className="yd-children">
                  {children.map((y) => {
                    const childPct = Math.round((y.count / d.count) * 100);
                    return (
                      <div key={y.year} className="stats-bar-item yd-child">
                        <span className="label">{y.year}年</span>
                        <div className="stats-bar-track">
                          <div className="stats-bar-fill" style={{ width: `${childPct}%`, background: '#fdba74' }}>
                            {y.count}
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function MonthlyTimeline({ stats }: { stats: StatsType }) {
  const monthly = useMemo(() => {
    return stats.monthly_end
      .sort((a, b) => a.month.localeCompare(b.month))
      .slice(-60);
  }, [stats.monthly_end]);

  const maxCount = Math.max(...monthly.map((m) => m.count), 1);
  return (
    <div className="stats-section">
      <h3>完结时间线</h3>
      <div className="monthly-grid">
        {monthly.map((m) => {
          const pct = Math.round((m.count / maxCount) * 100);
          const [y, mo] = m.month.split('-');
          const shortYear = y.slice(2);
          return (
            <div key={m.month} className="monthly-item">
              <span className="monthly-val">{m.count}</span>
              <div className="monthly-bar-wrap">
                <div className="monthly-bar" style={{ height: `${Math.max(pct, 3)}%` }} />
              </div>
              <span className="monthly-label">{shortYear}.{mo}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function CountryDist({ stats }: { stats: StatsType }) {
  const flags: Record<string, string> = {
    '日本':'🇯🇵','美国':'🇺🇸','中国':'🇨🇳','韩国':'🇰🇷','英国':'🇬🇧',
    '法国':'🇫🇷','德国':'🇩🇪','意大利':'🇮🇹','西班牙':'🇪🇸','俄罗斯':'🇷🇺',
    '加拿大':'🇨🇦','澳大利亚':'🇦🇺','印度':'🇮🇳','泰国':'🇹🇭','台湾':'🇹🇼',
    '香港':'🇭🇰','巴西':'🇧🇷','墨西哥':'🇲🇽','荷兰':'🇳🇱','瑞典':'🇸🇪',
  };
  const palette = ['#FF6B6B','#4ECDC4','#45B7D1','#96CEB4','#FFD93D','#DDA0DD','#F08A5D','#00ADB5'];
  if (!stats.by_country.length) return null;
  return (
    <div className="stats-section">
      <h3>国家分布</h3>
      <div className="country-grid">
        {stats.by_country.map((c, i) => {
          const color = palette[i % palette.length];
          return (
            <div key={c.country} className="country-card" style={{ borderColor: `${color}33`, background: `${color}0a` }}>
              <span className="cc-flag">{flags[c.country] || '🌍'}</span>
              <span className="cc-name">{c.country}</span>
              <span className="cc-count" style={{ color }}>{c.count}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function TagDist({ stats }: { stats: StatsType }) {
  if (!stats.by_tags.length) return null;
  const palette = ['#FF6B6B','#4ECDC4','#45B7D1','#96CEB4','#FFD93D','#DDA0DD','#F08A5D','#00ADB5','#FF2E63','#6C5B7B'];
  return (
    <div className="stats-section">
      <h3>标签分布</h3>
      <div className="pill-grid">
        {stats.by_tags.map((t, i) => {
          const color = palette[i % palette.length];
          return (
            <div key={t.tag} className="pill" style={{ borderColor: `${color}44`, background: `${color}14` }}>
              <span className="pill-name" style={{ color }}>{t.tag}</span>
              <span className="pill-count" style={{ background: `${color}22`, color: `${color}cc` }}>{t.count}</span>
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


