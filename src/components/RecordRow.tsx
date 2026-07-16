import type { MediaRecord } from '../types';
import { MEDIA_TYPES, STATUS_STYLES } from '../types';

interface Props {
  record: MediaRecord;
  onEdit: (r: MediaRecord) => void;
  onDelete: (id: number) => void;
}

export default function RecordRow({ record, onEdit, onDelete }: Props) {
  const typeInfo = MEDIA_TYPES[record.media_type ?? ''] ?? { icon: '', label: record.media_type ?? '未知', color: '#888' };
  const statusInfo = STATUS_STYLES[record.status ?? ''] ?? { label: record.status ?? '未知', color: '#888', bg: 'rgba(136,136,136,0.15)' };

  const hasProgress = record.current_episode != null && record.total_episode != null;
  const progressPct = hasProgress && record.total_episode! > 0
    ? Math.min(100, Math.round((record.current_episode! / record.total_episode!) * 100))
    : null;

  return (
    <div className="record-row" onClick={() => onEdit(record)}>
      <div className="record-name-wrap">
        <span className="record-name">{record.record_name}</span>
      </div>

      <div style={{ textAlign: 'center' }}>
        <span style={{ color: typeInfo.color, fontSize: 13, fontWeight: 600 }}>
          {record.media_type || '未知'}
        </span>
      </div>

      <div style={{ textAlign: 'center' }}>
        <span className="record-status" style={{ color: statusInfo.color, background: statusInfo.bg }}>
          {statusInfo.label}
        </span>
      </div>

      <div style={{ textAlign: 'center', color: 'var(--text-secondary)', fontSize: 13 }}>
        {record.season != null ? `第${record.season}季` : <span style={{ color: 'var(--text-muted)' }}>--</span>}
      </div>

      <div className="record-progress">
        {hasProgress ? (
          <div className="progress-cell">
            <span className="progress-text">{record.current_episode}/{record.total_episode}</span>
            <div className="progress-bar">
              <div className="progress-fill" style={{ width: `${progressPct}%` }} />
            </div>
          </div>
        ) : record.status === '已完成' ? (
          <span style={{ color: 'var(--success)' }}>✓</span>
        ) : (
          <span style={{ color: 'var(--text-muted)' }}>--</span>
        )}
      </div>

      <div style={{ color: 'var(--text-secondary)', fontSize: 12, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', padding: '0 4px' }}>
        {record.tags || <span style={{ color: 'var(--text-muted)' }}>--</span>}
      </div>

      <div style={{ textAlign: 'center', color: 'var(--text-secondary)', fontSize: 12 }}>
        {record.country || <span style={{ color: 'var(--text-muted)' }}>--</span>}
      </div>

      <div style={{ textAlign: 'center', color: 'var(--text-secondary)', fontSize: 12 }}>
        {record.end_time ? record.end_time.slice(0, 19) : <span style={{ color: 'var(--text-muted)' }}>--</span>}
      </div>

      <div className="record-actions" onClick={(e) => e.stopPropagation()}>
        <button className="btn-icon" onClick={() => onEdit(record)} title="编辑">✎</button>
        <button className="btn-icon" onClick={() => onDelete(record.id)} title="删除" style={{ color: 'var(--danger)' }}>✕</button>
      </div>
    </div>
  );
}
