import { memo, useRef, useCallback, useEffect } from 'react';
import type { MediaRecord } from '../types';
import { MEDIA_TYPES, STATUS_STYLES } from '../types';

interface Props {
  record: MediaRecord;
  onEdit: (r: MediaRecord) => void;
  onDelete: (id: number) => void;
  onProgressPlus: (r: MediaRecord) => void;
  onProgressMinus: (r: MediaRecord) => void;
}

// 必须 memo：列表最多 200 行，任一行进度变化都会 setRecords。
// 不 memo 时整表逐行重渲染，动画要等 200 次 reconcile 完才播，表现为卡顿。
// 前提是父组件传入的回调全部用 useCallback 稳定住，否则 memo 无效。
function RecordRow({ record, onEdit, onDelete, onProgressPlus, onProgressMinus }: Props) {
  const clickTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const timerRowId = useRef<number | null>(null);

  // 行内双击/单击判定。除了双击（历史行为）外，
  // 操作区常驻 +1 / -1 按钮，无需记住手势
  const handleClick = useCallback(() => {
    if (timerRowId.current !== record.id) {
      timerRowId.current = record.id;
      clickTimer.current = setTimeout(() => {
        clickTimer.current = null;
        timerRowId.current = null;
        onEdit(record);
      }, 250);
      return;
    }
    if (clickTimer.current) {
      clearTimeout(clickTimer.current);
      clickTimer.current = null;
      timerRowId.current = null;
      onProgressPlus(record);
    }
  }, [record, onEdit, onProgressPlus]);

  // 卸载时清掉待触发的编辑定时器，避免切页后弹窗凭空打开
  useEffect(() => () => {
    if (clickTimer.current) clearTimeout(clickTimer.current);
  }, []);

  // 脏类型（历史导入留下的「书」「韩剧」等）走 fallback，颜色用 CSS 变量
  const typeInfo = MEDIA_TYPES[record.media_type ?? ''] ?? { icon: '', label: record.media_type ?? '未知', color: 'var(--text-secondary)' };
  const statusInfo = STATUS_STYLES[record.status ?? ''] ?? { label: record.status ?? '未知', color: 'var(--text-secondary)', bg: 'var(--fill-light)' };

  const hasProgress = record.current_episode != null && record.total_episode != null;
  const progressPct = hasProgress && record.total_episode! > 0
    ? Math.min(100, Math.round((record.current_episode! / record.total_episode!) * 100))
    : null;
  const barColor = progressPct == null ? '#22d3ee'
    : progressPct >= 100 ? '#22c55e'
    : progressPct >= 66 ? '#06b6d4'
    : progressPct >= 33 ? '#f97316'
    : '#f43f5e';

  return (
    <div className="record-row" onClick={handleClick}>
      <div className="record-name-wrap">
        <span className="record-name">{record.record_name}</span>
      </div>

      <div className="cell-center">
        <span className="record-type-label" style={{ color: typeInfo.color }}>
          {record.media_type || '未知'}
        </span>
      </div>

      <div className="cell-center">
        <span className="record-status" style={{ color: statusInfo.color, background: statusInfo.bg }}>
          {statusInfo.label}
        </span>
      </div>

      <div className="cell-center cell-md">
        {record.season != null ? `第${record.season}季` : <span className="cell-empty">--</span>}
      </div>

      <div className="record-progress">
        {hasProgress ? (
          <div className="progress-cell">
            <span className="progress-text">
              {/* key 变化时重挂载，CSS 动画只播一次；
                  原先用 framer-motion，117 行各挂一个常驻动画实例 */}
              <span key={record.current_episode} className="ep-bump">
                {record.current_episode}
              </span>
              /{record.total_episode}
            </span>
            <div className="progress-bar">
              <div className="progress-fill" style={{ width: `${progressPct}%`, background: `linear-gradient(90deg, ${barColor}, ${barColor}dd)` }} />
            </div>
          </div>
        ) : record.status === '已完成' ? (
          <span className="record-done">✓</span>
        ) : (
          <span className="cell-empty">--</span>
        )}
      </div>

      <div className="record-tags">
        {record.tags || <span className="cell-empty">--</span>}
      </div>

      <div className="cell-center record-country">
        {record.country || <span className="cell-empty">--</span>}
      </div>

      <div className="cell-center record-endtime">
        {record.end_time ? record.end_time.slice(0, 19) : <span className="cell-empty">--</span>}
      </div>

      <div className="record-actions" onClick={(e) => e.stopPropagation()}>
        {/* 进度步进：仅对有进度字段且未完结的条目显示，
            否则这两个按钮在绝大多数行上都是无意义的灰键 */}
        {hasProgress && record.status !== '已完成' && (
          <>
            <button
              className="btn-icon btn-step"
              onClick={() => onProgressMinus(record)}
              disabled={record.current_episode! <= 0}
              title="进度 -1"
            >−</button>
            <button
              className="btn-icon btn-step"
              onClick={() => onProgressPlus(record)}
              title="进度 +1"
            >+</button>
          </>
        )}
        <button className="btn-icon" onClick={() => onEdit(record)} title="编辑">✎</button>
        <button className="btn-icon btn-delete" onClick={() => onDelete(record.id)} title="删除">✕</button>
      </div>
    </div>
  );
}

// 浅比较即可：record 是不可变对象，只有被改动的那一行引用会变；
// 其余行引用不变，配合 memo 直接跳过渲染。
export default memo(RecordRow);
