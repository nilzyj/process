import type { MediaRecord } from '../types';
import { MEDIA_TYPES, STATUS_STYLES } from '../types';

interface Props {
  record: MediaRecord;
  onEdit: (r: MediaRecord) => void;
  onDelete: (id: number) => void;
}

export default function RecordRow({ record, onEdit, onDelete }: Props) {
  const typeInfo = MEDIA_TYPES[record.media_type ?? ''] ?? { icon: '📄', label: record.media_type ?? '未知', color: '#888' };
  const statusInfo = STATUS_STYLES[record.status ?? ''] ?? { label: record.status ?? '未知', color: '#888', bg: 'rgba(136,136,136,0.15)' };

  const hasProgress = record.current_episode != null && record.total_episode != null;
  const progressPercent = hasProgress && record.total_episode! > 0
    ? Math.min(100, Math.round((record.current_episode! / record.total_episode!) * 100))
    : null;

  return (
    <div className="record-row" onClick={() => onEdit(record)}>
      <div className="record-type-icon">{typeInfo.icon}</div>

      <div>
        <span className="record-name">
          {record.record_name}
          {record.season != null && <span className="record-season"> 季{record.season}</span>}
        </span>
      </div>

      <div>
        <span className="record-type-label" style={{ color: typeInfo.color, background: `${typeInfo.color}1a` }}>
          {typeInfo.label}
        </span>
      </div>

      <div>
        <span className="record-status" style={{ color: statusInfo.color, background: statusInfo.bg }}>
          {statusInfo.label}
        </span>
      </div>

      <div className="record-progress">
        {hasProgress ? (
          <>
            <span>{record.current_episode}/{record.total_episode}</span>
            <div className="progress-bar-wrap">
              <div className="progress-bar-fill" style={{ width: `${progressPercent}%` }} />
            </div>
          </>
        ) : record.status === '已完成' ? (
          <span style={{ color: 'var(--success)' }}>✓</span>
        ) : (
          <span style={{ color: 'var(--text-muted)' }}>--</span>
        )}
      </div>

      <div className="record-country">
        {record.country || <span style={{ color: 'var(--text-muted)' }}>--</span>}
      </div>

      <div className="record-actions" onClick={(e) => e.stopPropagation()}>
        <button className="btn-icon" onClick={() => onEdit(record)} title="编辑">✎</button>
        <button className="btn-icon" onClick={() => onDelete(record.id)} title="删除" style={{ color: 'var(--danger)' }}>✕</button>
      </div>
    </div>
  );
}
