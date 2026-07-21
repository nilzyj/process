import { useState, useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Stats as StatsType, MediaRecord } from '../types';
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
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <div className="stats-section" style={{ paddingRight: 0 }}>
            <h3>活动热力图</h3>
            <ActivityHeatmap data={stats.daily_activity} />
          </div>
          <CountryDist stats={stats} />
          <YearDist stats={stats} />
        </div>
      </div>
      <MonthlyTimeline stats={stats} />
      <TagDist stats={stats} />
      <SeriesDist stats={stats} />
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
  const years = [...stats.by_year].sort((a, b) => b.count - a.count);
  if (!years.length) return null;
  const maxCount = Math.max(...years.map((y) => y.count), 1);
  const colors = ['#f97316','#a855f7','#06b6d4','#22c55e','#ef4444','#eab308','#ec4899','#6366f1','#14b8a6','#f43f5e'];
  return (
    <div className="stats-section">
      <h3>发行年份</h3>
      <div className="stats-bar-list">
        {years.map((yr, i) => {
          const isOpen = expanded === String(yr.year);
          const pct = Math.round((yr.count / maxCount) * 100);
          const color = colors[i % colors.length];
          return (
            <div key={yr.year}>
              <div className="stats-bar-item clickable" onClick={() => setExpanded(isOpen ? null : String(yr.year))}>
                <span className="label">
                  <span className="yd-arrow">{isOpen ? '▼' : '▶'}</span>
                  {yr.year}年
                </span>
                <div className="stats-bar-track">
                  <div className="stats-bar-fill" style={{ width: `${pct}%`, background: color }}>
                    {yr.count}
                  </div>
                </div>
              </div>
              {isOpen && (
                <div className="mt-month-grid">
                  {Array.from({ length: 12 }, (_, idx) => (
                    <div key={`${yr.year}-${idx}`} className="mt-cell" style={{ background: `${color}15` }}>
                      <span className="mt-cell-month">{idx + 1}月</span>
                      <span className="mt-cell-count" style={{ color: `${color}bb` }}>-</span>
                    </div>
                  ))}
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
  const [expanded, setExpanded] = useState<string | null>(null);
  const [expandedMonth, setExpandedMonth] = useState<string | null>(null);
  const [monthRecords, setMonthRecords] = useState<MediaRecord[]>([]);
  const [monthLoading, setMonthLoading] = useState(false);
  const years = useMemo(() => {
    // Gather years from both monthly_end and by_year
    const yearSet = new Set<string>();
    for (const m of stats.monthly_end) {
      yearSet.add(m.month.split('-')[0]);
    }
    for (const y of stats.by_year) {
      yearSet.add(String(y.year));
    }
    if (yearSet.size === 0) return [];
    const monthMap = new Map<string, number>();
    for (const m of stats.monthly_end) {
      monthMap.set(m.month, m.count);
    }
    return Array.from(yearSet).sort().map((year) => {
      let total = 0;
      const months = Array.from({ length: 12 }, (_, i) => {
        const mk = `${year}-${String(i + 1).padStart(2, '0')}`;
        const count = monthMap.get(mk) ?? 0;
        total += count;
        return { month: mk, count };
      });
      return { year, total, months };
    }).filter((y) => y.total > 0);
  }, [stats]);
  if (!years.length) return null;
  const maxYearTotal = Math.max(...years.map((y) => y.total), 1);
  const colors = ['#22d3ee','#2dd4bf','#34d399','#4ade80','#a3e635','#facc15','#fb923c','#f87171','#c084fc','#818cf8'];

  return (
    <div className="stats-section">
      <h3>完结时间线</h3>
      <div className="stats-bar-list">
        {years.map((yr, i) => {
          const isOpen = expanded === yr.year;
          const pct = Math.round((yr.total / maxYearTotal) * 100);
          const color = colors[i % colors.length];
          return (
            <div key={yr.year}>
              <div className="stats-bar-item clickable" onClick={() => setExpanded(isOpen ? null : yr.year)}>
                <span className="label">
                  <span className="yd-arrow">{isOpen ? '▼' : '▶'}</span>
                  {yr.year}年
                </span>
                <div className="stats-bar-track">
                  <div className="stats-bar-fill" style={{ width: `${pct}%`, background: color }}>
                    {yr.total}
                  </div>
                </div>
              </div>
              {isOpen && (<>
                <div className="mt-month-grid">
                  {yr.months.map((m, idx) => {
                    const isMonthOpen = expandedMonth === m.month;
                    return (
                      <div
                        key={m.month}
                        className={`mt-cell${m.count > 0 ? ' clickable' : ''}`}
                        style={{ background: `${color}15` }}
                        onClick={async () => {
                          if (isMonthOpen) { setExpandedMonth(null); setMonthRecords([]); return; }
                          if (m.count === 0) return;
                          setExpandedMonth(m.month);
                          setMonthLoading(true);
                          try {
                            const y = parseInt(yr.year);
                            const nextMonth = idx === 11 ? `${y + 1}-01` : `${yr.year}-${String(idx + 2).padStart(2, '0')}`;
                            const res = await invoke<{ records: MediaRecord[]; total: number }>('list_records', {
                              filter: { end_time_start: m.month, end_time_end: nextMonth, page: 1, page_size: 200 },
                            });
                            setMonthRecords(res.records);
                          } catch { setMonthRecords([]); } finally { setMonthLoading(false); }
                        }}
                      >
                        <span className="mt-cell-month">{idx + 1}月</span>
                        <span className="mt-cell-count" style={{ color: m.count > 0 ? color : 'var(--text-muted)' }}>{m.count}</span>
                      </div>
                    );
                  })}
                </div>
                {expandedMonth && expandedMonth.startsWith(yr.year) && (
                  <div className="mt-month-records">
                    {monthLoading ? (
                      <span className="series-loading">加载中...</span>
                    ) : monthRecords.length === 0 ? (
                      <span className="series-loading">暂无记录</span>
                    ) : (
                      monthRecords.map((r) => (
                        <div key={r.id} className="series-record-item" style={{ borderLeftColor: color }}>
                          <span className="sr-name">{r.record_name}</span>
                          <span className="sr-meta">
                            {r.media_type && <span className="sr-type">{r.media_type}</span>}
                            {r.status && <span className="sr-status">{r.status}</span>}
                          </span>
                        </div>
                      ))
                    )}
                  </div>
                )}
              </>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function CountryDist({ stats }: { stats: StatsType }) {
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
              <span className="cc-name" style={{ color }}>{c.country}</span>
              <span className="cc-count" style={{ color: `${color}bb` }}>{c.count}</span>
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

function SeriesDist({ stats }: { stats: StatsType }) {
  const [expanded, setExpanded] = useState<string | null>(null);
  const [records, setRecords] = useState<MediaRecord[]>([]);
  const [loading, setLoading] = useState(false);

  const handleClick = async (tag: string) => {
    if (expanded === tag) {
      setExpanded(null);
      setRecords([]);
      return;
    }
    setExpanded(tag);
    setLoading(true);
    try {
      const res = await invoke<{ records: MediaRecord[]; total: number }>('list_records', {
        filter: { tag, page: 1, page_size: 200 },
      });
      setRecords(res.records);
    } catch {
      setRecords([]);
    } finally {
      setLoading(false);
    }
  };

  if (!stats.series_stats.length) return null;
  const palette = ['#f97316','#a855f7','#06b6d4','#22c55e','#ef4444','#eab308','#ec4899','#6366f1'];
  return (
    <div className="stats-section">
      <h3>系列统计</h3>
      <div className="series-grid">
        {stats.series_stats.map((s, i) => {
          const color = palette[i % palette.length];
          const rate = s.total > 0 ? Math.round((s.completed / s.total) * 100) : 0;
          const isOpen = expanded === s.tag;
          return (
            <div key={s.tag} className="series-card" style={{ borderColor: `${color}33`, background: `${color}08` }}>
              <div className="series-card-main" onClick={() => handleClick(s.tag)}>
                <div className="series-header" style={{ color }}>
                  <span className="series-name">{s.tag}</span>
                  <span className="series-total">{s.total}</span>
                </div>
                <div className="series-bar">
                  <div className="series-bar-fill" style={{ width: `${rate}%`, background: color }} />
                </div>
                <div className="series-meta">
                  <span className="series-completed" style={{ color: `${color}cc` }}>{s.completed} 已完成</span>
                  <span className="series-rate" style={{ color: `${color}99` }}>{rate}%</span>
                </div>
                <div className="series-types">
                  {s.by_type.map((t) => (
                    <span key={t.media_type} className="series-type-pill" style={{ borderColor: `${color}33`, color: `${color}bb` }}>
                      {t.media_type} {t.count}
                    </span>
                  ))}
                </div>
              </div>
              {isOpen && (
                <div className="series-records">
                  {loading ? (
                    <div className="series-loading">加载中...</div>
                  ) : records.length === 0 ? (
                    <div className="series-loading">暂无记录</div>
                  ) : (
                    records.map((r) => (
                      <div key={r.id} className="series-record-item" style={{ borderLeftColor: color }}>
                        <span className="sr-name">{r.record_name}</span>
                        <span className="sr-meta">
                          {r.media_type && <span className="sr-type">{r.media_type}</span>}
                          {r.status && <span className="sr-status">{r.status}</span>}
                          {r.country && <span className="sr-country">{r.country}</span>}
                        </span>
                      </div>
                    ))
                  )}
                </div>
              )}
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


