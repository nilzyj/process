import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { open } from '@tauri-apps/plugin-dialog';
import type { LibraryConfig, LibraryState, RejectedPath, VideoFile } from '../types';
import LibraryFolderBar from '../components/LibraryFolderBar';
import VideoGroup from '../components/VideoGroup';

/**
 * 超过该数量的分组默认全部折叠。
 * 实测库里可达 1888 个分组、12680 个文件，全展开会同时挂载全部卡片而卡死。
 */
const AUTO_COLLAPSE_GROUPS = 40;

/** 分组总数超过该值时不提供「全部展开」，避免一次性挂载上万卡片 */
const SAFE_EXPAND_ALL = 1500;

export default function Library() {
  const [videos, setVideos] = useState<VideoFile[]>([]);
  const [folders, setFolders] = useState<string[]>([]);
  const [missing, setMissing] = useState<string[]>([]);
  const [potplayer, setPotplayer] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState('');
  const [note, setNote] = useState('');
  const [rejected, setRejected] = useState<RejectedPath[]>([]);
  const [dragOver, setDragOver] = useState(false);

  const applyResult = useCallback((r: LibraryState) => {
    // folders 必须一并更新，否则增删目录后 chip 与配置脱节（删不掉/加不上）
    setFolders(r.folders);
    setVideos(r.videos);
    setMissing(r.missing);
    setScanning(false);

    const notes: string[] = [];
    if (r.created_dirs.length > 0) {
      notes.push(`已创建 ${r.created_dirs.length} 个目录`);
    }
    if (r.ignored > 0) {
      notes.push(`忽略 ${r.ignored} 个非视频文件`);
    }
    if (r.skipped > 0) {
      notes.push(`跳过 ${r.skipped} 个无法读取的文件`);
    }
    if (r.rejected.length > 0) {
      notes.push(`${r.rejected.length} 个路径无法使用`);
    }
    setNote(notes.join(' · '));
    setRejected(r.rejected);
  }, []);

  const rescan = useCallback(async () => {
    setScanning(true);
    setError('');
    try {
      applyResult(await invoke<LibraryState>('rescan_library'));
    } catch (e) {
      setError(String(e));
      setScanning(false);
    }
  }, [applyResult]);

  const addFolders = useCallback(async (paths: string[]) => {
    setScanning(true);
    setError('');
    try {
      applyResult(await invoke<LibraryState>('add_library_folder', { paths }));
    } catch (e) {
      setError(String(e));
      setScanning(false);
    }
  }, [applyResult]);

  const removeFolder = useCallback(async (path: string) => {
    // 立即从界面移除，避免重扫期间 chip 滞留造成的「点了没反应」错觉
    setFolders((prev) => prev.filter((f) => f.toLowerCase() !== path.toLowerCase()));
    try {
      applyResult(await invoke<LibraryState>('remove_library_folder', { path }));
    } catch (e) {
      setError(String(e));
      // 后端失败则回滚，避免界面与真实配置不一致
      invoke<LibraryConfig>('get_library_config')
        .then((cfg) => setFolders(cfg.folders))
        .catch(() => {});
    }
  }, [applyResult]);

  // 启动时加载配置；有目录则自动扫描
  useEffect(() => {
    const init = async () => {
      try {
        const cfg = await invoke<LibraryConfig>('get_library_config');
        setFolders(cfg.folders);
        setPotplayer(cfg.potplayer_path);

        // 未手动指定时，探测自动定位是否可用，好让用户看到真实状态
        if (!cfg.potplayer_path) {
          invoke<string | null>('locate_potplayer')
            .then((found) => { if (found) setPotplayer(found); })
            .catch(() => {});
        }

        if (cfg.folders.length > 0) {
          applyResult(await invoke<LibraryState>('rescan_library'));
        } else {
          setScanning(false);
        }
      } catch (e) {
        setError(String(e));
        setScanning(false);
      }
    };
    init();
  }, [applyResult]);

  // 拖放：Tauri 原生事件，core:default 已含 allow-listen，无需额外权限
  useEffect(() => {
    let dispose: (() => void) | undefined;
    let cancelled = false;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === 'over') setDragOver(true);
        else if (p.type === 'leave') setDragOver(false);
        else if (p.type === 'drop') {
          setDragOver(false);
          if (p.paths.length > 0) addFolders(p.paths);
        }
      })
      .then((un) => {
        if (cancelled) un();
        else dispose = un;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      dispose?.();
    };
  }, [addFolders]);

  const handlePlay = useCallback(async (path: string) => {
    setError('');
    try {
      await invoke<string>('play_video', { path });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const handleReveal = useCallback(async (path: string) => {
    try {
      await invoke<string>('reveal_in_explorer', { path });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const handleCopyPath = useCallback(async (path: string) => {
    try {
      await navigator.clipboard.writeText(path);
      setNote('路径已复制');
      setTimeout(() => setNote(''), 1500);
    } catch {
      setError('复制失败，请手动复制');
    }
  }, []);

  const handleSetPotPlayer = useCallback(async () => {
    const picked = await open({
      multiple: false,
      directory: false,
      title: '选择 PotPlayer 可执行文件',
    });
    const exe = Array.isArray(picked) ? picked[0] : picked;
    if (!exe) return;
    const cfg = await invoke<LibraryConfig>('get_library_config');
    cfg.potplayer_path = exe;
    await invoke<string>('save_library_config', { cfg });
    setPotplayer(exe);
    setNote('播放器路径已保存');
    setTimeout(() => setNote(''), 2000);
  }, []);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return videos;
    return videos.filter(
      (v) => v.name.toLowerCase().includes(q) || v.path.toLowerCase().includes(q),
    );
  }, [videos, search]);

  // 按文件所在目录分组。用户的典型用法是「一个目录 = 一部剧的全部视频」，
  // 平铺后无法区分归属；path 里已含完整目录，前端分组即可，无需后端配合。
  const groups = useMemo(() => {
    const m = new Map<string, VideoFile[]>();
    for (const v of filtered) {
      const sep = Math.max(v.path.lastIndexOf('\\'), v.path.lastIndexOf('/'));
      const dir = sep > 0 ? v.path.slice(0, sep) : v.path;
      const arr = m.get(dir);
      if (arr) arr.push(v);
      else m.set(dir, [v]);
    }

    const list = [...m.entries()].map(([dir, items]) => ({
      dir,
      name: dir.split(/[\\/]/).filter(Boolean).pop() ?? dir,
      items,
    }));

    // 同名末段会撞车（如两个剧都叫 S01），此时向前多取一段以示区分
    const nameCount = new Map<string, number>();
    for (const g of list) nameCount.set(g.name, (nameCount.get(g.name) ?? 0) + 1);

    return list
      .map((g) =>
        (nameCount.get(g.name) ?? 0) > 1
          ? {
              ...g,
              name: g.dir.split(/[\\/]/).filter(Boolean).slice(-2).join(' / '),
            }
          : g,
      )
      .sort((a, b) =>
        a.dir.localeCompare(b.dir, undefined, { numeric: true, sensitivity: 'base' }),
      );
  }, [filtered]);

  /**
   * 只记录用户的显式选择，缺省值由分组规模派生。
   * 这样不需要在 effect 里 setState（会多渲染一次），
   * 也不会覆盖用户已经做过的操作。
   */
  const [overrides, setOverrides] = useState<Map<string, boolean>>(new Map());
  const defaultCollapsed = groups.length > AUTO_COLLAPSE_GROUPS;

  const isCollapsed = useCallback(
    (dir: string) => overrides.get(dir) ?? defaultCollapsed,
    [overrides, defaultCollapsed],
  );

  const collapsedCount = useMemo(
    () => groups.filter((g) => isCollapsed(g.dir)).length,
    [groups, isCollapsed],
  );

  const toggleGroup = useCallback(
    (dir: string) => {
      setOverrides((prev) => {
        const next = new Map(prev);
        next.set(dir, !(prev.get(dir) ?? groups.length > AUTO_COLLAPSE_GROUPS));
        return next;
      });
    },
    [groups.length],
  );

  const collapseAll = useCallback(
    (v: boolean) => setOverrides(new Map(groups.map((g) => [g.dir, v]))),
    [groups],
  );

  return (
    <>
      <LibraryFolderBar
        folders={folders}
        missing={missing}
        potplayerPath={potplayer}
        onBrowse={addFolders}
        onRemove={removeFolder}
        onRescan={rescan}
        onSetPotPlayer={handleSetPotPlayer}
      />

      <div className="list-info">
        <span>
          {search ? `${filtered.length} / ${videos.length} 个文件` : `共 ${videos.length} 个文件`}
        </span>
        {note && <span style={{ color: 'var(--info)', fontSize: 'var(--fs-sm)' }}>{note}</span>}
      </div>

      {rejected.length > 0 && (
        <div className="library-rejected">
          <div className="library-rejected-title">
            ⚠️ 以下路径无法使用，未加入媒体库
            <button className="library-rejected-clear" onClick={() => setRejected([])} title="关闭">✕</button>
          </div>
          <ul>
            {rejected.map((r) => (
              <li key={r.path}>
                <code>{r.path}</code>
                <span>{r.reason}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="library-filter">
        <div className="search-wrapper">
          <span className="search-icon">🔍</span>
          <input
            className="search-input"
            placeholder="搜索文件名或路径..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
      </div>

      {scanning ? (
        <div className="empty-state">
          <div className="loading-spinner" />
          <p>正在扫描目录...</p>
        </div>
      ) : error ? (
        <div className="empty-state">
          <div className="icon">⚠️</div>
          <p style={{ color: 'var(--danger)', wordBreak: 'break-all', maxWidth: '80%' }}>{error}</p>
        </div>
      ) : videos.length === 0 ? (
        <div className="empty-state">
          <div className="icon">📼</div>
          <p>{folders.length === 0 ? '还没有添加媒体库目录' : '目录中没有找到视频文件'}</p>
        </div>
      ) : filtered.length === 0 ? (
        <div className="empty-state">
          <div className="icon">🔍</div>
          <p>没有匹配的文件</p>
        </div>
      ) : (
        <div className="video-groups">
          {groups.length > 1 && (
            <div className="video-group-actions">
              <span>
                {groups.length} 个目录
                {collapsedCount > 0 && collapsedCount < groups.length && (
                  <> · 已折叠 {collapsedCount} 个</>
                )}
              </span>
              {/* 上万文件时全展开会一次挂载全部卡片，直接卡死，不提供该入口 */}
              {filtered.length <= SAFE_EXPAND_ALL && (
                <button onClick={() => collapseAll(false)}>全部展开</button>
              )}
              <button onClick={() => collapseAll(true)}>全部折叠</button>
            </div>
          )}

          {groups.map((g) => (
            <VideoGroup
              key={g.dir}
              name={g.name}
              dir={g.dir}
              items={g.items}
              collapsed={isCollapsed(g.dir)}
              onToggle={toggleGroup}
              onPlay={handlePlay}
              onReveal={handleReveal}
              onCopyPath={handleCopyPath}
            />
          ))}
        </div>
      )}

      {dragOver && (
        <div className="drop-overlay">
          <div className="drop-hint">松开以添加为媒体库目录</div>
        </div>
      )}
    </>
  );
}