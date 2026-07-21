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
  const { decades, allYears } = useMemo(() => {
    const sorted = [...stats.by_year].sort((a, b) => b.year - a.year);
    const map = new Map<number, { decade: number; total: number; years: typeof sorted }>();
    for (const y of sorted) {
      const d = Math.floor(y.year / 10) * 10;
      if (!map.has(d)) map.set(d, { decade: d, total: 0, years: [] });
      const entry = map.get(d)!;
      entry.total += y.count;
      entry.years.push(y);
    }
    return {
      decades: Array.from(map.values()).sort((a, b) => b.decade - a.decade),
      allYears: sorted,
    };
  }, [stats.by_year]);

  if (!allYears.length) return null;
  const maxTotal = Math.max(...decades.map((d) => d.total), 1);
  const colors = ['#f97316','#a855f7','#06b6d4','#22c55e','#ef4444','#eab308','#ec4899','#6366f1','#14b8a6','#f43f5e'];
  return (
    <div className="stats-section">
      <h3>发行年份</h3>
      <div className="stats-bar-list">
        {decades.map((d, i) => {
          const isOpen = expanded === String(d.decade);
          const pct = Math.round((d.total / maxTotal) * 100);
          const color = colors[i % colors.length];
          return (
            <div key={d.decade}>
              <div className="stats-bar-item clickable" onClick={() => setExpanded(isOpen ? null : String(d.decade))}>
                <span className="label">
                  <span className="yd-arrow">{isOpen ? '▼' : '▶'}</span>
                  {d.decade}年代
                </span>
                <div className="stats-bar-track">
                  <div className="stats-bar-fill" style={{ width: `${pct}%`, background: color }}>
                    {d.total}
                  </div>
                </div>
              </div>
              {isOpen && (
                <div className="yd-children">
                  {d.years.map((yr) => {
                    const childPct = Math.round((yr.count / d.total) * 100);
                    return (
                      <div key={yr.year} className="stats-bar-item yd-child">
                        <span className="label">{yr.year}年</span>
                        <div className="stats-bar-track">
                          <div className="stats-bar-fill" style={{ width: `${childPct}%`, background: color, opacity: 0.7 }}>
                            {yr.count}
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

function isSeriesTag(tag: string): boolean {
  return tag.includes('系列') || tag.includes('宇宙') || tag.includes('传奇') || tag.includes('纪');
}

function isStudioTag(tag: string): boolean {
  const known = new Set(['皮克斯', '梦工厂', '迪士尼', '京都动画', '动画工房', '皮克斯工作室', '蓝天工作室', '迪士尼动画', '索尼动画', '周刊少年Sunday', '少年Jump+']);
  if (known.has(tag)) return true;
  if (/STUDIO|STAFF|FOX|PICTURES|MADHOUSE|CloverWorks|POLYGON|SHAFT|ufotable|Lerche|CITRUS|Production|Sunday|少年Jump/i.test(tag)) return true;
  return false;
}

function isPersonTag(tag: string): boolean {
  if (/电影|动画|动漫|书籍|漫画|纪录片|电视剧|播客|TED|第\d+[季部]/.test(tag)) return false;
  if (tag.includes('·') || tag.includes('・')) return true;
  const cjk = tag.match(/[\u4e00-\u9fff\uf900-\ufaff]/g);
  if (cjk && cjk.length >= 2 && cjk.length === [...tag].filter(c => !/[\s]/.test(c)).length) return true;
  if (/^[a-zA-Z\u00c0-\u024f][a-zA-Z\u00c0-\u024f\s.'-]+$/.test(tag)) return true;
  return false;
}

const KNOWN_DIRECTORS = new Set([
  '诺兰', '詹姆斯·卡梅隆', '克里斯托弗·诺兰', '宫崎骏', '新海诚',
  '姜文', '王家卫', '张艺谋', '陈凯歌', '贾樟柯', '李安', '侯孝贤',
  '周星驰', '徐克', '吴京', '陈思诚', '徐峥', '宁浩', '韩寒', '郭敬明',
  '昆汀·塔伦蒂诺', '史蒂文·斯皮尔伯格', '大卫·芬奇',
  '蒂姆·伯顿', '吕克·贝松', '诺兰', '詹姆斯·卡梅隆',
  '是枝裕和', '黑泽明', '北野武',
]);

function TagDist({ stats }: { stats: StatsType }) {
  const [popover, setPopover] = useState<{ tag: string; color: string; left: number; top: number } | null>(null);
  const [records, setRecords] = useState<MediaRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const refMap = useMemo(() => new Map<string, HTMLElement>(), []);

  const { regularTags, authorTags, actorTags, directorTags, studioTags, seriesTags } = useMemo(() => {
    const typeMap = new Map<string, Map<string, number>>();
    for (const tt of stats.tag_types) {
      const tags = tt.tag.split(',').map((s) => s.trim()).filter(Boolean);
      for (const t of tags) {
        if (!t) continue;
        if (!typeMap.has(t)) typeMap.set(t, new Map());
        const m = typeMap.get(t)!;
        m.set(tt.media_type, (m.get(tt.media_type) || 0) + tt.count);
      }
    }

    const regular: { tag: string; count: number }[] = [];
    const author: { tag: string; count: number }[] = [];
    const actor: { tag: string; count: number }[] = [];
    const director: { tag: string; count: number }[] = [];
    const studio: { tag: string; count: number }[] = [];
    const series: { tag: string; count: number }[] = [];
    for (const t of stats.by_tags) {
      if (isSeriesTag(t.tag)) { series.push(t); continue; }
      if (isStudioTag(t.tag)) { studio.push(t); continue; }
      if (!isPersonTag(t.tag)) { regular.push(t); continue; }
      const typeDist = typeMap.get(t.tag);
      let isAuthor = false;
      if (typeDist) {
        let maxType = ''; let maxCount = 0;
        for (const [type, cnt] of typeDist) {
          if (cnt > maxCount) { maxCount = cnt; maxType = type; }
        }
        if (maxType === '书籍' || maxType === 'TED') isAuthor = true;
      }
      if (isAuthor) author.push(t);
      else if (KNOWN_DIRECTORS.has(t.tag)) director.push(t);
      else actor.push(t);
    }
    return { regularTags: regular, authorTags: author, actorTags: actor, directorTags: director, studioTags: studio, seriesTags: series };
  }, [stats.by_tags, stats.tag_types]);

  useEffect(() => {
    if (!popover) return;
    const handler = (e: MouseEvent) => {
      const el = e.target as HTMLElement;
      if (!el.closest('.pill-popover') && !el.closest('.pill.clickable')) {
        setPopover(null);
        setRecords([]);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [popover]);

  if (!stats.by_tags.length) return null;
  const palette = ['#FF6B6B','#4ECDC4','#45B7D1','#96CEB4','#FFD93D','#DDA0DD','#F08A5D','#00ADB5','#FF2E63','#6C5B7B','#f97316','#a855f7','#22c55e'];

  const renderTag = (t: { tag: string; count: number }, i: number, maxCount: number, asList: boolean) => {
    if (asList) {
      return (
        <span
          key={t.tag}
          ref={(el) => { if (el) refMap.set(t.tag, el); }}
          className="cloud-tag clickable"
          style={{ color: palette[i % palette.length] }}
          onClick={async () => {
            if (popover?.tag === t.tag) { setPopover(null); setRecords([]); return; }
            const rect = refMap.get(t.tag)?.getBoundingClientRect();
            if (!rect) return;
            setPopover({ tag: t.tag, color: palette[i % palette.length], left: rect.left, top: rect.bottom + 4 });
            setLoading(true);
            try {
              const res = await invoke<{ records: MediaRecord[]; total: number }>('list_records', {
                filter: { tag: t.tag, page: 1, page_size: 200 },
              });
              setRecords(res.records);
            } catch { setRecords([]); } finally { setLoading(false); }
          }}
        >
          {t.tag}
          <sup className="cloud-count">{t.count}</sup>
        </span>
      );
    }
    const ratio = t.count / maxCount;
    const size = Math.round(13 + ratio * 21);
    const weight = ratio > 0.5 ? 800 : ratio > 0.25 ? 700 : 600;
    return (
      <span
        key={t.tag}
        ref={(el) => { if (el) refMap.set(t.tag, el); }}
        className="cloud-tag clickable"
        style={{ fontSize: size, fontWeight: weight, color: palette[i % palette.length] }}
        onClick={async () => {
          if (popover?.tag === t.tag) { setPopover(null); setRecords([]); return; }
          const rect = refMap.get(t.tag)?.getBoundingClientRect();
          if (!rect) return;
          setPopover({ tag: t.tag, color: palette[i % palette.length], left: rect.left, top: rect.bottom + 4 });
          setLoading(true);
          try {
            const res = await invoke<{ records: MediaRecord[]; total: number }>('list_records', {
              filter: { tag: t.tag, page: 1, page_size: 200 },
            });
            setRecords(res.records);
          } catch { setRecords([]); } finally { setLoading(false); }
        }}
      >
        {t.tag}
        <sup className="cloud-count">{t.count}</sup>
      </span>
    );
  };

  return (
    <div className="stats-section">
      <h3>标签分布</h3>
      {regularTags.length > 0 && (
        <>
          <div className="tag-section-label">普通标签</div>
          <div className="cloud-grid">
            {regularTags.map((t, i) => renderTag(t, i, Math.max(...regularTags.map((x) => x.count), 1), false))}
          </div>
        </>
      )}
      {authorTags.length > 0 && (
        <>
          <div className="tag-section-label">作者</div>
          <div className="cloud-grid series-tag-list">
            {authorTags.map((t, i) => renderTag(t, i, 1, true))}
          </div>
        </>
      )}
      {directorTags.length > 0 && (
        <>
          <div className="tag-section-label">导演</div>
          <div className="cloud-grid series-tag-list">
            {directorTags.map((t, i) => renderTag(t, i, 1, true))}
          </div>
        </>
      )}
      {actorTags.length > 0 && (
        <>
          <div className="tag-section-label">演员</div>
          <div className="cloud-grid series-tag-list">
            {actorTags.map((t, i) => renderTag(t, i, 1, true))}
          </div>
        </>
      )}
      {studioTags.length > 0 && (
        <>
          <div className="tag-section-label">动画工作室</div>
          <div className="cloud-grid series-tag-list">
            {studioTags.map((t, i) => renderTag(t, i, 1, true))}
          </div>
        </>
      )}
      {seriesTags.length > 0 && (
        <>
          <div className="tag-section-label">系列标签</div>
          <div className="cloud-grid series-tag-list">
            {seriesTags.map((t, i) => renderTag(t, i, 1, true))}
          </div>
        </>
      )}
      {popover && (
        <div
          className="pill-popover"
          style={{ left: popover.left, top: popover.top }}
        >
          <div className="pill-popover-arrow" style={{ borderBottomColor: popover.color }} />
          {loading ? (
            <div className="series-loading">加载中...</div>
          ) : records.length === 0 ? (
            <div className="series-loading">暂无记录</div>
          ) : (
            <div className="pill-popover-list">
              {records.map((r) => (
                <div key={r.id} className="series-record-item" style={{ borderLeftColor: popover.color }}>
                  <span className="sr-name">{r.record_name}</span>
                  <span className="sr-meta">
                    {r.media_type && <span className="sr-type">{r.media_type}</span>}
                    {r.status && <span className="sr-status">{r.status}</span>}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
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


