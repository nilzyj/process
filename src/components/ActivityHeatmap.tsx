import { useMemo } from 'react';

interface Props {
  data: { date: string; count: number }[];
}

// GitHub 风格绿色梯度。0 用 CSS 变量跟随底色，避免热力图出现一块异色底板。
const HEAT_COLORS = ['#0e4429', '#006d32', '#26a641', '#39d353'];

// 必须与 App.css 里 .hm-cell 的 width + .hm-grid 的 gap 保持一致，
// 否则月份标签的水平偏移会和实际格子列错位。
const HM_STEP = 17;

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
    if (maxCount === 0 || count === 0) return 'var(--heat-empty)';
    const r = count / maxCount;
    if (r <= 0.25) return HEAT_COLORS[0];
    if (r <= 0.5) return HEAT_COLORS[1];
    if (r <= 0.75) return HEAT_COLORS[2];
    return HEAT_COLORS[3];
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
        {months.map((m, i) => (
          <span
            key={m.label}
            className="hm-month-label"
            style={{ marginLeft: i === 0 ? 0 : (m.col - months[i - 1].col) * HM_STEP }}
          >
            {m.label}
          </span>
        ))}
      </div>
      <div className="heatmap-body">
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
        <div className="hm-legend-cell" style={{ background: 'var(--heat-empty)' }} />
        {HEAT_COLORS.map((c) => (
          <div key={c} className="hm-legend-cell" style={{ background: c }} />
        ))}
        <span>多</span>
      </div>
    </div>
  );
}
