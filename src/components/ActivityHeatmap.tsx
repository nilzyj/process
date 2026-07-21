import { useMemo } from 'react';

interface Props {
  data: { date: string; count: number }[];
}

export default function ActivityHeatmap({ data }: Props) {
  const countMap = useMemo(() => {
    const m = new Map<string, number>();
    for (const d of data) m.set(d.date, d.count);
    return m;
  }, [data]);

  const { weeks, maxCount } = useMemo(() => {
    const today = new Date();
    const start = new Date(today);
    start.setDate(start.getDate() - 364);
    const day = start.getDay();
    start.setDate(start.getDate() - (day === 0 ? 6 : day - 1));

    const w: { date: string; count: number }[][] = [];
    const cur = new Date(start);
    let mx = 0;
    while (cur <= today) {
      const week: { date: string; count: number }[] = [];
      for (let d = 0; d < 7; d++) {
        const y = cur.getFullYear();
        const m = String(cur.getMonth() + 1).padStart(2, '0');
        const dd = String(cur.getDate()).padStart(2, '0');
        const ds = `${y}-${m}-${dd}`;
        const c = countMap.get(ds) || 0;
        week.push({ date: ds, count: c });
        if (c > mx) mx = c;
        cur.setDate(cur.getDate() + 1);
      }
      w.push(week);
    }
    return { weeks: w, maxCount: mx };
  }, [countMap]);

  const getColor = (count: number) => {
    if (maxCount === 0) return '#F5EBE0';
    const r = count / maxCount;
    if (r === 0) return '#F5EBE0';
    if (r <= 0.25) return '#FFE0B2';
    if (r <= 0.5) return '#FFCC80';
    if (r <= 0.75) return '#FFAB91';
    return '#F48FB1';
  };

  const months = useMemo(() => {
    const seen = new Set<number>();
    const labels: { label: string; col: number }[] = [];
    weeks.forEach((week, wi) => {
      const m = new Date(week[3]?.date ?? week[0].date).getMonth();
      if (!seen.has(m)) {
        seen.add(m);
        labels.push({ label: ['1月','2月','3月','4月','5月','6月','7月','8月','9月','10月','11月','12月'][m], col: wi });
      }
    });
    return labels;
  }, [weeks]);

  if (!data.length) return null;

  return (
    <div className="heatmap">
      <div className="heatmap-months">
        <span className="hm-spacer" />
        {months.map((m, i) => (
          <span
            key={m.label}
            className="hm-month-label"
            style={{ marginLeft: i === 0 ? 0 : (m.col - months[i - 1].col) * 14 }}
          >
            {m.label}
          </span>
        ))}
      </div>
      <div className="heatmap-body">
        <div className="hm-day-labels">
          {['Mon','','Wed','','Fri','',''].map((l, i) => (
            <span key={i} className="hm-day-label">{l}</span>
          ))}
        </div>
        <div className="hm-grid">
          {weeks.map((week, wi) => (
            <div key={wi} className="hm-col">
              {week.map((day, di) => (
                <div
                  key={di}
                  className="hm-cell"
                  style={{ backgroundColor: getColor(day.count) }}
                  title={`${day.date}: ${day.count} 条`}
                />
              ))}
            </div>
          ))}
        </div>
      </div>
      <div className="heatmap-legend">
        <span>少</span>
        <div className="hm-legend-cell" style={{ background: '#161b22' }} />
        <div className="hm-legend-cell" style={{ background: '#0e4429' }} />
        <div className="hm-legend-cell" style={{ background: '#006d32' }} />
        <div className="hm-legend-cell" style={{ background: '#26a641' }} />
        <div className="hm-legend-cell" style={{ background: '#39d353' }} />
        <span>多</span>
      </div>
    </div>
  );
}
