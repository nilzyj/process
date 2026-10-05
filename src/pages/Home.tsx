import { useState, useEffect, useCallback, useRef, lazy, Suspense } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { MediaRecord, RecordFilter, NewRecord, UpdateRecord, PaginatedResult } from '../types';
import FilterBar from '../components/FilterBar';
import RecordRow from '../components/RecordRow';

const RecordForm = lazy(() => import('../components/RecordForm'));

export default function Home() {
  const [records, setRecords] = useState<MediaRecord[]>([]);
  const [total, setTotal] = useState(0);
  // 仅首屏为 true：后续查询静默进行，旧结果留在屏幕上，
  // 否则 loading 分支会顶掉列表造成闪烁
  const [loading, setLoading] = useState(true);
  const hasRecords = records.length > 0;
  const [error, setError] = useState('');
  // 立即回显的输入值，与实际发查询的 search 分离
  const [search, setSearch] = useState('');
  const [query, setQuery] = useState('');
  const [mediaType, setMediaType] = useState('全部');
  const [status, setStatus] = useState('进行中');
  const [editing, setEditing] = useState<MediaRecord | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [page, setPage] = useState(1);
  const [loadingMore, setLoadingMore] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);

  // 每页行数随视口缩放：库里 2900+ 条，一次性渲染 200 行会明显掉帧。
  // 滚动接近底部再拉下一页，DOM 里始终只有一两屏的量。
  const PAGE_SIZE = 200;

  const buildFilter = useCallback((page: number): RecordFilter => {
    const filter: RecordFilter = { page, page_size: PAGE_SIZE };
    if (query) filter.search = query;
    if (mediaType !== '全部') filter.media_type = mediaType;
    if (status !== '全部') filter.status = status;
    return filter;
  }, [query, mediaType, status]);

  // 搜索防抖：每敲一个字都查一遍既闪又浪费。
  // 250ms 停顿后才认为输入结束，中文输入法拼字期间也不会触发。
  useEffect(() => {
    const timer = setTimeout(() => setQuery(search), 250);
    return () => clearTimeout(timer);
  }, [search]);

  const fetchRecords = useCallback(async (silent?: boolean) => {
    // 静默刷新（增删改后）完全不碰 loading；主动查询时也只有在无数据时才亮 spinner
    if (!silent) setLoading(!hasRecords);
    setError('');
    try {
      const result = await invoke<PaginatedResult>('list_records', { filter: buildFilter(1) });
      setRecords(result.records);
      setTotal(result.total);
      setPage(1);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [buildFilter, hasRecords]);

  // 追加下一页：筛选条件变化时 buildFilter 换新引用，
  // loadMore 随之重建，滚动回调拿到的一定是当前条件的函数
  const loadMore = useCallback(async () => {
    if (loadingMore || loading) return;
    const next = page + 1;
    if (records.length >= total) return;
    setLoadingMore(true);
    try {
      const result = await invoke<PaginatedResult>('list_records', { filter: buildFilter(next) });
      setRecords((prev) => {
        // 后端按 id 倒序返回，翻页稳定；仍按 id 去重，防止筛选并发返回造成重复行
        const seen = new Set(prev.map((r) => r.id));
        return [...prev, ...result.records.filter((r) => !seen.has(r.id))];
      });
      setPage(next);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoadingMore(false);
    }
  }, [buildFilter, loading, loadingMore, page, records.length, total]);

  // 滚动到距底部 400px 内即预取，滚动手感上无等待
  useEffect(() => {
    const el = listRef.current;
    if (!el) return;
    const onScroll = () => {
      if (el.scrollHeight - el.scrollTop - el.clientHeight < 400) loadMore();
    };
    el.addEventListener('scroll', onScroll, { passive: true });
    return () => el.removeEventListener('scroll', onScroll);
  }, [loadMore]);

  const fetchInitial = useCallback(async () => {
    // 优先用后端预取的缓存（lib.rs 启动时按 status="进行中" 预取），
    // 让首屏立刻有内容，再用最新筛选条件在后台校正
    try {
      const cached = await invoke<PaginatedResult | null>('get_cached_records');
      if (cached && cached.records.length > 0) {
        setRecords(cached.records);
        setTotal(cached.total);
        setLoading(false);
        invoke<PaginatedResult>('list_records', { filter: buildFilter(1) })
          .then((r) => {
            setRecords(r.records);
            setTotal(r.total);
            setPage(1);
          })
          .catch(() => {});
        return;
      }
    } catch {}
    await fetchRecords();
  }, [buildFilter, fetchRecords]);

  useEffect(() => {
    fetchInitial();
  }, [fetchInitial]);

  const handleSave = async (data: NewRecord | UpdateRecord) => {
    try {
      if ('id' in data) {
        await invoke('update_record', { record: data });
      } else {
        await invoke('add_record', { record: data });
      }
      setShowForm(false);
      setEditing(null);
      fetchRecords(true);
    } catch (e) {
      console.error(e);
    }
  };

  // 进度步进。upsertProgress 是二者的共同实现，
// delta 为 +1 或 -1；写库前先同步本地 state，避免每点一次都等一次 IPC 往返
const upsertProgress = useCallback(async (record: MediaRecord, delta: 1 | -1) => {
    if (record.current_episode == null || record.total_episode == null) return;
    if (record.status === '已完成') return;

    const next = record.current_episode + delta;
    if (next < 0 || next > record.total_episode) return;

    const done = next >= record.total_episode;
    // 满进度时补 end_time：后端 update_record 是整体覆盖，不会自动写。
    // 缺了它这条记录不会进入「完结时间线」统计（按 end_time 过滤）
    const endTime = done
      ? (record.end_time || new Date().toISOString().slice(0, 19).replace('T', ' '))
      : record.end_time;

    setRecords((prev) => prev.map((r) => r.id === record.id
      ? { ...r, current_episode: next, status: done ? '已完成' : r.status, end_time: endTime }
      : r));

    try {
      await invoke('update_record', {
        record: {
          id: record.id,
          record_name: record.record_name,
          season: record.season,
          remark: record.remark,
          media_type: record.media_type,
          status: done ? '已完成' : record.status,
          end_time: endTime,
          country: record.country,
          tags: record.tags,
          current_episode: next,
          total_episode: record.total_episode,
          year: record.year,
        },
      });
    } catch (e) {
      setError(String(e));
      fetchRecords(true);
    }
  }, [fetchRecords]);

  const handleProgressPlus = useCallback((r: MediaRecord) => upsertProgress(r, 1), [upsertProgress]);
  const handleProgressMinus = useCallback((r: MediaRecord) => upsertProgress(r, -1), [upsertProgress]);

  // 下面几个回调必须 useCallback：它们作为 props 传给 memo 化的 RecordRow，
// 每次渲染都新建引用会让 memo 完全失效（退化成整表重渲染）
const handleDelete = useCallback(async (id: number) => {
    try {
      await invoke('delete_record', { id });
      fetchRecords(true);
    } catch (e) {
      setError(String(e));
    }
  }, [fetchRecords]);

  const handleEdit = useCallback((record: MediaRecord) => {
    setEditing(record);
    setShowForm(true);
  }, []);

  const handleAdd = useCallback(() => {
    setEditing(null);
    setShowForm(true);
  }, []);

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
        <span>{records.length >= total ? `共 ${total} 条记录` : `已显示 ${records.length} / ${total} 条`}</span>
        <button className="btn btn-primary btn-sm" onClick={handleAdd}>+ 新增</button>
      </div>

      <div className="record-list" ref={listRef}>
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

        {loading && !hasRecords ? (
          <div className="empty-state">
            <div className="loading-spinner" />
            <p>加载中...</p>
          </div>
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
              onProgressMinus={handleProgressMinus}
            />
          ))
        )}

        {loadingMore && (
          <div className="load-more">
            <div className="loading-spinner" style={{ width: 18, height: 18 }} />
          </div>
        )}
      </div>

      {showForm && (
        <Suspense fallback={null}>
          <RecordForm
            record={editing}
            onSave={handleSave}
            onClose={() => { setShowForm(false); setEditing(null); }}
          />
        </Suspense>
      )}
    </>
  );
}
