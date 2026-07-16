import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { MediaRecord, RecordFilter, NewRecord, UpdateRecord, PaginatedResult } from '../types';
import FilterBar from '../components/FilterBar';
import RecordRow from '../components/RecordRow';
import RecordForm from '../components/RecordForm';

interface Props {
  connected: boolean;
}

export default function Home({ connected }: Props) {
  const [records, setRecords] = useState<MediaRecord[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [search, setSearch] = useState('');
  const [mediaType, setMediaType] = useState('全部');
  const [status, setStatus] = useState('全部');
  const [editing, setEditing] = useState<MediaRecord | null>(null);
  const [showForm, setShowForm] = useState(false);

  const fetchRecords = useCallback(async () => {
    if (!connected) return;
    setLoading(true);
    setError('');
    try {
      const filter: RecordFilter = { page: 1, page_size: 200 };
      if (search) filter.search = search;
      if (mediaType !== '全部') filter.media_type = mediaType;
      if (status !== '全部') filter.status = status;

      const result = await invoke<PaginatedResult>('list_records', { filter });
      setRecords(result.records);
      setTotal(result.total);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [search, mediaType, status, connected]);

  useEffect(() => {
    fetchRecords();
  }, [fetchRecords]);

  const handleSave = async (data: NewRecord | UpdateRecord) => {
    try {
      if ('id' in data) {
        await invoke('update_record', { record: data });
      } else {
        await invoke('add_record', { record: data });
      }
      setShowForm(false);
      setEditing(null);
      fetchRecords();
    } catch (e) {
      console.error(e);
    }
  };

  const handleProgressPlus = async (record: MediaRecord) => {
    if (record.current_episode == null || record.total_episode == null) return;
    if (record.status === '已完成') return;

    const next = record.current_episode + 1;
    const done = next >= record.total_episode;
    try {
      await invoke('update_record', {
        record: {
          id: record.id,
          record_name: record.record_name,
          season: record.season,
          remark: record.remark,
          media_type: record.media_type,
          status: done ? '已完成' : record.status,
          end_time: record.end_time,
          country: record.country,
          tags: record.tags,
          current_episode: next,
          total_episode: record.total_episode,
          year: record.year,
        },
      });
      fetchRecords();
    } catch (e) {
      console.error(e);
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await invoke('delete_record', { id });
      fetchRecords();
    } catch (e) {
      console.error(e);
    }
  };

  const handleEdit = (record: MediaRecord) => {
    setEditing(record);
    setShowForm(true);
  };

  const handleAdd = () => {
    setEditing(null);
    setShowForm(true);
  };

  return (
    <>
      <FilterBar
        search={search}
        mediaType={mediaType}
        status={status}
        onSearchChange={setSearch}
        onTypeChange={setMediaType}
        onStatusChange={setStatus}
      />

      <div className="list-info">
        <span>{connected ? `共 ${total} 条记录` : ''}</span>
        <button className="btn btn-primary btn-sm" onClick={handleAdd} disabled={!connected}>+ 新增</button>
      </div>

      <div className="record-list">
        <div className="list-header">
          <span>名称</span>
          <span style={{ textAlign: 'center' }}>类型</span>
          <span style={{ textAlign: 'center' }}>状态</span>
          <span style={{ textAlign: 'center' }}>季</span>
          <span style={{ textAlign: 'center' }}>进度</span>
          <span>标签</span>
          <span style={{ textAlign: 'center' }}>国家</span>
          <span style={{ textAlign: 'center' }}>完成时间</span>
          <span></span>
        </div>

        {!connected ? null : loading ? (
          <div className="empty-state"><p>加载中...</p></div>
        ) : error ? (
          <div className="empty-state">
            <div className="icon">⚠️</div>
            <p style={{ color: 'var(--danger)', wordBreak: 'break-all', maxWidth: '80%' }}>{error}</p>
          </div>
        ) : records.length === 0 ? (
          <div className="empty-state">
            <div className="icon">📭</div>
            <p>暂无记录</p>
          </div>
        ) : (
          records.map((r) => (
            <RecordRow
              key={r.id}
              record={r}
              onEdit={handleEdit}
              onDelete={handleDelete}
              onProgressPlus={handleProgressPlus}
            />
          ))
        )}
      </div>

      {showForm && (
        <RecordForm
          record={editing}
          onSave={handleSave}
          onClose={() => { setShowForm(false); setEditing(null); }}
        />
      )}
    </>
  );
}
