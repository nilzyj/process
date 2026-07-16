import { useState, useEffect } from 'react';
import type { MediaRecord, NewRecord, UpdateRecord } from '../types';
import { TYPE_LIST, STATUS_LIST } from '../types';

interface Props {
  record: MediaRecord | null;
  onSave: (data: NewRecord | UpdateRecord) => void;
  onClose: () => void;
}

const empty = (): NewRecord => ({
  record_name: '',
  season: null,
  remark: '',
  media_type: '电影',
  status: '待办',
  end_time: null,
  country: '',
  tags: '',
  current_episode: null,
  total_episode: null,
  year: null,
});

export default function RecordForm({ record, onSave, onClose }: Props) {
  const [form, setForm] = useState<NewRecord>(empty);

  useEffect(() => {
    if (record) {
      setForm({
        record_name: record.record_name,
        season: record.season,
        remark: record.remark ?? '',
        media_type: record.media_type ?? '电影',
        status: record.status ?? '待办',
        end_time: record.end_time,
        country: record.country ?? '',
        tags: record.tags ?? '',
        current_episode: record.current_episode,
        total_episode: record.total_episode,
        year: record.year,
      });
    } else {
      setForm(empty());
    }
  }, [record]);

  const update = (k: keyof NewRecord, v: string | number | null) =>
    setForm((prev) => ({ ...prev, [k]: v }));

  const handleSubmit = () => {
    if (!form.record_name.trim()) return;
    if (record) {
      onSave({ ...form, id: record.id } as UpdateRecord);
    } else {
      onSave(form);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>{record ? '编辑' : '新增'}记录</h3>
          <button className="btn-icon" onClick={onClose}>✕</button>
        </div>

        <div className="modal-body">
          <div className="form-group">
            <label>名称 *</label>
            <input value={form.record_name} onChange={(e) => update('record_name', e.target.value)} autoFocus />
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>类型</label>
              <select value={form.media_type ?? ''} onChange={(e) => update('media_type', e.target.value)}>
                {TYPE_LIST.filter(t => t !== '全部').map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>
            <div className="form-group">
              <label>状态</label>
              <select value={form.status ?? ''} onChange={(e) => update('status', e.target.value)}>
                {STATUS_LIST.filter(s => s !== '全部').map((s) => (
                  <option key={s} value={s}>{s}</option>
                ))}
              </select>
            </div>
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>季</label>
              <input type="number" value={form.season ?? ''} onChange={(e) => update('season', e.target.value ? Number(e.target.value) : null)} />
            </div>
            <div className="form-group">
              <label>年份</label>
              <input type="number" value={form.year ?? ''} onChange={(e) => update('year', e.target.value ? Number(e.target.value) : null)} />
            </div>
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>当前进度</label>
              <input type="number" value={form.current_episode ?? ''} onChange={(e) => update('current_episode', e.target.value ? Number(e.target.value) : null)} />
            </div>
            <div className="form-group">
              <label>总量</label>
              <input type="number" value={form.total_episode ?? ''} onChange={(e) => update('total_episode', e.target.value ? Number(e.target.value) : null)} />
            </div>
          </div>

          <div className="form-row">
            <div className="form-group">
              <label>国家</label>
              <input value={form.country ?? ''} onChange={(e) => update('country', e.target.value)} />
            </div>
            <div className="form-group">
              <label>标签</label>
              <input value={form.tags ?? ''} onChange={(e) => update('tags', e.target.value)} placeholder="逗号分隔" />
            </div>
          </div>

          <div className="form-group">
            <label>备注</label>
            <textarea value={form.remark ?? ''} onChange={(e) => update('remark', e.target.value)} />
          </div>
        </div>

        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onClose}>取消</button>
          <button className="btn btn-primary" onClick={handleSubmit}>
            {record ? '保存' : '添加'}
          </button>
        </div>
      </div>
    </div>
  );
}
