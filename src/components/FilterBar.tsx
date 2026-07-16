import { TYPE_LIST, STATUS_LIST } from '../types';

interface Props {
  search: string;
  mediaType: string;
  status: string;
  onSearchChange: (v: string) => void;
  onTypeChange: (v: string) => void;
  onStatusChange: (v: string) => void;
}

export default function FilterBar({
  search, mediaType, status,
  onSearchChange, onTypeChange, onStatusChange,
}: Props) {
  return (
    <div className="filter-bar">
      <div className="search-wrapper">
        <span className="search-icon">🔍</span>
        <input
          className="search-input"
          placeholder="搜索名称..."
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
        />
      </div>

      <select className="filter-select" value={mediaType} onChange={(e) => onTypeChange(e.target.value)}>
        {TYPE_LIST.map((t) => (
          <option key={t} value={t}>{t === '全部' ? '全部类型' : t}</option>
        ))}
      </select>

      <select className="filter-select" value={status} onChange={(e) => onStatusChange(e.target.value)}>
        {STATUS_LIST.map((s) => (
          <option key={s} value={s}>{s === '全部' ? '全部状态' : s}</option>
        ))}
      </select>

      <div style={{ flex: 1 }} />
    </div>
  );
}
